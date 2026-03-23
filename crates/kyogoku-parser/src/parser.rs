use anyhow::Result;
use std::path::Path;

use crate::block::TranslationBlock;

/// Parser trait as defined in the spec.
/// Any new format must implement this trait.
pub trait Parser: Send + Sync {
    /// Returns the file extensions this parser handles
    fn extensions(&self) -> &[&str];

    /// Parse content string into TranslationBlock sequence
    fn parse(&self, content: &str) -> Result<Vec<TranslationBlock>>;

    /// Serialize blocks back to the original format
    /// `template` is the original file content for preserving structure
    fn serialize(&self, blocks: &[TranslationBlock], template: &str) -> Result<String>;

    /// Check if this parser can handle the given file extension
    fn can_handle(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| self.extensions().iter().any(|e| e.eq_ignore_ascii_case(ext)))
            .unwrap_or(false)
    }
}

/// Registry of available file parsers.
pub struct ParserRegistry {
    parsers: Vec<Box<dyn Parser>>,
}

impl ParserRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            parsers: Vec::new(),
        };

        // Register default parsers
        registry.register(Box::new(crate::txt::TxtParser));
        registry.register(Box::new(crate::srt::SrtParser));
        registry.register(Box::new(crate::json::JsonParser));
        registry.register(Box::new(crate::ass::AssParser));
        registry.register(Box::new(crate::vtt::VttParser));
        registry.register(Box::new(crate::rpy::RpyParser));

        registry
    }

    pub fn register(&mut self, parser: Box<dyn Parser>) {
        self.parsers.push(parser);
    }

    pub fn get_parser(&self, path: &Path) -> Option<&dyn Parser> {
        self.parsers.iter().find(|p| p.can_handle(path)).map(|p| p.as_ref())
    }

    pub fn supported_extensions(&self) -> Vec<&str> {
        self.parsers
            .iter()
            .flat_map(|p| p.extensions().iter().copied())
            .collect()
    }

    /// Parse a file and return TranslationBlocks
    pub fn parse_file(&self, path: &Path) -> Result<Vec<TranslationBlock>> {
        let parser = self
            .get_parser(path)
            .ok_or_else(|| anyhow::anyhow!("No parser found for file: {}", path.display()))?;

        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read file {}: {}", path.display(), e))?;

        parser.parse(&content)
    }

    /// Write translated blocks back to file
    pub fn write_file(&self, path: &Path, blocks: &[TranslationBlock], template: &str) -> Result<()> {
        let parser = self
            .get_parser(path)
            .ok_or_else(|| anyhow::anyhow!("No parser found for file: {}", path.display()))?;

        let output = parser.serialize(blocks, template)?;
        std::fs::write(path, output)
            .map_err(|e| anyhow::anyhow!("Failed to write file {}: {}", path.display(), e))?;

        Ok(())
    }
}

impl Default for ParserRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry() {
        let registry = ParserRegistry::new();
        let exts = registry.supported_extensions();
        assert!(exts.contains(&"txt"));
        assert!(exts.contains(&"srt"));
        assert!(exts.contains(&"json"));
        assert!(exts.contains(&"ass"));
        assert!(exts.contains(&"ssa"));
        assert!(exts.contains(&"vtt"));
    }

    #[test]
    fn test_parser_selection() {
        let registry = ParserRegistry::new();
        
        assert!(registry.get_parser(Path::new("test.txt")).is_some());
        assert!(registry.get_parser(Path::new("test.srt")).is_some());
        assert!(registry.get_parser(Path::new("test.json")).is_some());
        assert!(registry.get_parser(Path::new("test.ass")).is_some());
        assert!(registry.get_parser(Path::new("test.ssa")).is_some());
        assert!(registry.get_parser(Path::new("test.vtt")).is_some());
        assert!(registry.get_parser(Path::new("test.xyz")).is_none());
    }
}
