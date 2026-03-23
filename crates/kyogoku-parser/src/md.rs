use anyhow::Result;
use serde_json::json;

use crate::block::TranslationBlock;
use crate::parser::Parser;

/// Markdown parser that preserves frontmatter and code blocks.
/// 
/// Frontmatter (YAML/TOML between ---/+++ delimiters) is preserved as-is.
/// Code blocks (``` or ~~~) are preserved as-is.
/// Only paragraph text is extracted for translation.
pub struct MdParser;

#[derive(Debug, Clone, Copy, PartialEq)]
enum State {
    Normal,
    Frontmatter,
    CodeBlock,
}

impl Parser for MdParser {
    fn extensions(&self) -> &[&str] {
        &["md", "markdown"]
    }

    fn parse(&self, content: &[u8]) -> Result<Vec<TranslationBlock>> {
        let content_str = std::str::from_utf8(content)?;
        let lines: Vec<&str> = content_str.lines().collect();
        let mut blocks = Vec::new();
        let mut state = State::Normal;
        let mut code_fence: Option<&str> = None;
        let mut paragraph_start: Option<usize> = None;
        let mut paragraph_lines: Vec<&str> = Vec::new();

        // Check for frontmatter at start
        if !lines.is_empty() && (lines[0] == "---" || lines[0] == "+++") {
            state = State::Frontmatter;
        }

        for (idx, line) in lines.iter().enumerate() {
            match state {
                State::Frontmatter => {
                    // End of frontmatter
                    if idx > 0 && (*line == "---" || *line == "+++") {
                        state = State::Normal;
                    }
                }
                State::CodeBlock => {
                    // End of code block
                    if let Some(fence) = code_fence
                        && line.starts_with(fence)
                        && line.trim() == fence
                    {
                        state = State::Normal;
                        code_fence = None;
                    }
                }
                State::Normal => {
                    // Check for code block start
                    let trimmed = line.trim_start();
                    if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                        // Flush any pending paragraph
                        if !paragraph_lines.is_empty() {
                            let text = paragraph_lines.join("\n");
                            if !text.trim().is_empty() {
                                blocks.push(
                                    TranslationBlock::new(&text).with_metadata(json!({
                                        "type": "paragraph",
                                        "start_line": paragraph_start.unwrap_or(idx) + 1
                                    })),
                                );
                            }
                            paragraph_lines.clear();
                            paragraph_start = None;
                        }
                        
                        state = State::CodeBlock;
                        code_fence = Some(if trimmed.starts_with("```") { "```" } else { "~~~" });
                    } else if is_structural_line(line) {
                        // Flush paragraph before structural elements
                        if !paragraph_lines.is_empty() {
                            let text = paragraph_lines.join("\n");
                            if !text.trim().is_empty() {
                                blocks.push(
                                    TranslationBlock::new(&text).with_metadata(json!({
                                        "type": "paragraph",
                                        "start_line": paragraph_start.unwrap_or(idx) + 1
                                    })),
                                );
                            }
                            paragraph_lines.clear();
                            paragraph_start = None;
                        }
                        
                        // Headers are translatable
                        if line.starts_with('#') {
                            let text = line.trim_start_matches('#').trim();
                            if !text.is_empty() {
                                blocks.push(
                                    TranslationBlock::new(text).with_metadata(json!({
                                        "type": "header",
                                        "line": idx + 1,
                                        "prefix": extract_header_prefix(line)
                                    })),
                                );
                            }
                        }
                    } else if line.trim().is_empty() {
                        // End of paragraph
                        if !paragraph_lines.is_empty() {
                            let text = paragraph_lines.join("\n");
                            if !text.trim().is_empty() {
                                blocks.push(
                                    TranslationBlock::new(&text).with_metadata(json!({
                                        "type": "paragraph",
                                        "start_line": paragraph_start.unwrap_or(idx) + 1
                                    })),
                                );
                            }
                            paragraph_lines.clear();
                            paragraph_start = None;
                        }
                    } else {
                        // Part of paragraph
                        if paragraph_start.is_none() {
                            paragraph_start = Some(idx);
                        }
                        paragraph_lines.push(line);
                    }
                }
            }
        }

