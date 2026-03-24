//! Plugin loader - discovers and loads plugins from disk.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tracing::{debug, info, warn};

use super::{PluginInfo, PluginManifest, PluginType};

/// Discovers and loads plugins from the filesystem.
pub struct PluginLoader {
    plugin_dirs: Vec<PathBuf>,
}

impl PluginLoader {
    /// Create a new loader with default plugin directories.
    pub fn new() -> Self {
        let mut dirs = Vec::new();

        // User plugin directory: ~/.config/kyogoku/plugins/
        if let Some(config_dir) = crate::config::Config::config_dir() {
            dirs.push(config_dir.join("plugins"));
        }

        // Project-local plugins: ./kyogoku-plugins/
        dirs.push(PathBuf::from("./kyogoku-plugins"));

        Self { plugin_dirs: dirs }
    }

    /// Create loader with custom directories
    pub fn with_dirs(dirs: Vec<PathBuf>) -> Self {
        Self { plugin_dirs: dirs }
    }

    /// Discover all available plugins
    pub fn discover(&self) -> Vec<PluginInfo> {
        let mut plugins = Vec::new();

        for dir in &self.plugin_dirs {
            if !dir.exists() {
                debug!("Plugin directory does not exist: {}", dir.display());
                continue;
            }

            match self.scan_directory(dir) {
                Ok(found) => plugins.extend(found),
                Err(e) => warn!("Failed to scan plugin directory {}: {}", dir.display(), e),
            }
        }

        info!("Discovered {} plugins", plugins.len());
        plugins
    }

    /// Scan a directory for plugin manifests
    fn scan_directory(&self, dir: &Path) -> Result<Vec<PluginInfo>> {
        let mut plugins = Vec::new();

        let entries = std::fs::read_dir(dir)
            .with_context(|| format!("Failed to read directory: {}", dir.display()))?;

        for entry in entries.flatten() {
            let path = entry.path();

            // Look for plugin.toml in subdirectories
            if path.is_dir() {
                let manifest_path = path.join("plugin.toml");
                if manifest_path.exists() {
                    match self.load_plugin_info(&manifest_path) {
                        Ok(info) => {
                            debug!("Found plugin: {} v{}", info.name, info.version);
                            plugins.push(info);
                        }
                        Err(e) => {
                            warn!(
                                "Failed to load plugin manifest {}: {}",
                                manifest_path.display(),
                                e
                            );
                        }
                    }
                }
            }

            // Also check for standalone .wasm files
            if path.extension().is_some_and(|ext| ext == "wasm")
                && let Some(info) = self.create_standalone_plugin_info(&path) {
                    debug!("Found standalone WASM plugin: {}", info.name);
                    plugins.push(info);
                }
        }

        Ok(plugins)
    }

    /// Load plugin info from a manifest file
    fn load_plugin_info(&self, manifest_path: &Path) -> Result<PluginInfo> {
        let manifest = PluginManifest::load(manifest_path)?;
        let manifest_dir = manifest_path.parent().unwrap_or(Path::new("."));
        let binary_path = manifest.binary_path(manifest_dir);

        // Verify binary exists
        if !binary_path.exists() {
            anyhow::bail!(
                "Plugin binary not found: {} (expected at {})",
                manifest.plugin.name,
                binary_path.display()
            );
        }

        Ok(PluginInfo {
            name: manifest.plugin.name,
            version: manifest.plugin.version,
            description: manifest.plugin.description,
            extensions: manifest.parser.extensions,
            plugin_type: manifest.plugin.plugin_type,
            path: binary_path,
        })
    }

    /// Create plugin info for standalone WASM files (infer metadata from filename)
    fn create_standalone_plugin_info(&self, path: &Path) -> Option<PluginInfo> {
        let stem = path.file_stem()?.to_str()?;

        // Parse name_ext pattern: "csv_parser.wasm" -> name="csv-parser", ext=["csv"]
        let name = stem.replace('_', "-");
        let ext = if name.ends_with("-parser") {
            vec![name.trim_end_matches("-parser").to_string()]
        } else {
            Vec::new()
        };

        Some(PluginInfo {
            name: name.clone(),
            version: "0.0.0".to_string(),
            description: format!("Standalone plugin: {}", stem),
            extensions: ext,
            plugin_type: PluginType::Wasm,
            path: path.to_path_buf(),
        })
    }

    /// Get plugin directories
    pub fn plugin_dirs(&self) -> &[PathBuf] {
        &self.plugin_dirs
    }
}

impl Default for PluginLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_discover_empty() {
        let tmp = TempDir::new().unwrap();
        let loader = PluginLoader::with_dirs(vec![tmp.path().to_path_buf()]);
        let plugins = loader.discover();
        assert!(plugins.is_empty());
    }

    #[test]
    fn test_discover_with_manifest() {
        let tmp = TempDir::new().unwrap();
        let plugin_dir = tmp.path().join("test-plugin");
        std::fs::create_dir_all(&plugin_dir).unwrap();

        // Create manifest
        let manifest = r#"
[plugin]
name = "test-plugin"
version = "1.0.0"
description = "Test plugin"
plugin_type = "wasm"
binary = "plugin.wasm"

[parser]
extensions = ["test"]
"#;
        std::fs::write(plugin_dir.join("plugin.toml"), manifest).unwrap();
        std::fs::write(plugin_dir.join("plugin.wasm"), b"fake wasm").unwrap();

        let loader = PluginLoader::with_dirs(vec![tmp.path().to_path_buf()]);
        let plugins = loader.discover();

        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "test-plugin");
        assert_eq!(plugins[0].version, "1.0.0");
        assert_eq!(plugins[0].extensions, vec!["test"]);
    }
}
