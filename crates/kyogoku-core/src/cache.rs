use anyhow::{Context, Result};
use sled::Db;
use std::path::Path;

use crate::config::Config;

/// Translation cache using sled KV store.
/// Caches translations by content hash to avoid re-translating.
pub struct TranslationCache {
    db: Db,
    path: std::path::PathBuf,
}

impl TranslationCache {
    pub fn open(path: &Path) -> Result<Self> {
        match sled::open(path) {
            Ok(db) => {
                tracing::debug!("Opened translation cache at {}", path.display());
                Ok(Self {
                    db,
                    path: path.to_path_buf(),
                })
            }
            Err(e) => {
                // Check if it's a corruption error
                let err_str = e.to_string();
                if err_str.contains("corruption")
                    || err_str.contains("CRC")
                    || err_str.contains("invalid")
                    || err_str.contains("magic")
                {
                    tracing::warn!(
                        "Cache corruption detected at {}. Attempting recovery...",
                        path.display()
                    );
                    Self::recover_and_open(path)
                } else {
                    Err(e).with_context(|| format!("Failed to open cache at {}", path.display()))
                }
            }
        }
    }

    /// Attempt to recover from a corrupted cache
    fn recover_and_open(path: &Path) -> Result<Self> {
        // Strategy: backup old cache, create new one
        let backup_path = path.with_extension("bak");

        if path.exists() {
            // Remove old backup if exists
            if backup_path.exists() {
                std::fs::remove_dir_all(&backup_path).ok();
            }

            // Rename corrupted cache to backup
            if let Err(e) = std::fs::rename(path, &backup_path) {
                tracing::warn!("Failed to backup corrupted cache: {}. Removing instead.", e);
                std::fs::remove_dir_all(path).with_context(|| {
                    format!("Failed to remove corrupted cache at {}", path.display())
                })?;
            } else {
                tracing::info!(
                    "Corrupted cache backed up to {}. Creating fresh cache.",
                    backup_path.display()
                );
            }
        }

        // Create fresh cache
        std::fs::create_dir_all(path)
            .with_context(|| format!("Failed to create cache directory {}", path.display()))?;

        let db = sled::open(path)
            .with_context(|| format!("Failed to create new cache at {}", path.display()))?;

        tracing::info!("Created fresh translation cache at {}", path.display());
        Ok(Self {
            db,
            path: path.to_path_buf(),
        })
    }

    pub fn open_default() -> Result<Self> {
        let path = Config::cache_path().context("Could not determine cache directory")?;

        std::fs::create_dir_all(&path)
            .with_context(|| format!("Failed to create cache directory {}", path.display()))?;

        Self::open(&path)
    }

    /// Open cache with automatic recovery on corruption
    pub fn open_with_recovery(path: &Path) -> Result<Self> {
        Self::open(path)
    }

    /// Get cached translation by content hash
    pub fn get(&self, hash: &str) -> Option<String> {
        match self.db.get(hash.as_bytes()) {
            Ok(Some(value)) => String::from_utf8(value.to_vec()).ok(),
            Ok(None) => None,
            Err(e) => {
                // Log read errors but don't crash
                tracing::warn!("Cache read error for key {}: {}", hash, e);
                None
            }
        }
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

    /// Get the cache path
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Check cache health and attempt repair if needed
    pub fn health_check(&self) -> Result<CacheHealth> {
        let entry_count = self.db.len();
        let disk_size = Self::calculate_disk_size(&self.path)?;

        // Check for potential corruption by trying to iterate
        let mut corrupted_keys = 0;
        for item in self.db.iter() {
            if item.is_err() {
                corrupted_keys += 1;
            }
        }

        let status = if corrupted_keys > 0 {
            CacheStatus::Degraded
        } else {
            CacheStatus::Healthy
        };

        Ok(CacheHealth {
            status,
            entry_count,
            disk_size_bytes: disk_size,
            corrupted_entries: corrupted_keys,
        })
    }

    fn calculate_disk_size(path: &Path) -> Result<u64> {
        let mut total = 0u64;
        if path.is_dir() {
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                let metadata = entry.metadata()?;
                if metadata.is_file() {
                    total += metadata.len();
                } else if metadata.is_dir() {
                    total += Self::calculate_disk_size(&entry.path())?;
                }
            }
        }
        Ok(total)
    }
}

/// Cache health information
#[derive(Debug, Clone)]
pub struct CacheHealth {
    pub status: CacheStatus,
    pub entry_count: usize,
    pub disk_size_bytes: u64,
    pub corrupted_entries: usize,
}

impl CacheHealth {
    /// Format disk size as human-readable string
    pub fn disk_size_human(&self) -> String {
        let bytes = self.disk_size_bytes;
        if bytes < 1024 {
            format!("{} B", bytes)
        } else if bytes < 1024 * 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else if bytes < 1024 * 1024 * 1024 {
            format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
        }
    }
}

/// Cache status indicator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheStatus {
    Healthy,
    Degraded,
    Corrupted,
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
