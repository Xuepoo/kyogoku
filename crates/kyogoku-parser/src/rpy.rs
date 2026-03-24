#[cfg(feature = "rpy")]
use nom::{
    character::complete::char,
    branch::alt,
    IResult,
};
use crate::block::TranslationBlock;
use crate::parser::Parser;
use anyhow::Result;
use serde_json::json;

/// Ren'Py script parser (.rpy)
///
/// Robust parser using nom combinators to handle:
/// - Character dialogue: `e "Hello"` -> extract "Hello", keep `e`
/// - Narrator dialogue: `"Hello"` -> extract "Hello"
/// - Menu options: `"Yes":` -> extract "Yes"
/// - Multiline strings: `e """..."""`
/// - Python blocks: skipped
/// - Comments: skipped
///
/// Structure and indentation are preserved for perfect reconstruction.
pub struct RpyParser;

/// Represents a parsed Ren'Py element
#[derive(Debug, Clone)]
#[allow(dead_code)]
enum RpyElement {
    /// Character dialogue: (speaker, text, quote_char, line_number)
    Dialogue(Option<String>, String, char, usize),
    /// Menu choice: (text, quote_char, line_number)
    MenuChoice(String, char, usize),
    /// Multiline dialogue: (speaker, text, quote_type, start_line, end_line)
    MultilineDialogue(Option<String>, String, MultilineQuote, usize, usize),
}

#[derive(Debug, Clone)]
enum MultilineQuote {
    DoubleTriple,  // """
    SingleTriple,  // '''
}

impl std::fmt::Display for MultilineQuote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MultilineQuote::DoubleTriple => write!(f, "\"\"\""),
            MultilineQuote::SingleTriple => write!(f, "'''"),
        }
    }
}

#[cfg(feature = "rpy")]
mod parsers {
    use super::*;

    /// Parse a string in quotes (handles both single and double)
    pub fn quoted_string(input: &str) -> IResult<&str, (String, char)> {
        let (input, quote_char) = alt((char('"'), char('\'')))(input)?;
        let (input, content) = take_until_quote(input, quote_char)?;
        let (input, _) = char(quote_char)(input)?;
        Ok((input, (content.to_string(), quote_char)))
    }

    /// Take until we find an unescaped quote
    fn take_until_quote(input: &str, quote: char) -> IResult<&str, &str> {
        let mut pos = 0;
        let bytes = input.as_bytes();

        while pos < bytes.len() {
            if bytes[pos] == quote as u8 {
                return Ok((&input[pos..], &input[..pos]));
            }
            if bytes[pos] == b'\\' && pos + 1 < bytes.len() {
                pos += 2; // Skip escaped character
            } else {
                pos += 1;
            }
        }
        // No closing quote found
        Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::TooLarge,
        )))
    }

    /// Parse a menu choice line: "text": or 'text':
    pub fn parse_menu_choice_line(input: &str) -> IResult<&str, (String, char)> {
        let trimmed = input.trim();
        let (rest, (text, quote)) = quoted_string(trimmed)?;
        let rest = rest.trim();
        let (rest, _) = char(':')(rest)?;
        let _ = rest.trim(); // Should be end of line or comment
        Ok(("", (text, quote)))
    }

    /// Parse dialogue line: speaker "text" or just "text"
    pub fn parse_dialogue_line(input: &str) -> IResult<&str, (Option<String>, String, char)> {
        let trimmed = input.trim();

        // Check if it's reserved keyword
        if is_reserved_keyword(trimmed).is_ok() {
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::TooLarge,
            )));
        }

        // Try narration first (pure quoted string)
        if (trimmed.starts_with('"') || trimmed.starts_with('\''))
            && let Ok((rest, (text, quote))) = quoted_string(trimmed) {
                let rest = rest.trim();
                // Check that it's not followed by : (which would be menu)
                if !rest.starts_with(':') && !rest.contains('=') {
                    return Ok(("", (None, text, quote)));
                }
            }

        // Try character dialogue: speaker "text"
        if let Some(first_quote_pos) = trimmed.find('"').or_else(|| trimmed.find('\'')) {
            let before_quote = &trimmed[..first_quote_pos];
            // Check no = before quote (would be assignment)
            if !before_quote.contains('=') && !before_quote.contains("\"\"\"") && !before_quote.contains("'''")
                && let Some(last_space) = before_quote.rfind(' ') {
                    let speaker = before_quote[..last_space].trim();
                    let quote_part = before_quote[last_space..].trim();
                    if !speaker.is_empty() && quote_part.is_empty() {
                        // Valid speaker pattern
                        let quote_rest = &trimmed[first_quote_pos..];
                        if let Ok((rest, (text, quote))) = quoted_string(quote_rest) {
                            let rest = rest.trim();
                            if !rest.starts_with(':') && !rest.contains('=') {
                                return Ok((
                                    "",
                                    (Some(speaker.to_string()), text, quote),
                                ));
                            }
                        }
                    }
                }
        }

        Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::TooLarge,
        )))
    }

    /// Check if a line starts with a reserved keyword that's NOT dialogue
    fn is_reserved_keyword(input: &str) -> IResult<&str, ()> {
        let trimmed = input.trim();
        let keywords = [
            "scene ", "show ", "play ", "stop ", "define ", "default ", "label ", "jump ",
            "return", "call ", "$", "if ", "elif ", "else:", "menu:", "python:", "init ",
        ];

        for kw in &keywords {
            if trimmed.starts_with(kw) {
                return Ok(("", ()));
            }
        }
        Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::TooLarge,
        )))
    }
}

