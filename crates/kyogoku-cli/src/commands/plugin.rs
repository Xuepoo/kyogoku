//! Plugin management commands.

use anyhow::Result;
use kyogoku_core::{PluginLoader, PluginRegistry};

/// List all installed plugins
pub async fn list() -> Result<()> {
    let mut registry = PluginRegistry::new();
    registry.load_all()?;

    let plugins = registry.list();

    if plugins.is_empty() {
        println!("No plugins installed.");
        println!();
        println!("Plugin directories:");
        for dir in PluginLoader::new().plugin_dirs() {
            println!("  {}", dir.display());
        }
        return Ok(());
    }

    println!("Installed plugins:\n");
    for plugin in plugins {
        println!(
            "  {} v{} - {}",
            plugin.name, plugin.version, plugin.description
        );
        if !plugin.extensions.is_empty() {
            println!("    Extensions: {}", plugin.extensions.join(", "));
        }
        println!("    Type: {:?}", plugin.plugin_type);
        println!();
    }

    Ok(())
}

/// Show plugin details
pub async fn info(name: &str) -> Result<()> {
    let mut registry = PluginRegistry::new();
    registry.load_all()?;

    match registry.get(name) {
        Some(plugin) => {
            let info = &plugin.info;
            println!("Plugin: {}", info.name);
            println!("Version: {}", info.version);
            println!("Description: {}", info.description);
            println!("Type: {:?}", info.plugin_type);
            println!("Extensions: {}", info.extensions.join(", "));
            println!("Path: {}", info.path.display());
        }
        None => {
            println!("Plugin '{}' not found.", name);
            println!();
            println!("Use 'kyogoku plugin list' to see available plugins.");
        }
    }

    Ok(())
}

/// Show plugin directories
pub async fn dirs() -> Result<()> {
    let loader = PluginLoader::new();

    println!("Plugin directories:\n");
    for dir in loader.plugin_dirs() {
        let exists = dir.exists();
        let status = if exists { "✓" } else { "✗ (not found)" };
        println!("  {} {}", status, dir.display());
    }

    println!();
    println!("To install a plugin, create a directory with:");
    println!("  plugin.toml  - Plugin manifest");
    println!("  plugin.wasm  - WebAssembly binary");

    Ok(())
}
