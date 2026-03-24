# CSV Parser Plugin Example

This directory contains an example plugin for Kyogoku that parses CSV files.

## Plugin Structure

```
csv-plugin/
├── plugin.toml      # Plugin manifest
├── src/
│   └── lib.rs       # Rust source (compiles to WASM)
├── Cargo.toml       # Rust project config
└── README.md        # This file
```

## Building

```bash
# Install wasm32-unknown-unknown target
rustup target add wasm32-unknown-unknown

# Build the WASM module
cargo build --release --target wasm32-unknown-unknown

# Copy to plugin directory
cp target/wasm32-unknown-unknown/release/csv_parser.wasm ./
```

## Installing

Copy the entire `csv-plugin/` directory to:
- `~/.config/kyogoku/plugins/csv-plugin/`

Or for project-local plugins:
- `./kyogoku-plugins/csv-plugin/`

## CSV Format

The parser expects CSV files with the following format:

```csv
key,source,context
greeting,こんにちは,Used for hello
farewell,さようなら,Used for goodbye
```

Column mapping:
- `key`: Unique identifier (stored in metadata)
- `source`: Text to translate
- `context`: Optional context hint (stored in metadata)

## WASM Interface

The plugin exports these functions:

```rust
// Allocate memory
fn alloc(size: i32) -> i32;

// Free memory
fn dealloc(ptr: i32, size: i32);

// Parse CSV content, returns JSON pointer
fn parse(ptr: i32, len: i32) -> i32;

// Serialize blocks back to CSV, returns output pointer
fn serialize(blocks_ptr: i32, blocks_len: i32, template_ptr: i32, template_len: i32) -> i32;

// Get length of last result
fn get_result_len() -> i32;
```