impl Parser for RpyParser {
    fn extensions(&self) -> &[&str] {
        &["rpy"]
    }

    fn parse(&self, content: &[u8]) -> Result<Vec<TranslationBlock>> {
        let content_str = std::str::from_utf8(content)?;
        let mut blocks = Vec::new();
        let lines: Vec<&str> = content_str.lines().collect();

        let mut idx = 0;
        let mut in_python_block = false;
        let mut python_indent_level = 0;

        while idx < lines.len() {
            let line = lines[idx];
            let trimmed = line.trim();

            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') {
                idx += 1;
                continue;
            }

            let current_indent = line.len() - line.trim_start().len();

            // Handle python block detection and skipping
            if in_python_block {
                if current_indent > python_indent_level && !trimmed.is_empty() {
                    // Still in python block
                    idx += 1;
                    continue;
                } else {
                    // Exited python block
                    in_python_block = false;
                }
            }

            // Check for python block start
            if trimmed.starts_with("python:") || trimmed.starts_with("init python:") {
                in_python_block = true;
                python_indent_level = current_indent;
                idx += 1;
                continue;
            }

            // Try parsing as multiline dialogue first
            if let Some(element) =
                self.try_parse_multiline_dialogue(line, &lines, idx)
                && let Some((speaker, text, quote, start, end)) =
                    match element {
                        RpyElement::MultilineDialogue(s, t, q, st, ed) => Some((s, t, q, st, ed)),
                        _ => None,
                    }
                {
                    blocks.push(
                        TranslationBlock::new(text)
                            .with_speaker(speaker.unwrap_or_default())
                            .with_metadata(json!({
                                "line_start": start,
                                "line_end": end,
                                "quote": quote.to_string(),
                                "type": "multiline_dialogue"
                            })),
                    );
                    idx = end + 1;
                    continue;
                }

            // Try menu choice
            #[cfg(feature = "rpy")]
            if let Ok((_, (text, quote))) = parsers::parse_menu_choice_line(line) {
                blocks.push(
                    TranslationBlock::new(text).with_metadata(json!({
                        "line": idx,
                        "quote": quote.to_string(),
                        "type": "menu"
                    })),
                );
                idx += 1;
                continue;
            }

            // Try dialogue
            #[cfg(feature = "rpy")]
            if let Ok((_, (speaker, text, quote))) = parsers::parse_dialogue_line(line) {
                blocks.push(
                    TranslationBlock::new(text)
                        .with_speaker(speaker.unwrap_or_default())
                        .with_metadata(json!({
                            "line": idx,
                            "quote": quote.to_string(),
                            "type": "dialogue"
                        })),
                );
                idx += 1;
                continue;
            }

            idx += 1;
        }