        // Flush final paragraph
        if !paragraph_lines.is_empty() {
            let text = paragraph_lines.join("\n");
            if !text.trim().is_empty() {
                blocks.push(
                    TranslationBlock::new(&text).with_metadata(json!({
                        "type": "paragraph",
                        "start_line": paragraph_start.unwrap_or(lines.len()) + 1
                    })),
                );
            }
        }

        tracing::debug!("Parsed {} blocks from Markdown", blocks.len());
        Ok(blocks)
    }

    fn serialize(&self, blocks: &[TranslationBlock], template: &[u8]) -> Result<Vec<u8>> {
        let template_str = std::str::from_utf8(template)?;
        let lines: Vec<&str> = template_str.lines().collect();
        let mut output_lines: Vec<String> = Vec::new();
        let mut state = State::Normal;
        let mut code_fence: Option<&str> = None;
        let mut block_idx = 0;
        let mut in_paragraph = false;
        let mut skip_until_blank = false;

        // Check for frontmatter at start
        if !lines.is_empty() && (lines[0] == "---" || lines[0] == "+++") {
            state = State::Frontmatter;
        }

        for (idx, line) in lines.iter().enumerate() {
            match state {
                State::Frontmatter => {
                    output_lines.push(line.to_string());
                    if idx > 0 && (*line == "---" || *line == "+++") {
                        state = State::Normal;
                    }
                }
                State::CodeBlock => {
                    output_lines.push(line.to_string());
                    if let Some(fence) = code_fence
                        && line.starts_with(fence)
                        && line.trim() == fence
                    {
                        state = State::Normal;
                        code_fence = None;
                    }
                }
                State::Normal => {
                    let trimmed = line.trim_start();
                    
                    if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                        skip_until_blank = false;
                        in_paragraph = false;
                        output_lines.push(line.to_string());
                        state = State::CodeBlock;
                        code_fence = Some(if trimmed.starts_with("```") { "```" } else { "~~~" });
                    } else if line.starts_with('#') {
                        skip_until_blank = false;
                        in_paragraph = false;
                        // Find matching header block
                        if block_idx < blocks.len() {
                            let block = &blocks[block_idx];
                            if block.metadata.get("type").and_then(|v| v.as_str()) == Some("header") {
                                let prefix = block.metadata.get("prefix").and_then(|v| v.as_str()).unwrap_or("# ");
                                output_lines.push(format!("{}{}", prefix, block.output()));
                                block_idx += 1;
                                continue;
                            }
                        }
                        output_lines.push(line.to_string());
                    } else if is_structural_line(line) || line.trim().is_empty() {
                        skip_until_blank = false;
                        in_paragraph = false;
                        output_lines.push(line.to_string());
                    } else {
                        // Paragraph text
                        if skip_until_blank {
                            continue;
                        }
                        
                        if !in_paragraph {
                            // Start of new paragraph - output translated block
                            if block_idx < blocks.len() {
                                let block = &blocks[block_idx];
                                if block.metadata.get("type").and_then(|v| v.as_str()) == Some("paragraph") {
                                    output_lines.push(block.output().to_string());
                                    block_idx += 1;
                                    in_paragraph = true;
                                    skip_until_blank = true;
                                    continue;
                                }
                            }
                            output_lines.push(line.to_string());
                            in_paragraph = true;
                        }
                    }
                }
            }
        }

        let mut output = output_lines.join("\n");
        // Preserve trailing newline if original had one
        if template_str.ends_with('\n') && !output.ends_with('\n') {
            output.push('\n');
        }

        Ok(output.into_bytes())
    }
}

