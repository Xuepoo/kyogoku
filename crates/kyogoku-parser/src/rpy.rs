use crate::block::TranslationBlock;
use crate::parser::Parser;
use anyhow::Result;
use serde_json::json;

/// Ren'Py script parser (.rpy)
///
/// Handles dialogue lines, menu choices, and narration.
/// Preserves indentation and Python-like structure.
pub struct RpyParser;

impl Parser for RpyParser {
    fn extensions(&self) -> &[&str] {
        &["rpy"]
    }

    fn parse(&self, content: &str) -> Result<Vec<TranslationBlock>> {
        let mut blocks = Vec::new();
        let lines = content.lines().enumerate();

        for (line_idx, line) in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Heuristic for dialogue: [indent] [character] "text"
            // Heuristic for menu: [indent] "text":

            if let Some((speaker, text, quote_char)) = parse_dialogue_line(line) {
                let block = TranslationBlock::new(text)
                    .with_speaker(speaker.unwrap_or_default())
                    .with_metadata(json!({
                        "line": line_idx,
                        "quote": quote_char,
                        "type": "dialogue"
                    }));
                blocks.push(block);
            } else if let Some((text, quote_char)) = parse_menu_choice(line) {
                let block = TranslationBlock::new(text).with_metadata(json!({
                    "line": line_idx,
                    "quote": quote_char,
                    "type": "menu"
                }));
                blocks.push(block);
            } else if let Some((_text, _quote_str)) = parse_triple_quote_start(line) {
                // Handle multi-line string start
                // This is a simplified handling - assuming it might be narration
                // For MVP we might skip complex multiline unless requested
                // But let's try to capture it if it's a simple narration
                // TODO: Implement full multiline support
            }
        }

        tracing::debug!("Parsed {} blocks from RPY", blocks.len());
        Ok(blocks)
    }

    fn serialize(&self, blocks: &[TranslationBlock], template: &str) -> Result<String> {
        let mut lines: Vec<String> = template.lines().map(|s| s.to_string()).collect();

        // Iterate backwards to avoid index shifting if we were inserting lines (we aren't)
        // But mainly to match lines by index
        for block in blocks {
            let line_idx = block
                .metadata
                .get("line")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);
            let quote = block
                .metadata
                .get("quote")
                .and_then(|v| v.as_str())
                .unwrap_or("\"");
            let output_text = block.output();

            if let Some(idx) = line_idx
                && let Some(original_line) = lines.get_mut(idx)
            {
                // We need to replace the content inside the quotes
                // This is tricky if there are escaped quotes.
                // Ideally we use the same parsing logic to find range.
                if let Some(new_line) = replace_string_content(original_line, output_text, quote) {
                    *original_line = new_line;
                }
            }
        }

        // Rejoin with original line endings? .lines() eats them.
        // Assuming \n for now.
        Ok(lines.join("\n") + "\n")
    }
}

/// Parse a line to see if it's a dialogue line.
/// Returns (Option<Speaker>, Text, QuoteChar)
fn parse_dialogue_line(line: &str) -> Option<(Option<String>, String, String)> {
    // Regex would be nice, but let's do manual parsing for dependency minimalism
    // Pattern: ^(\s*)(?:(\w+)\s+)?(["'])(.*)(["'])$

    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }

    // Ignore known keywords that might look like identifiers but aren't speakers
    if trimmed.starts_with("scene ")
        || trimmed.starts_with("show ")
        || trimmed.starts_with("play ")
        || trimmed.starts_with("stop ")
        || trimmed.starts_with("define ")
        || trimmed.starts_with("default ")
        || trimmed.starts_with("label ")
        || trimmed.starts_with("jump ")
        || trimmed.starts_with("return")
        || trimmed.starts_with("call ")
        || trimmed.starts_with("$")
        || trimmed.starts_with("if ")
        || trimmed.starts_with("elif ")
        || trimmed.starts_with("else:")
        || trimmed.starts_with("menu:")
    {
        return None;
    }

    // Check for trailing colon (menu choice or block start)
    if trimmed.ends_with(':') {
        return None;
    }

    let mut chars = trimmed.chars().peekable();
    let first = *chars.peek()?;

    let quote_char = if first == '"' || first == '\'' {
        Some(first)
    } else {
        None
    };

    if let Some(q) = quote_char {
        // Narration: "Text"
        if let Some(content) = extract_string_content(trimmed, q) {
            return Some((None, content, q.to_string()));
        }
    } else {
        // Dialogue: e "Text"
        // Find first space
        if let Some(space_idx) = trimmed.find(' ') {
            let speaker = &trimmed[..space_idx];
            let rest = trimmed[space_idx..].trim();

            if let Some(first_char) = rest.chars().next()
                && (first_char == '"' || first_char == '\'')
                && let Some(content) = extract_string_content(rest, first_char)
            {
                return Some((Some(speaker.to_string()), content, first_char.to_string()));
            }
        }
    }

    None
}