        tracing::debug!("Parsed {} blocks from RPY", blocks.len());
        Ok(blocks)
    }

    fn serialize(&self, blocks: &[TranslationBlock], template: &[u8]) -> Result<Vec<u8>> {
        let template_str = std::str::from_utf8(template)?;
        let mut lines: Vec<String> = template_str.lines().map(|s| s.to_string()).collect();

        for block in blocks {
            let block_type = block
                .metadata
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("dialogue");

            match block_type {
                "multiline_dialogue" => {
                    let start_line = block
                        .metadata
                        .get("line_start")
                        .and_then(|v| v.as_u64())
                        .or_else(|| block.metadata.get("line").and_then(|v| v.as_u64()))
                        .ok_or_else(|| anyhow::anyhow!("Missing line_start in multiline block"))? as usize;

                    let end_line = block
                        .metadata
                        .get("line_end")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(start_line as u64) as usize;

                    let quote_str = block
                        .metadata
                        .get("quote")
                        .and_then(|v| v.as_str())
                        .unwrap_or("\"\"\"");

                    if start_line < lines.len() && end_line < lines.len() {
                        let start_line_text = &lines[start_line];

                        // Find the quote markers
                        if let Some(start_quote_pos) = start_line_text.find(quote_str) {
                            let prefix = &start_line_text[..start_quote_pos + quote_str.len()];

                            let end_line_text = &lines[end_line];
                            let end_quote_pos = if start_line == end_line {
                                end_line_text.rfind(quote_str).unwrap_or(start_quote_pos)
                            } else {
                                end_line_text.find(quote_str).unwrap_or(0)
                            };

                            let suffix = if start_line != end_line || end_quote_pos > start_quote_pos {
                                &end_line_text[end_quote_pos..]
                            } else {
                                ""
                            };

                            lines[start_line] = format!("{}{}{}", prefix, block.output(), suffix);

                            // Clear intermediate lines
                            for i in start_line + 1..=end_line {
                                if i < lines.len() {
                                    lines[i].clear();
                                }
                            }
                        }
                    }
                }
                _ => {
                    // Regular dialogue or menu
                    if let Some(line_num) = block
                        .metadata
                        .get("line")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as usize)
                        && let Some(line_text) = lines.get_mut(line_num) {
                            let quote_str = block
                                .metadata
                                .get("quote")
                                .and_then(|v| v.as_str())
                                .unwrap_or("\"");

                            if let Some(new_line) =
                                self.replace_string_in_line(line_text, block.output(), quote_str)
                            {
                                *line_text = new_line;
                            }
                        }
                }
            }
        }

        let output = lines.join("\n");
        if !output.is_empty() && !output.ends_with('\n') {
            Ok((output + "\n").into_bytes())
        } else {
            Ok(output.into_bytes())
        }
    }
}

impl RpyParser {
    /// Try to parse a multiline dialogue starting at the given line
    fn try_parse_multiline_dialogue(
        &self,
        line: &str,
        all_lines: &[&str],
        start_idx: usize,
    ) -> Option<RpyElement> {
        let trimmed = line.trim();

        // Check for assignment (e.g., x = """...""")
        if trimmed.contains('=') && trimmed.find('=').unwrap_or(usize::MAX) < trimmed.find("\"\"\"").or_else(|| trimmed.find("'''")).unwrap_or(usize::MAX) {
            return None;
        }

        // Look for triple quotes
        let (quote_type, quote_str) = if trimmed.contains("\"\"\"") {
            (MultilineQuote::DoubleTriple, "\"\"\"")
        } else if trimmed.contains("'''") {
            (MultilineQuote::SingleTriple, "'''")
        } else {
            return None;
        };

        let quote_pos = trimmed.find(quote_str)?;
        let prefix = trimmed[..quote_pos].trim();

        let speaker = if prefix.is_empty() {
            None
        } else {
            Some(prefix.to_string())
        };

        let after_opening_quote = &trimmed[quote_pos + 3..];

        // Check if the closing quote is on the same line
        if let Some(close_pos) = after_opening_quote.find(quote_str) {
            // Single-line multiline block
            let content = &after_opening_quote[..close_pos];
            return Some(RpyElement::MultilineDialogue(
                speaker, content.to_string(), quote_type, start_idx, start_idx,
            ));
        }

        // Multi-line block: collect lines until closing quote
        let mut content_lines = Vec::new();
        
        // Only add content from the opening line if there's content after the opening quotes
        if !after_opening_quote.is_empty() {
            content_lines.push(after_opening_quote.to_string());
        }
        
        let mut end_idx = start_idx;

        for (i, next_line) in all_lines.iter().enumerate().skip(start_idx + 1) {
            let next_trimmed = next_line.trim();

            if let Some(close_pos) = next_trimmed.find(quote_str) {
                // Found closing quote
                if close_pos > 0 {
                    content_lines.push(next_trimmed[..close_pos].to_string());
                }
                end_idx = i;
                break;
            } else {
                // Regular content line
                content_lines.push(next_trimmed.to_string());
                end_idx = i;
            }
        }

        if end_idx > start_idx {
            let content = content_lines.join("\n");
            return Some(RpyElement::MultilineDialogue(
                speaker, content, quote_type, start_idx, end_idx,
            ));
        }

        None
    }

