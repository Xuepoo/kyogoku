//! Plugin registry - manages loaded plugins and provides parser lookup.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use tracing::{debug, info};

use kyogoku_parser::{Parser, TranslationBlock};

use super::{PluginInfo, PluginLoader};

/// Type alias for plugin parse function
type ParseFn = Box<dyn Fn(&[u8]) -> Result<Vec<TranslationBlock>> + Send + Sync>;

/// Type alias for plugin serialize function
type SerializeFn = Box<dyn Fn(&[TranslationBlock], &[u8]) -> Result<Vec<u8>> + Send + Sync>;

/// A loaded plugin that implements the Parser trait.
pub struct LoadedPlugin {
    pub info: PluginInfo,
    // For now, we'll use a simple function pointer approach
    // WASM runtime will be added in a follow-up task
    parse_fn: Option<ParseFn>,
    serialize_fn: Option<SerializeFn>,
}

impl LoadedPlugin {
    /// Create a placeholder plugin (for listing purposes)
    pub fn placeholder(info: PluginInfo) -> Self {
        Self {
            info,
            parse_fn: None,
            serialize_fn: None,
        }
    }

    /// Check if the plugin is fully loaded and functional
    pub fn is_loaded(&self) -> bool {
        self.parse_fn.is_some() && self.serialize_fn.is_some()
    }
}

impl Parser for LoadedPlugin {
    fn extensions(&self) -> &[&str] {
        // This is a bit awkward due to lifetime requirements
        // We'll use a static empty slice as fallback
        // In practice, we check extensions via PluginRegistry
        &[]
    }

    fn parse(&self, content: &[u8]) -> Result<Vec<TranslationBlock>> {
        if let Some(ref parse_fn) = self.parse_fn {
            parse_fn(content)
        } else {
            anyhow::bail!("Plugin {} is not loaded", self.info.name)
        }
    }

    fn serialize(&self, blocks: &[TranslationBlock], template: &[u8]) -> Result<Vec<u8>> {
        if let Some(ref serialize_fn) = self.serialize_fn {
            serialize_fn(blocks, template)
        } else {
            anyhow::bail!("Plugin {} is not loaded", self.info.name)
        }
    }
}

/// Registry of loaded plugins.
pub struct PluginRegistry {
    plugins: HashMap<String, Arc<LoadedPlugin>>,
    extension_map: HashMap<String, String>, // extension -> plugin name
}

impl PluginRegistry {
    /// Create an empty registry
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            extension_map: HashMap::new(),
        }
    }

    /// Discover and register all available plugins
    pub fn load_all(&mut self) -> Result<()> {
        let loader = PluginLoader::new();
        let discovered = loader.discover();

        for info in discovered {
            self.register_plugin(info)?;
        }

        Ok(())
    }

    /// Register a plugin from its info
    fn register_plugin(&mut self, info: PluginInfo) -> Result<()> {
        let name = info.name.clone();
        let extensions = info.extensions.clone();

        // Create placeholder plugin (actual loading happens on demand)
        let plugin = Arc::new(LoadedPlugin::placeholder(info));
        self.plugins.insert(name.clone(), plugin);

        // Map extensions to plugin
        for ext in extensions {
            debug!("Registering extension '{}' -> plugin '{}'", ext, name);
            self.extension_map.insert(ext.to_lowercase(), name.clone());
        }

        info!("Registered plugin: {}", name);
        Ok(())
    }

    /// Get a plugin by name
    pub fn get(&self, name: &str) -> Option<Arc<LoadedPlugin>> {
        self.plugins.get(name).cloned()
    }

    /// Get a plugin by file extension
    pub fn get_by_extension(&self, ext: &str) -> Option<Arc<LoadedPlugin>> {
        let name = self.extension_map.get(&ext.to_lowercase())?;
        self.plugins.get(name).cloned()
    }

    /// Check if a plugin can handle a file
    pub fn can_handle(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| self.extension_map.contains_key(&ext.to_lowercase()))
    }

    /// List all registered plugins
    pub fn list(&self) -> Vec<&PluginInfo> {
        self.plugins.values().map(|p| &p.info).collect()
    }

    /// Get supported extensions from plugins
    pub fn supported_extensions(&self) -> Vec<&str> {
        self.extension_map.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_registry() {
        let registry = PluginRegistry::new();
        assert!(registry.list().is_empty());
        assert!(registry.supported_extensions().is_empty());
    }
}
