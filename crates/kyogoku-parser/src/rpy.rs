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

    fn parse(&self, content: &[u8]) -> Result<Vec<TranslationBlock>> {
        let content_str = std::str::from_utf8(content)?;
        let mut blocks = Vec::new();
        let mut lines = content_str.lines().enumerate().peekable();

        // Track python blocks to skip them
        let mut in_python_block = false;
        let mut python_indent = 0;

        while let Some((line_idx, line)) = lines.next() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Check indentation for python block handling
            let current_indent = line.len() - line.trim_start().len();

            if in_python_block {
                if current_indent > python_indent {
                    // Inside python block, skip
                    continue;
                } else {
                    // Dedented, block ended
                    in_python_block = false;
                }
            }

            // Check if this starts a python block
            // Heuristic: starts with python/init python and ends with :
            if (trimmed.starts_with("python") || trimmed.starts_with("init python"))
                && trimmed.ends_with(':')
            {
                in_python_block = true;
                python_indent = current_indent;
                continue;
            }

            // Check for multiline string start: [speaker] """ or '''
            if let Some((speaker, quote_str, is_closed)) = parse_multiline_start(line) {
                if is_closed {
                    // One-line multiline block: e """Content"""
                    let trimmed = line.trim();
                    let q3_idx = match trimmed.find(&quote_str) {
                        Some(idx) => idx,
                        None => continue,
                    };
                    let remainder = &trimmed[q3_idx + 3..];
                    let end_idx = match remainder.find(&quote_str) {
                        Some(idx) => idx,
                        None => continue,
                    };
                    let content = &remainder[..end_idx];

                    blocks.push(
                        TranslationBlock::new(content.to_string())
                            .with_speaker(speaker.unwrap_or_default())
                            .with_metadata(json!({
                                "line": line_idx,
                                "quote": quote_str,
                                "type": "multiline_dialogue"
                            })),
                    );
                    continue;
                }

                // Open multiline block
                let mut content_lines = Vec::new();
                let mut end_line_idx = line_idx;

                // Handle content on the start line (e """Content)
                let trimmed = line.trim();
                let q3_idx = match trimmed.find(&quote_str) {
                    Some(idx) => idx,
                    None => continue,
                };
                let remainder = &trimmed[q3_idx + 3..];
                if !remainder.is_empty() {
                    content_lines.push(remainder.to_string());
                }

                // Consume lines until closing quote
                while let Some((next_idx, next_line)) = lines.peek() {
                    let next_trimmed = next_line.trim();
                    if next_trimmed.contains(&quote_str) {
                        // Found end!
                        // It could be inline: "Content""" or """ (end)

                        // Check position
                        let q3_end_idx = match next_trimmed.find(&quote_str) {
                            Some(idx) => idx,
                            None => break, // Should not happen given contains check
                        };
                        let content_part = &next_trimmed[..q3_end_idx];

                        if !content_part.is_empty() {
                            content_lines.push(content_part.to_string());
                        }

                        // Consume this line and finish block
                        end_line_idx = *next_idx;
                        lines.next();
                        break;
                    } else {
                        // Just a content line
                        // Note: Ren'Py preserves newlines in multiline strings
                        // But leading indentation might need stripping?
                        // Usually indentation relative to the block is stripped.
                        // For now, let's keep it simple: trim only common indentation if needed.
                        // But `trim()` removes ALL indentation. This might break formatting if it was intentional.
                        // However, standard Ren'Py convention is to indent content.
                        // If we use `trim()`, we lose paragraphs structure if it relies on spaces?
                        // No, newlines are preserved.
                        // Let's use `trim()` for now as it's safer than guessing indentation.
                        content_lines.push(next_trimmed.to_string());
                        lines.next();
                    }
                }

                // Construct block
                let full_text = content_lines.join("\n");
                let block = TranslationBlock::new(full_text)
                    .with_speaker(speaker.unwrap_or_default())
                    .with_metadata(json!({
                        "line_start": line_idx,
                        "line_end": end_line_idx,
                        "quote": quote_str,
                        "type": "multiline_dialogue"
                    }));
                blocks.push(block);
                continue;
            }

            // Heuristic for dialogue: [indent] [character] "text"
            // ... (existing logic)
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
            }
        }

        tracing::debug!("Parsed {} blocks from RPY", blocks.len());
        Ok(blocks)
    }

    fn serialize(&self, blocks: &[TranslationBlock], template: &[u8]) -> Result<Vec<u8>> {
        let template_str = std::str::from_utf8(template)?;
        let mut lines: Vec<String> = template_str.lines().map(|s| s.to_string()).collect();

        // Iterate backwards to avoid index shifting if we were inserting lines (we aren't)
        // But mainly to match lines by index
        for block in blocks {
            let type_ = block
                .metadata
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("dialogue");

            if type_ == "multiline_dialogue" || type_ == "multiline_dialogue_oneline" {
                let start_opt = block.metadata.get("line_start").and_then(|v| v.as_u64());
                let end_opt = block.metadata.get("line_end").and_then(|v| v.as_u64());

                // Fallback for backward compatibility or if line_start missing
                let start = start_opt
                    .or_else(|| block.metadata.get("line").and_then(|v| v.as_u64()))
                    .ok_or_else(|| {
                        anyhow::anyhow!("Missing line metadata for block {}", block.id)
                    })? as usize;
                let end = end_opt.unwrap_or(start as u64) as usize;

                let quote = block
                    .metadata
                    .get("quote")
                    .and_then(|v| v.as_str())
                    .unwrap_or("\"\"\"");

                if start >= lines.len() || end >= lines.len() {
                    continue;
                }

                // We construct the new block content in the start line
                // and clear subsequent lines to avoid duplication/ghosting

                let start_line_content = lines[start].clone();
                let end_line_content = lines[end].clone();

                // Find start quote in start line
                if let Some(q_start_idx) = start_line_content.find(quote) {
                    let prefix = &start_line_content[..q_start_idx + 3]; // e """ or just """

                    // Find end quote in end line
                    let q_end_idx = if start == end {
                        // Search from end, but must be after start quote
                        // However, if content is empty e """""", rfind might find the first one?
                        // We want the last one.
                        start_line_content.rfind(quote).unwrap_or(q_start_idx)
                    } else {
                        end_line_content.find(quote).unwrap_or(0)
                    };

                    // If we failed to find end quote, fallback?
                    if start == end && q_start_idx == q_end_idx {
                        // This implies unclosed or empty?
                        // If empty e """""", prefix covers it?
                        // If it is e """""", q_start=2. q_end=2.
                        // We want q_end to be the SECOND quote if exists.
                        // But finding the range is tricky if identical.
                        // Assuming valid input, there are 2 occurrences if start==end.
                        // rfind finds last. find finds first.
                    }

                    let suffix = if start == end {
                        if q_end_idx > q_start_idx {
                            &start_line_content[q_end_idx..]
                        } else {
                            // Only one quote found? Broken line?
                            ""
                        }
                    } else {
                        &end_line_content[q_end_idx..]
                    };

                    let new_text = block.output();

                    // Replace start line
                    lines[start] = format!("{}{}{}", prefix, new_text, suffix);

                    // Clear intermediate lines
                    for i in start + 1..=end {
                        if i < lines.len() {
                            lines[i].clear();
                        }
                    }
                }

                continue;
            }

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
        let output = lines.join("\n") + "\n";
        Ok(output.into_bytes())
    }
}