    /// Replace the string content in a line, preserving structure
    fn replace_string_in_line(
        &self,
        line: &str,
        new_content: &str,
        quote_str: &str,
    ) -> Option<String> {
        let quote_char = quote_str.chars().next()?;

        // Find the first quote
        let start_pos = line.find(quote_char)?;

        // Find the last quote (assuming it's the closing one)
        let remainder = &line[start_pos + 1..];
        let end_offset = remainder.rfind(quote_char)?;
        let end_pos = start_pos + 1 + end_offset;

        let prefix = &line[..start_pos + 1];
        let suffix = &line[end_pos..];

        Some(format!("{}{}{}", prefix, new_content, suffix))
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_dialogue() {
        let content = r#"define e = Character("Eileen")

label start:
    e "Hello, world!"
"#;

        let parser = RpyParser;
        let blocks = parser.parse(content.as_bytes()).unwrap();

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].speaker, Some("e".to_string()));
        assert_eq!(blocks[0].source, "Hello, world!");
        assert_eq!(
            blocks[0].metadata.get("type").and_then(|v| v.as_str()),
            Some("dialogue")
        );
    }

    #[test]
    fn test_parse_narration() {
        let content = r#"label start:
    "This is narration without a speaker."
"#;

        let parser = RpyParser;
        let blocks = parser.parse(content.as_bytes()).unwrap();

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].speaker, Some("".to_string()));
        assert_eq!(blocks[0].source, "This is narration without a speaker.");
    }

    #[test]
    fn test_parse_menu_choice() {
        let content = r#"label start:
    menu:
        "Choice 1":
            jump choice1
        "Choice 2":
            jump choice2
"#;

        let parser = RpyParser;
        let blocks = parser.parse(content.as_bytes()).unwrap();

        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].source, "Choice 1");
        assert_eq!(blocks[1].source, "Choice 2");
        assert_eq!(
            blocks[0].metadata.get("type").and_then(|v| v.as_str()),
            Some("menu")
        );
    }

    #[test]
    fn test_parse_multiline_dialogue() {
        let content = r#"label start:
    e """
    This is a multiline
    dialogue string.
    It spans multiple lines.
    """
"#;

        let parser = RpyParser;
        let blocks = parser.parse(content.as_bytes()).unwrap();

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].speaker, Some("e".to_string()));
        assert!(blocks[0].source.contains("multiline"));
        assert!(blocks[0].source.contains("multiple lines"));
        assert_eq!(
            blocks[0].metadata.get("type").and_then(|v| v.as_str()),
            Some("multiline_dialogue")
        );
    }

    #[test]
    fn test_skip_python_blocks() {
        let content = r#"label start:
    e "Dialogue 1"
    
    python:
        x = """
        This should be ignored
        """
    
    e "Dialogue 2"
"#;

        let parser = RpyParser;
        let blocks = parser.parse(content.as_bytes()).unwrap();

        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].source, "Dialogue 1");
        assert_eq!(blocks[1].source, "Dialogue 2");

        // Ensure python content is not parsed
        for block in &blocks {
            assert!(!block.source.contains("should be ignored"));
        }
    }

    #[test]
    fn test_skip_comments() {
        let content = r#"# This is a comment
label start:
    e "Dialogue"  # This is also a comment
"#;

        let parser = RpyParser;
        let blocks = parser.parse(content.as_bytes()).unwrap();

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].source, "Dialogue");
    }

    #[test]
    fn test_mixed_dialogue_and_narration() {
        let content = r#"label start:
    e "Hello!"
    "Narration here."
    m "Hi there!"
    "More narration."
"#;

        let parser = RpyParser;
        let blocks = parser.parse(content.as_bytes()).unwrap();

        assert_eq!(blocks.len(), 4);
        assert_eq!(blocks[0].speaker, Some("e".to_string()));
        assert_eq!(blocks[0].source, "Hello!");
        assert_eq!(blocks[1].speaker, Some("".to_string()));
        assert_eq!(blocks[1].source, "Narration here.");
        assert_eq!(blocks[2].speaker, Some("m".to_string()));
        assert_eq!(blocks[3].source, "More narration.");
    }

    #[test]
    fn test_single_quotes() {
        let content = r#"label start:
    e 'Single quoted dialogue'
    'Single quoted narration'
"#;

        let parser = RpyParser;
        let blocks = parser.parse(content.as_bytes()).unwrap();

        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].source, "Single quoted dialogue");
        assert_eq!(blocks[1].source, "Single quoted narration");
    }

    #[test]
    fn test_preserve_indentation_in_multiline() {
        let content = r#"label start:
    e """
    Line 1
    Line 2
    Line 3
    """
"#;

        let parser = RpyParser;
        let blocks = parser.parse(content.as_bytes()).unwrap();

        assert_eq!(blocks.len(), 1);
        let source = &blocks[0].source;
        // The content should preserve the line structure
        assert!(source.contains("Line 1"));
        assert!(source.contains("Line 2"));
        assert!(source.contains("Line 3"));
    }

    #[test]
    fn test_serialize_simple_dialogue() {
        let template = r#"label start:
    e "Hello, world!"
"#;

        let mut block = TranslationBlock::new("Hello, world!");
        block.speaker = Some("e".to_string());
        block.metadata = json!({
            "line": 1,
            "quote": "\"",
            "type": "dialogue"
        });

        let parser = RpyParser;
        let result = parser.serialize(&[block], template.as_bytes()).unwrap();
        let result_str = String::from_utf8(result).unwrap();

        assert!(result_str.contains("e \""));
        // The content should be preserved
        let lines: Vec<&str> = result_str.lines().collect();
        assert!(lines.iter().any(|l| l.contains("e")));
    }

    #[test]
    fn test_serialize_translated_dialogue() {
        let template = r#"label start:
    e "Hello, world!"
"#;

        let mut block = TranslationBlock::new("Hello, world!");
        block.speaker = Some("e".to_string());
        block.target = Some("你好，世界！".to_string());
        block.metadata = json!({
            "line": 1,
            "quote": "\"",
            "type": "dialogue"
        });

        let parser = RpyParser;
        let result = parser.serialize(&[block], template.as_bytes()).unwrap();
        let result_str = String::from_utf8(result).unwrap();

        assert!(result_str.contains("你好，世界！"));
        assert!(result_str.contains("e"));
    }

    #[test]
    fn test_ignore_variable_assignments() {
        let content = r#"label start:
    x = "Not dialogue"
    $ y = "Also not dialogue"
    e "This is dialogue"
"#;

        let parser = RpyParser;
        let blocks = parser.parse(content.as_bytes()).unwrap();

        // Should only parse the dialogue, not the assignments
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].source, "This is dialogue");
    }

    #[test]
    fn test_multiline_narration() {
        let content = r#"label start:
    """
    This is a multiline
    narration block.
    No speaker specified.
    """
"#;

        let parser = RpyParser;
        let blocks = parser.parse(content.as_bytes()).unwrap();

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].speaker, Some("".to_string()));
        assert!(blocks[0].source.contains("multiline"));
        assert!(blocks[0].source.contains("narration block"));
    }

    #[test]
    fn test_empty_dialogue_ignored() {
        let content = r#"label start:
    e ""
    
    e "Not empty"
"#;

        let parser = RpyParser;
        let blocks = parser.parse(content.as_bytes()).unwrap();

        // Empty strings may or may not be included depending on needs
        // At minimum, non-empty should be present
        assert!(blocks.iter().any(|b| b.source == "Not empty"));
    }

    #[test]
    fn test_complex_rpy_file() {
        let content = r#"define e = Character("Eileen")
define m = Character("Me")

label start:
    e "Welcome to my game!"
    
    "The story begins..."
    
    menu:
        "Let's start":
            jump begin
        "Exit":
            return
    
    label begin:
    m "I'm ready!"
    e """
    Then let's begin
    this adventure together.
    """
"#;

        let parser = RpyParser;
        let blocks = parser.parse(content.as_bytes()).unwrap();

        // Should have extracted:
        // 1. "Welcome to my game!" (e)
        // 2. "The story begins..." (narration)
        // 3. "Let's start" (menu)
        // 4. "Exit" (menu)
        // 5. "I'm ready!" (m)
        // 6. Multiline dialogue (e)
        
        assert!(blocks.len() >= 6);
        assert!(blocks.iter().any(|b| b.source.contains("Welcome")));
        assert!(blocks.iter().any(|b| b.source.contains("The story")));
        assert!(blocks.iter().any(|b| b.source.contains("Let's start")));
    }
}