/// Parse a menu choice line.
/// Pattern: ^(\s*)(["'])(.*)(["'])\s*:$
fn parse_menu_choice(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    if !trimmed.ends_with(':') {
        return None;
    }

    // Remove trailing colon
    let content_part = trimmed[..trimmed.len() - 1].trim();

    let first_char = content_part.chars().next()?;
    if (first_char == '"' || first_char == '\'')
        && let Some(content) = extract_string_content(content_part, first_char)
    {
        return Some((content, first_char.to_string()));
    }

    None
}

fn parse_triple_quote_start(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    if trimmed.starts_with("\"\"\"") {
        return Some((String::new(), "\"\"\"".to_string()));
    }
    if trimmed.starts_with("'''") {
        return Some((String::new(), "'''".to_string()));
    }
    None
}

/// Extract content inside quotes, handling basic escaping
fn extract_string_content(s: &str, quote: char) -> Option<String> {
    if !s.starts_with(quote) || !s.ends_with(quote) {
        return None;
    }
    // Simple slice for now, assuming no crazy escaping or trailing comments
    // In Ren'Py, "Text" is valid.
    if s.len() < 2 {
        return None;
    }

    // TODO: Handle escaped quotes inside
    Some(s[1..s.len() - 1].to_string())
}

/// Replace content inside the first occurrence of quote pair
fn replace_string_content(line: &str, new_content: &str, quote_str: &str) -> Option<String> {
    let quote_char = quote_str.chars().next()?;

    // Find start quote
    let start_idx = line.find(quote_char)?;

    // Find end quote - searching from end to be safe?
    // Or just search after start.
    // If we assume the structure hasn't changed, we can look for the last quote
    // that matches, but we need to be careful about comments/trailing spaces.

    // Simple approach: replace the *last* occurrence of quote if it's at the end
    // But dialogue might be: e "Text" # comment
    // So we need to match the parsing logic.

    let remainder = &line[start_idx + 1..];
    let end_idx = remainder.rfind(quote_char)?;
    let actual_end_idx = start_idx + 1 + end_idx;

    let prefix = &line[..start_idx + 1];
    let suffix = &line[actual_end_idx..];

    Some(format!("{}{}{}", prefix, new_content, suffix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dialogue() {
        let line = "    e \"Hello world\"";
        let (speaker, text, q) = parse_dialogue_line(line).unwrap();
        assert_eq!(speaker, Some("e".to_string()));
        assert_eq!(text, "Hello world");
        assert_eq!(q, "\"");
    }

    #[test]
    fn test_parse_narration() {
        let line = "\"Just narration\"";
        let (speaker, text, _) = parse_dialogue_line(line).unwrap();
        assert_eq!(speaker, None);
        assert_eq!(text, "Just narration");
    }

    #[test]
    fn test_parse_menu() {
        let line = "    \"Choice A\":";
        let (text, _) = parse_menu_choice(line).unwrap();
        assert_eq!(text, "Choice A");
    }

    #[test]
    fn test_replace_content() {
        let line = "    e \"Hello\"";
        let new_line = replace_string_content(line, "World", "\"").unwrap();
        assert_eq!(new_line, "    e \"World\"");
    }
}
