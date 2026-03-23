use anyhow::{Context, Result};
use sled::Db;
use std::path::Path;

use crate::config::Config;

/// Translation cache using sled KV store.
/// Caches translations by content hash to avoid re-translating.
pub struct TranslationCache {
    db: Db,
}

impl TranslationCache {
    pub fn open(path: &Path) -> Result<Self> {
        let db = sled::open(path)
            .with_context(|| format!("Failed to open cache at {}", path.display()))?;

        tracing::debug!("Opened translation cache at {}", path.display());
        Ok(Self { db })
    }

    pub fn open_default() -> Result<Self> {
        let path = Config::cache_path()
            .context("Could not determine cache directory")?;

        std::fs::create_dir_all(&path)
            .with_context(|| format!("Failed to create cache directory {}", path.display()))?;

        Self::open(&path)
    }

    /// Get cached translation by content hash
    pub fn get(&self, hash: &str) -> Option<String> {
        self.db
            .get(hash.as_bytes())
            .ok()
            .flatten()
            .and_then(|v| String::from_utf8(v.to_vec()).ok())
    }

    /// Store translation by content hash
    pub fn set(&self, hash: &str, translation: &str) -> Result<()> {
        self.db
            .insert(hash.as_bytes(), translation.as_bytes())
            .context("Failed to insert into cache")?;

        Ok(())
    }

    /// Check if a hash exists in cache
    pub fn contains(&self, hash: &str) -> bool {
        self.db.contains_key(hash.as_bytes()).unwrap_or(false)
    }

    /// Clear all cached translations
    pub fn clear(&self) -> Result<()> {
        self.db.clear().context("Failed to clear cache")?;
        Ok(())
    }

    /// Get number of cached entries
    pub fn len(&self) -> usize {
        self.db.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Flush to disk
    pub fn flush(&self) -> Result<()> {
        self.db.flush().context("Failed to flush cache")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cache_operations() {
        let tmp = TempDir::new().unwrap();
        let cache = TranslationCache::open(tmp.path()).unwrap();

        assert!(cache.is_empty());

        // Test set and get
        cache.set("hash1", "translation1").unwrap();
        assert_eq!(cache.get("hash1"), Some("translation1".to_string()));

        // Test contains
        assert!(cache.contains("hash1"));
        assert!(!cache.contains("hash2"));

        // Test len
        assert_eq!(cache.len(), 1);

        // Test clear
        cache.clear().unwrap();
        assert!(cache.is_empty());
    }
}