fn is_structural_line(line: &str) -> bool {
    let trimmed = line.trim();
    // Headers
    trimmed.starts_with('#')
        // Horizontal rules
        || trimmed.chars().all(|c| c == '-' || c == '*' || c == '_' || c.is_whitespace())
            && trimmed.chars().filter(|c| *c == '-' || *c == '*' || *c == '_').count() >= 3
        // List items (we preserve these as-is for now)
        || trimmed.starts_with("- ")
        || trimmed.starts_with("* ")
        || trimmed.starts_with("+ ")
        || (trimmed.len() > 2 && trimmed.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) && trimmed.contains(". "))
        // Blockquotes
        || trimmed.starts_with('>')
        // Tables
        || trimmed.starts_with('|')
}

fn extract_header_prefix(line: &str) -> String {
    let mut prefix = String::new();
    for c in line.chars() {
        if c == '#' || c == ' ' {
            prefix.push(c);
        } else {
            break;
        }
    }
    // Ensure there's a space after #'s
    if !prefix.ends_with(' ') {
        prefix.push(' ');
    }
    prefix
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_md_parse_simple() {
        let content = b"# Hello\n\nThis is a paragraph.";
        let parser = MdParser;
        let blocks = parser.parse(content).unwrap();

        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].source, "Hello");
        assert_eq!(blocks[1].source, "This is a paragraph.");
    }

    #[test]
    fn test_md_parse_frontmatter() {
        let content = b"---\ntitle: Test\ndate: 2024-01-01\n---\n\n# Header\n\nContent here.";
        let parser = MdParser;
        let blocks = parser.parse(content).unwrap();

        // Frontmatter should be skipped
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].source, "Header");
        assert_eq!(blocks[1].source, "Content here.");
    }

    #[test]
    fn test_md_parse_code_blocks() {
        let content = b"# Title\n\nSome text.\n\n```rust\nfn main() {}\n```\n\nMore text.";
        let parser = MdParser;
        let blocks = parser.parse(content).unwrap();

        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].source, "Title");
        assert_eq!(blocks[1].source, "Some text.");
        assert_eq!(blocks[2].source, "More text.");
    }

    #[test]
    fn test_md_serialize() {
        let template = b"# Hello\n\nThis is text.\n";
        let blocks = vec![
            TranslationBlock::new("Hello")
                .with_target("你好")
                .with_metadata(json!({"type": "header", "line": 1, "prefix": "# "})),
            TranslationBlock::new("This is text.")
                .with_target("这是文本。")
                .with_metadata(json!({"type": "paragraph", "start_line": 3})),
        ];

        let parser = MdParser;
        let output = parser.serialize(&blocks, template).unwrap();
        let output_str = std::str::from_utf8(&output).unwrap();

        assert!(output_str.contains("# 你好"));
        assert!(output_str.contains("这是文本。"));
    }

    #[test]
    fn test_md_preserve_frontmatter() {
        let template = b"---\ntitle: Test\n---\n\n# Header\n";
        let blocks = vec![TranslationBlock::new("Header")
            .with_target("标题")
            .with_metadata(json!({"type": "header", "line": 5, "prefix": "# "}))];

        let parser = MdParser;
        let output = parser.serialize(&blocks, template).unwrap();
        let output_str = std::str::from_utf8(&output).unwrap();

        assert!(output_str.starts_with("---\ntitle: Test\n---"));
        assert!(output_str.contains("# 标题"));
    }

    #[test]
    fn test_md_preserve_code_blocks() {
        let template = b"```python\nprint('hello')\n```\n\nText here.\n";
        let blocks = vec![TranslationBlock::new("Text here.")
            .with_target("文本在这里。")
            .with_metadata(json!({"type": "paragraph", "start_line": 5}))];

        let parser = MdParser;
        let output = parser.serialize(&blocks, template).unwrap();
        let output_str = std::str::from_utf8(&output).unwrap();

        assert!(output_str.contains("```python\nprint('hello')\n```"));
        assert!(output_str.contains("文本在这里。"));
    }

    #[test]
    fn test_md_multiline_paragraph() {
        let content = b"First line\nof paragraph.\n\nSecond paragraph.";
        let parser = MdParser;
        let blocks = parser.parse(content).unwrap();

        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].source, "First line\nof paragraph.");
        assert_eq!(blocks[1].source, "Second paragraph.");
    }
}
