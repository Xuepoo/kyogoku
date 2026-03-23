use anyhow::{Context, Result};
use serde_json::json;

use crate::block::TranslationBlock;
use crate::parser::Parser;

/// SRT subtitle file parser.
pub struct SrtParser;

impl Parser for SrtParser {
    fn extensions(&self) -> &[&str] {
        &["srt"]
    }

    fn parse(&self, content: &[u8]) -> Result<Vec<TranslationBlock>> {
        let content_str = std::str::from_utf8(content)?;
        let mut blocks = Vec::new();
        let mut lines = content_str.lines().peekable();

        while lines.peek().is_some() {
            // Skip empty lines
            while lines.peek().is_some_and(|l| l.trim().is_empty()) {
                lines.next();
            }

            // Parse index
            let index_line = match lines.next() {
                Some(line) if !line.trim().is_empty() => line.trim(),
                _ => break,
            };

            let index: u32 = index_line
                .parse()
                .with_context(|| format!("Invalid SRT index: {}", index_line))?;

            // Parse timestamp
            let timestamp = lines.next().context("Missing timestamp")?.to_string();

            // Parse text (can be multiple lines)
            let mut text_lines = Vec::new();
            while let Some(line) = lines.peek() {
                if line.trim().is_empty() {
                    lines.next();
                    break;
                }
                if let Some(l) = lines.next() {
                    text_lines.push(l.to_string());
                } else {
                    break;
                }
            }

            let text = text_lines.join("\n");

            blocks.push(TranslationBlock::new(&text).with_metadata(json!({
                "index": index,
                "timestamp": timestamp,
            })));
        }

        tracing::debug!("Parsed {} blocks from SRT", blocks.len());
        Ok(blocks)
    }

    fn serialize(&self, blocks: &[TranslationBlock], _template: &[u8]) -> Result<Vec<u8>> {
        let mut output = String::new();

        for (idx, block) in blocks.iter().enumerate() {
            // Index
            output.push_str(&format!("{}\n", idx + 1));

            // Timestamp from metadata
            if let Some(ts) = block.metadata.get("timestamp").and_then(|v| v.as_str()) {
                output.push_str(ts);
                output.push('\n');
            }

            // Text
            output.push_str(block.output());
            output.push_str("\n\n");
        }

        Ok(output.trim_end().as_bytes().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_SRT: &[u8] = b"1
00:00:01,000 --> 00:00:04,000
Hello, world!

2
00:00:05,000 --> 00:00:08,000
This is a test.

3
00:00:09,000 --> 00:00:12,000
Multi-line
subtitle text
";

    #[test]
    fn test_srt_parse() {
        let parser = SrtParser;
        let blocks = parser.parse(SAMPLE_SRT).unwrap();

        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].source, "Hello, world!");
        assert_eq!(blocks[1].source, "This is a test.");
        assert!(blocks[2].source.contains("Multi-line"));
    }

    #[test]
    fn test_srt_serialize() {
        let blocks = vec![
            TranslationBlock::new("Hello")
                .with_metadata(json!({"index": 1, "timestamp": "00:00:01,000 --> 00:00:04,000"}))
                .with_target("你好"),
        ];

        let parser = SrtParser;
        let output = parser.serialize(&blocks, b"").unwrap();
        let output_str = std::str::from_utf8(&output).unwrap();

        assert!(output_str.contains("你好"));
        assert!(output_str.contains("00:00:01,000"));
    }
}
