use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Glossary entry for consistent term translation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlossaryEntry {
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub context: Option<String>,
}

/// Glossary for managing translation terms.
#[derive(Debug, Clone, Default)]
pub struct Glossary {
    entries: HashMap<String, GlossaryEntry>,
}

impl Glossary {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read glossary from {}", path.display()))?;

        let entries: Vec<GlossaryEntry> = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse glossary from {}", path.display()))?;

        let mut glossary = Self::new();
        for entry in entries {
            glossary.add(entry);
        }

        tracing::info!("Loaded {} glossary entries from {}", glossary.len(), path.display());
        Ok(glossary)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let entries: Vec<&GlossaryEntry> = self.entries.values().collect();
        let content = serde_json::to_string_pretty(&entries)
            .context("Failed to serialize glossary")?;

        std::fs::write(path, content)
            .with_context(|| format!("Failed to write glossary to {}", path.display()))?;

        Ok(())
    }

    pub fn add(&mut self, entry: GlossaryEntry) {
        self.entries.insert(entry.source.clone(), entry);
    }

    pub fn get(&self, source: &str) -> Option<&GlossaryEntry> {
        self.entries.get(source)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Find all glossary entries that appear in the given text
    pub fn find_matches(&self, text: &str) -> Vec<&GlossaryEntry> {
        self.entries
            .values()
            .filter(|entry| text.contains(&entry.source))
            .collect()
    }

    /// Format glossary entries for inclusion in translation prompt
    pub fn format_for_prompt(&self, text: &str) -> Option<String> {
        let matches = self.find_matches(text);
        if matches.is_empty() {
            return None;
        }

        let formatted: Vec<String> = matches
            .iter()
            .map(|e| format!("- {} → {}", e.source, e.target))
            .collect();

        Some(format!("术语表 (Glossary):\n{}", formatted.join("\n")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glossary() {
        let mut glossary = Glossary::new();
        
        glossary.add(GlossaryEntry {
            source: "田中".to_string(),
            target: "田中".to_string(),
            context: Some("人名".to_string()),
        });

        assert_eq!(glossary.len(), 1);
        assert!(glossary.get("田中").is_some());
    }

    #[test]
    fn test_find_matches() {
        let mut glossary = Glossary::new();
        
        glossary.add(GlossaryEntry {
            source: "勇者".to_string(),
            target: "勇者".to_string(),
            context: None,
        });

        let matches = glossary.find_matches("勇者は魔王を倒した");
        assert_eq!(matches.len(), 1);
    }
}
