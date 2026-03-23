use anyhow::Result;
use serde_json::json;

use crate::block::TranslationBlock;
use crate::parser::Parser;

/// Plain text file parser - one line = one block.
pub struct TxtParser;

impl Parser for TxtParser {
    fn extensions(&self) -> &[&str] {
        &["txt"]
    }

    fn parse(&self, content: &str) -> Result<Vec<TranslationBlock>> {
        let blocks: Vec<TranslationBlock> = content
            .lines()
            .enumerate()
            .filter(|(_, line)| !line.trim().is_empty())
            .map(|(idx, line)| {
                TranslationBlock::new(line)
                    .with_metadata(json!({ "line": idx + 1 }))
            })
            .collect();

        tracing::debug!("Parsed {} blocks from TXT", blocks.len());
        Ok(blocks)
    }

    fn serialize(&self, blocks: &[TranslationBlock], _template: &str) -> Result<String> {
        let output: String = blocks
            .iter()
            .map(|block| block.output())
            .collect::<Vec<_>>()
            .join("\n");

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_txt_parse() {
        let content = "Hello\nWorld\n\nTest";
        let parser = TxtParser;
        let blocks = parser.parse(content).unwrap();

        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].source, "Hello");
        assert_eq!(blocks[1].source, "World");
        assert_eq!(blocks[2].source, "Test");
    }

    #[test]
    fn test_txt_serialize() {
        let blocks = vec![
            TranslationBlock::new("Hello").with_target("你好"),
            TranslationBlock::new("World").with_target("世界"),
        ];

        let parser = TxtParser;
        let output = parser.serialize(&blocks, "").unwrap();

        assert_eq!(output, "你好\n世界");
    }

    #[test]
    fn test_txt_roundtrip() {
        let content = "Line 1\nLine 2\nLine 3";
        let parser = TxtParser;
        
        let blocks = parser.parse(content).unwrap();
        let output = parser.serialize(&blocks, content).unwrap();
        
        assert_eq!(output, content);
    }
}
