//! Plugin manifest (plugin.toml) schema.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Type of plugin binary
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum PluginType {
    /// Native dynamic library
    Native,
    /// WebAssembly module
    #[default]
    Wasm,
}


/// Plugin manifest structure (plugin.toml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Plugin metadata
    pub plugin: PluginMeta,

    /// Parser configuration
    #[serde(default)]
    pub parser: ParserConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMeta {
    /// Unique plugin name (kebab-case)
    pub name: String,

    /// Semver version
    pub version: String,

    /// Human-readable description
    #[serde(default)]
    pub description: String,

    /// Plugin author(s)
    #[serde(default)]
    pub authors: Vec<String>,

    /// Plugin type (native or wasm)
    #[serde(default)]
    pub plugin_type: PluginType,

    /// Path to the plugin binary (relative to manifest)
    pub binary: PathBuf,

    /// Minimum Kyogoku version required
    #[serde(default)]
    pub min_kyogoku_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParserConfig {
    /// File extensions this parser handles
    #[serde(default)]
    pub extensions: Vec<String>,

    /// Priority (higher = preferred when multiple parsers match)
    #[serde(default)]
    pub priority: i32,
}

impl PluginManifest {
    /// Load manifest from a TOML file
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let manifest: Self = toml::from_str(&content)?;
        Ok(manifest)
    }

    /// Get full path to the plugin binary
    pub fn binary_path(&self, manifest_dir: &std::path::Path) -> PathBuf {
        manifest_dir.join(&self.plugin.binary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_parse() {
        let toml = r#"
[plugin]
name = "csv-parser"
version = "0.1.0"
description = "Parse CSV files for translation"
plugin_type = "wasm"
binary = "csv_parser.wasm"

[parser]
extensions = ["csv", "tsv"]
priority = 10
"#;
        let manifest: PluginManifest = toml::from_str(toml).unwrap();
        assert_eq!(manifest.plugin.name, "csv-parser");
        assert_eq!(manifest.plugin.plugin_type, PluginType::Wasm);
        assert_eq!(manifest.parser.extensions, vec!["csv", "tsv"]);
    }
}
