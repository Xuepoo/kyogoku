# Kyogoku Plugin Development Guide

This guide explains how to create custom format plugins for Kyogoku.

## Overview

Kyogoku supports external plugins for parsing and serializing custom file formats. Plugins can be written as:

- **WebAssembly (WASM)** modules - Cross-platform, sandboxed, recommended
- **Native libraries** - Platform-specific dynamic libraries (.so, .dylib, .dll)

Plugins are loaded from `~/.config/kyogoku/plugins/`.

## Plugin Structure

Each plugin requires:

```
plugins/
└── my-parser/
    ├── plugin.toml      # Manifest file
    └── my_parser.wasm   # Plugin binary
```

## Manifest File (plugin.toml)

```toml
[plugin]
name = "csv-parser"
version = "0.1.0"
description = "Parse CSV files for translation"
authors = ["Your Name <email@example.com>"]
plugin_type = "wasm"
binary = "csv_parser.wasm"
min_kyogoku_version = "0.5.0"

[parser]
extensions = ["csv", "tsv"]
priority = 10
```

### Fields

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Unique plugin identifier (kebab-case) |
| `version` | Yes | Semantic version (e.g., "0.1.0") |
| `description` | No | Human-readable description |
| `authors` | No | List of author names/emails |
| `plugin_type` | Yes | Either "wasm" or "native" |
| `binary` | Yes | Path to plugin binary (relative to manifest) |
| `min_kyogoku_version` | No | Minimum required Kyogoku version |
| `extensions` | Yes | File extensions to handle |
| `priority` | No | Higher values take precedence (default: 0) |

## Plugin Interface

Your plugin must implement two functions:

### parse

Parse file content into translation blocks.

```rust
fn parse(content: &[u8]) -> Result<Vec<TranslationBlock>>
```

### serialize

Write translated blocks back to the original format.

```rust
fn serialize(blocks: &[TranslationBlock], template: &[u8]) -> Result<Vec<u8>>
```

## TranslationBlock Structure

```rust
pub struct TranslationBlock {
    pub id: String,           // Unique ID (Blake3 hash of content)
    pub speaker: Option<String>,
    pub source: String,       // Original text
    pub target: Option<String>, // Translated text (filled by engine)
    pub metadata: serde_json::Value, // Format-specific data
}
```

### Metadata Examples

Store format-specific information in metadata for accurate reconstruction:

```json
// SRT subtitle
{
  "index": 1,
  "start_time": "00:01:23,456",
  "end_time": "00:01:25,789"
}

// ASS subtitle  
{
  "layer": 0,
  "style": "Default",
  "effect": "",
  "original_text": "{\\b1}Bold text{\\b0}"
}

// Game dialogue
{
  "line_number": 42,
  "label": "scene_intro"
}
```

## Writing a WASM Plugin (Rust)

### 1. Create a new project

```bash
cargo new --lib csv-parser
cd csv-parser
```

### 2. Configure Cargo.toml

```toml
[package]
name = "csv-parser"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
csv = "1.3"

[profile.release]
opt-level = "z"
lto = true
```

### 3. Implement the parser

```rust
// src/lib.rs
use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Serialize, Deserialize)]
pub struct TranslationBlock {
    pub id: String,
    pub speaker: Option<String>,
    pub source: String,
    pub target: Option<String>,
    pub metadata: serde_json::Value,
}

#[no_mangle]
pub extern "C" fn parse(content_ptr: *const u8, content_len: usize) -> *mut u8 {
    let content = unsafe { 
        std::slice::from_raw_parts(content_ptr, content_len) 
    };
    
    let result = parse_csv(content);
    let json = serde_json::to_vec(&result).unwrap();
    
    // Return JSON-encoded blocks
    let boxed = json.into_boxed_slice();
    let ptr = boxed.as_ptr() as *mut u8;
    std::mem::forget(boxed);
    ptr
}

fn parse_csv(content: &[u8]) -> Vec<TranslationBlock> {
    let mut blocks = Vec::new();
    let mut reader = csv::Reader::from_reader(content);
    
    for (idx, result) in reader.records().enumerate() {
        if let Ok(record) = result {
            let text = record.get(0).unwrap_or_default();
            blocks.push(TranslationBlock {
                id: format!("csv-{}", idx),
                speaker: None,
                source: text.to_string(),
                target: None,
                metadata: serde_json::json!({
                    "row": idx,
                    "columns": record.len()
                }),
            });
        }
    }
    
    blocks
}

#[no_mangle]
pub extern "C" fn serialize(
    blocks_ptr: *const u8, 
    blocks_len: usize,
    template_ptr: *const u8,
    template_len: usize
) -> *mut u8 {
    // Parse blocks from JSON
    let blocks_json = unsafe {
        std::slice::from_raw_parts(blocks_ptr, blocks_len)
    };
    let blocks: Vec<TranslationBlock> = serde_json::from_slice(blocks_json).unwrap();
    
    // Generate output
    let mut output = String::new();
    for block in blocks {
        let text = block.target.unwrap_or(block.source);
        output.push_str(&text);
        output.push('\n');
    }
    
    let boxed = output.into_bytes().into_boxed_slice();
    let ptr = boxed.as_ptr() as *mut u8;
    std::mem::forget(boxed);
    ptr
}
```

### 4. Build the WASM module

```bash
# Install wasm target
rustup target add wasm32-unknown-unknown

# Build
cargo build --release --target wasm32-unknown-unknown

# Copy to plugins directory
mkdir -p ~/.config/kyogoku/plugins/csv-parser
cp target/wasm32-unknown-unknown/release/csv_parser.wasm \
   ~/.config/kyogoku/plugins/csv-parser/
```

### 5. Create the manifest

```bash
cat > ~/.config/kyogoku/plugins/csv-parser/plugin.toml << 'EOF'
[plugin]
name = "csv-parser"
version = "0.1.0"
description = "Parse CSV files for translation"
plugin_type = "wasm"
binary = "csv_parser.wasm"

[parser]
extensions = ["csv", "tsv"]
EOF
```

## Managing Plugins

### List installed plugins

```bash
kyogoku plugin list
```

### Test a plugin

```bash
kyogoku translate test.csv --dry-run
```

### Remove a plugin

```bash
rm -rf ~/.config/kyogoku/plugins/csv-parser
```

## Best Practices

1. **Preserve structure**: Store enough metadata to perfectly reconstruct the original format
2. **Handle edge cases**: Empty files, malformed input, Unicode
3. **Test round-trips**: Ensure `parse()` → `serialize()` produces identical output
4. **Use meaningful IDs**: IDs should be stable across parses for caching
5. **Document your format**: Include a README in your plugin directory

## Troubleshooting

### Plugin not loading

1. Check manifest syntax: `toml verify plugin.toml`
2. Verify binary path is correct
3. Check file extensions don't conflict with built-in parsers
4. View logs: `RUST_LOG=debug kyogoku translate test.csv`

### WASM memory errors

- Ensure proper memory management (no dangling pointers)
- Use `wasm-opt` to optimize and validate the module
- Test with small files first

## Example Plugins

See the [examples/plugins/](../examples/plugins/) directory for reference implementations:

- `csv-parser/` - Simple CSV file parser
- `yaml-parser/` - YAML localization files
- `po-parser/` - GNU gettext PO files

## Support

- [GitHub Issues](https://github.com/Xuepoo/kyogoku/issues)
- [Discussions](https://github.com/Xuepoo/kyogoku/discussions)
