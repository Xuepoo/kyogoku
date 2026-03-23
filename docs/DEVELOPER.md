# Kyogoku Developer Guide

Guide for setting up the development environment, running tests, and contributing to Kyogoku.

## Table of Contents

1. [Development Environment](#development-environment)
2. [Project Structure](#project-structure)
3. [Building](#building)
4. [Testing](#testing)
5. [Code Style](#code-style)
6. [Contributing](#contributing)

---

## Development Environment

### Prerequisites

| Tool | Version | Purpose |
|------|---------|---------|
| Rust | 1.85+ (2024 edition) | Compiler and toolchain |
| Git | Latest | Version control |
| cargo-watch | Optional | Auto-rebuild on changes |

### Setup (Arch Linux)

```bash
# Install Rust
sudo pacman -S rustup gcc
rustup default stable

# Clone repository
git clone https://github.com/xuepoo/kyogoku
cd kyogoku

# Install development tools
cargo install cargo-watch cargo-nextest

# Verify setup
cargo build
cargo test --workspace
```

### Setup (Other Linux / macOS)

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Clone and build
git clone https://github.com/xuepoo/kyogoku
cd kyogoku
cargo build
```

### IDE Setup

**VS Code:**
```bash
# Install rust-analyzer extension
code --install-extension rust-lang.rust-analyzer
```

**Recommended settings (.vscode/settings.json):**
```json
{
  "rust-analyzer.cargo.features": "all",
  "rust-analyzer.checkOnSave.command": "clippy"
}
```

---

## Project Structure

```
kyogoku/
├── Cargo.toml              # Workspace manifest
├── Cargo.lock              # Dependency lock file
├── README.md               # Project overview
├── CHANGELOG.md            # Version history
├── LICENSE                 # MIT License
├── .github/
│   ├── copilot-instructions.md  # AI assistant context
│   └── workflows/               # CI/CD (future)
├── crates/
│   ├── kyogoku-cli/        # CLI binary
│   │   └── src/
│   │       ├── main.rs     # Entry point
│   │       └── commands/   # Subcommand implementations
│   ├── kyogoku-core/       # Core library
│   │   └── src/
│   │       ├── lib.rs      # Module exports
│   │       ├── config.rs   # Configuration system
│   │       ├── api.rs      # LLM API client
│   │       ├── cache.rs    # sled-based cache
│   │       ├── glossary.rs # Terminology management
│   │       └── engine.rs   # Translation engine
│   └── kyogoku-parser/     # Parser library
│       └── src/
│           ├── lib.rs      # Module exports
│           ├── block.rs    # TranslationBlock
│           ├── parser.rs   # Parser trait
│           ├── txt.rs      # Plain text parser
│           ├── srt.rs      # SRT subtitle parser
│           └── json.rs     # JSON/MTool parser
└── docs/                   # Documentation
```

### Crate Dependencies

```
kyogoku-cli
    ├── kyogoku-core
    │   └── kyogoku-parser
    └── kyogoku-parser
```

---

## Building

### Debug Build

```bash
cargo build
```

Binary location: `target/debug/kyogoku`

### Release Build

```bash
cargo build --release
```

Binary location: `target/release/kyogoku`

### Release Profile

The release profile is optimized for binary size:

```toml
[profile.release]
lto = true           # Link-time optimization
codegen-units = 1    # Single codegen unit
panic = "abort"      # Abort on panic (smaller binary)
strip = true         # Strip debug symbols
```

### Watch Mode

Auto-rebuild on file changes:

```bash
cargo watch -x build
```

---

## Testing

### Run All Tests

```bash
cargo test --workspace
```

### Run Tests for Specific Crate

```bash
cargo test -p kyogoku-core
cargo test -p kyogoku-parser
cargo test -p kyogoku-cli
```

### Run Specific Test

```bash
cargo test -p kyogoku-parser test_srt_parse
```

### Test with Output

```bash
cargo test --workspace -- --nocapture
```

### Using cargo-nextest (Faster)

```bash
cargo nextest run
```

### Test Coverage

```bash
# Install coverage tool
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --workspace --out Html
```

---

## Code Style

### Formatting

```bash
# Check formatting
cargo fmt --check

# Apply formatting
cargo fmt
```

### Linting

```bash
# Run clippy
cargo clippy --workspace

# Auto-fix clippy warnings
cargo clippy --fix --workspace --allow-dirty
```

### Pre-commit Checks

Before committing, run:

```bash
cargo fmt
cargo clippy --workspace
cargo test --workspace
```

### Style Guidelines

1. **Naming:** Use Rust naming conventions (snake_case for functions, PascalCase for types)
2. **Imports:** Group by stdlib, external crates, local modules
3. **Comments:** Document public APIs with `///`, avoid obvious comments
4. **Error Handling:** Use `anyhow::Result` for applications, `thiserror` for libraries (future)
5. **Async:** Use `tokio` runtime, avoid blocking in async contexts

---

## Contributing

### Getting Started

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Make your changes
4. Run tests: `cargo test --workspace`
5. Run lints: `cargo clippy --workspace`
6. Commit with descriptive message
7. Push and create a Pull Request

### Commit Messages

Follow conventional commits:

```
feat: add ASS subtitle parser
fix: correct SRT timestamp parsing
docs: update CONFIG.md with new options
refactor: simplify context window logic
test: add edge case tests for JSON parser
```

### Pull Request Guidelines

1. **Title:** Clear, concise description of the change
2. **Description:** Explain what and why, link related issues
3. **Tests:** Add tests for new functionality
4. **Documentation:** Update docs if user-facing behavior changes
5. **Changelog:** Add entry to CHANGELOG.md under `[Unreleased]`

### Adding a New Parser

1. Create `crates/kyogoku-parser/src/myformat.rs`
2. Implement the `Parser` trait:

```rust
use crate::{Parser, TranslationBlock};
use anyhow::Result;

pub struct MyFormatParser;

impl Parser for MyFormatParser {
    fn parse(&self, content: &str) -> Result<Vec<TranslationBlock>> {
        // Parse content into blocks
        todo!()
    }

    fn serialize(&self, blocks: &[TranslationBlock], template: &str) -> Result<String> {
        // Serialize blocks back to format
        todo!()
    }
}
```

3. Register in `parser.rs`:

```rust
registry.register("myf", Box::new(MyFormatParser));
```

4. Add tests in the same file
5. Update documentation

### Adding a New Config Option

1. Add field to appropriate struct in `config.rs`
2. Add `#[serde(default)]` or default function
3. Update `CONFIG.md` with the new option
4. Add test for serialization round-trip

---

## Development Commands Cheatsheet

| Command | Description |
|---------|-------------|
| `cargo build` | Debug build |
| `cargo build --release` | Release build |
| `cargo test --workspace` | Run all tests |
| `cargo clippy --workspace` | Lint all code |
| `cargo fmt` | Format code |
| `cargo doc --open` | Generate and view docs |
| `cargo run -- --help` | Run CLI with help |
| `cargo watch -x build` | Auto-rebuild |
| `cargo nextest run` | Fast test runner |

---

## Troubleshooting

### Build Errors

**Missing system dependencies:**
```bash
# Arch Linux
sudo pacman -S openssl pkg-config

# Ubuntu
sudo apt install libssl-dev pkg-config
```

**Rust version too old:**
```bash
rustup update stable
```

### Test Failures

**Temporary directory issues:**
```bash
# Ensure /tmp has space
df -h /tmp
```

**Cache conflicts:**
```bash
cargo clean
cargo test --workspace
```

---

*Last updated: 2026-03-23*
