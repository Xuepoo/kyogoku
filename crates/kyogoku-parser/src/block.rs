use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The unified intermediate representation for all translatable content.
/// All format parsers convert their source into this standardized structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationBlock {
    /// Content hash (Blake3) - serves as unique identifier
    pub id: String,
    /// Speaker/character name if applicable
    pub speaker: Option<String>,
    /// Original source text to be translated
    pub source: String,
    /// Translated text (None if not yet translated)
    pub target: Option<String>,
    /// Format-specific metadata (timestamps, tags, line numbers, etc.)
    pub metadata: Value,
}

impl TranslationBlock {
    /// Create a new TranslationBlock with auto-generated hash ID
    pub fn new(source: impl Into<String>) -> Self {
        let source = source.into();
        let id = Self::hash(&source);
        Self {
            id,
            speaker: None,
            source,
            target: None,
            metadata: Value::Null,
        }
    }

    /// Generate Blake3 hash of content
    pub fn hash(content: &str) -> String {
        blake3::hash(content.as_bytes()).to_hex().to_string()
    }

    pub fn with_speaker(mut self, speaker: impl Into<String>) -> Self {
        self.speaker = Some(speaker.into());
        self
    }

    pub fn with_target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = metadata;
        self
    }

    /// Check if this block needs translation
    pub fn needs_translation(&self) -> bool {
        self.target.is_none() && !self.source.is_empty()
    }

    /// Get the output text (target if available, otherwise source)
    pub fn output(&self) -> &str {
        self.target.as_deref().unwrap_or(&self.source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_creation() {
        let block = TranslationBlock::new("Hello, world!");
        assert!(!block.id.is_empty());
        assert_eq!(block.source, "Hello, world!");
        assert!(block.target.is_none());
        assert!(block.needs_translation());
    }

    #[test]
    fn test_hash_consistency() {
        let hash1 = TranslationBlock::hash("test");
        let hash2 = TranslationBlock::hash("test");
        assert_eq!(hash1, hash2);

        let hash3 = TranslationBlock::hash("different");
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_with_target() {
        let block = TranslationBlock::new("Hello").with_target("你好");
        assert!(!block.needs_translation());
        assert_eq!(block.output(), "你好");
    }

    #[test]
    fn test_with_speaker() {
        let block = TranslationBlock::new("Hello").with_speaker("Alice");
        assert_eq!(block.speaker, Some("Alice".to_string()));
    }
}
