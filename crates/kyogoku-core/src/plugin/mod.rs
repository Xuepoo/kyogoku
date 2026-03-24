//! Plugin system for Kyogoku.
//!
//! Allows loading external parsers from:
//! - Native dynamic libraries (.so, .dylib, .dll)
//! - WebAssembly modules (.wasm)
//!
//! Plugins are loaded from `~/.config/kyogoku/plugins/`

mod loader;
mod manifest;
mod registry;
mod wasm;

pub use loader::PluginLoader;
pub use manifest::{PluginManifest, PluginType};
pub use registry::PluginRegistry;
pub use wasm::WasmPluginRunner;

use anyhow::Result;
use kyogoku_parser::TranslationBlock;

/// Plugin trait that external parsers must implement.
/// This is the interface between Kyogoku and plugin code.
pub trait Plugin: Send + Sync {
    /// Plugin name (e.g., "csv-parser")
    fn name(&self) -> &str;

    /// Plugin version (semver)
    fn version(&self) -> &str;

    /// Human-readable description
    fn description(&self) -> &str;

    /// File extensions this plugin handles (e.g., ["csv", "tsv"])
    fn extensions(&self) -> Vec<String>;

    /// Parse file content into translation blocks
    fn parse(&self, content: &[u8]) -> Result<Vec<TranslationBlock>>;

    /// Serialize translated blocks back to original format
    fn serialize(&self, blocks: &[TranslationBlock], template: &[u8]) -> Result<Vec<u8>>;
}

/// Plugin metadata for listing/management
#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub extensions: Vec<String>,
    pub plugin_type: PluginType,
    pub path: std::path::PathBuf,
}