fn parse_multiline_start(line: &str) -> Option<(Option<String>, String, bool)> {
    let trimmed = line.trim();
    let q3_idx = trimmed.find("\"\"\"").or_else(|| trimmed.find("'''"))?;

    // Check for assignment before quotes (e.g. x = """)
    // This prevents python blocks from being parsed as dialogue
    if trimmed[..q3_idx].contains('=') {
        return None;
    }

    let quote_str = &trimmed[q3_idx..q3_idx + 3];
    let prefix = trimmed[..q3_idx].trim();
    let speaker = if prefix.is_empty() {
        None
    } else {
        Some(prefix.to_string())
    };

    let remainder = &trimmed[q3_idx + 3..];
    let is_closed = remainder.contains(quote_str);

    Some((speaker, quote_str.to_string(), is_closed))
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
        || trimmed.starts_with("python:")
        || trimmed.starts_with("init ")
    {
        return None;
    }

    // Safety check: if line contains "=", it's likely code assignment
    // unless it's inside the string. But we handle simple dialogue here.
    // e "text" = valid
    // $ x = "text" = invalid (code)
    // We already check for $ start.
    // What about: x = "text" (python assignment without $)
    // Ren'Py allows python in blocks or one-liners.
    // Generally dialogue doesn't have = outside quotes.
    // Let's check for = before the first quote.

    let first_quote_idx = trimmed.find('"').or_else(|| trimmed.find('\''))?;
    if trimmed[..first_quote_idx].contains('=') {
        return None;
    }

    // Check for trailing colon (menu choice or block start)
    if trimmed.ends_with(':') {
        return None;
    }

    // TRIPLE QUOTE CHECK:
    // If it contains """ or ''', reject it here (handled by parse_multiline_start)
    if trimmed.contains("\"\"\"") || trimmed.contains("'''") {
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
