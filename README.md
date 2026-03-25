# Kyogoku

> *"AI-powered translation engine for literature, light novels, and game scripts."*

[![Build Status](https://img.shields.io/github/actions/workflow/status/xuepoo/kyogoku/ci.yml?branch=main&style=flat-square)](https://github.com/xuepoo/kyogoku/actions)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024-orange?style=flat-square)](https://www.rust-lang.org/)

Kyogoku (京极) derives its name from "驱邪" (exorcism), aiming to banish the common "miasma" of machine translation: inconsistent terminology, lost context, and broken formatting. Through LLM deep inference and structured context management, Kyogoku delivers coherent, literary-quality translations.

## ✨ Features

- **Multi-format Support**: TXT, SRT, ASS/SSA, WebVTT, EPUB, Ren'Py scripts, JSON (MTool format), Markdown
- **Plugin System**: Extensible parser architecture with WASM runtime support
- **Intelligent Caching**: Blake3 content hashing with sled KV store for incremental translation
- **Context Window**: Maintains translation consistency with sliding window of previous translations
- **Glossary System**: Custom terminology enforcement for character names, locations, etc.
- **Multiple Providers**: OpenAI, DeepSeek, Anthropic, Google, local LLMs (Ollama)
- **Batch Processing**: Parallel translation with configurable concurrency and retry logic
- **RAG Memory** (Beta): Semantic search using local ONNX embeddings for context-aware translation
- **GUI Application**: Tauri 2.0-based desktop app with real-time translation preview and virtual scrolling
- **XDG Compliant**: Clean configuration following freedesktop.org standards

## 🚀 Quick Start

### Installation (Arch Linux / Source)

```bash
# Clone repository
git clone https://github.com/xuepoo/kyogoku
cd kyogoku

# Build release binary (CLI only)
cargo build --release --bin kyogoku

# Install to PATH
sudo cp target/release/kyogoku /usr/local/bin/
```

### GUI Installation

The GUI requires Node.js and pnpm:

```bash
# Install Node.js dependencies
cd crates/kyogoku-gui
pnpm install

# Run development version
pnpm tauri dev

# Build production release
pnpm tauri build
```

### Configuration

Initialize configuration with your API key:

```bash
# Create default config
kyogoku init

# Set your API provider and key
kyogoku config set api.provider deepseek
kyogoku config set api.api_key "your-api-key-here"

# Or use environment variables (recommended)
export DEEPSEEK_API_KEY="your-api-key-here"
kyogoku config set api.api_key ENV_VAR
```

### Basic Usage

```bash
# Translate a single file
kyogoku translate ./script.json -o ./output

# Translate a directory
kyogoku translate ./input_folder -o ./translated

# Specify source/target languages
kyogoku translate ./novel.txt --from ja --to zh

# Use custom glossary
kyogoku translate ./game.json --glossary ./glossary.json
```

## 📖 Documentation

| Document | Description |
|----------|-------------|
| [User Guide](docs/USER_GUIDE.md) | Installation, authentication, usage workflows |
| [Configuration Reference](docs/CONFIG.md) | Complete config.toml parameter reference |
| [Developer Guide](docs/DEVELOPER.md) | Development setup, testing, contributing |
| [Architecture](docs/ARCHITECTURE.md) | System design, data flow, module structure |
| [Roadmap](docs/ROADMAP.md) | Future features and milestones |
| [Changelog](CHANGELOG.md) | Version history and release notes |

## ✅ CI Status

GitHub Actions CI runs on:
- `push` to `dev` and `main`
- `pull_request` to `dev` and `main`

Checks include formatting (`rustfmt`), lint (`clippy -D warnings`), and workspace tests.

## 🔧 CLI Commands

```
kyogoku [OPTIONS] <COMMAND>

Commands:
  init       Initialize configuration
  config     Manage configuration (show/set/test)
  translate  Translate files or directories
  cache      Cache management (stats/clear)
  plugin     Plugin management (list/info/dirs)

Options:
  -v, --verbose  Verbose output (show info level logs)
  -d, --debug    Debug output (show debug level logs with tracing spans)
  -q, --quiet    Quiet mode (suppress all non-error output)
  -h, --help     Print help
  -V, --version  Print version
```

### Translation Options

```bash
# Basic translation
kyogoku translate <INPUT> -o <OUTPUT>

# Language options
--from <LANG>       Source language (default: from config)
--to <LANG>         Target language (default: from config)

# Advanced options
--glossary <PATH>   Custom glossary file
--no-cache          Skip cache lookup (force fresh translation)
--dry-run           Preview blocks without API calls
--format <EXT>      Force specific format (e.g., txt, json)
--json              Output results as JSON

# Examples
kyogoku translate novel.txt --from ja --to en --dry-run
kyogoku translate ./scripts/ -o ./translated --glossary ./terms.json
kyogoku translate game.json --no-cache --json > result.json
```

### Cache Management

```bash
# Show cache statistics (entries, size, health status)
kyogoku cache stats

# Clear all cached translations
kyogoku cache clear
```

### Plugin System

Kyogoku supports custom file format parsers via plugins:

```bash
# List installed plugins
kyogoku plugin list

# Show plugin information
kyogoku plugin info csv-parser

# Show plugin directories
kyogoku plugin dirs
```

Plugins are discovered from:
- `~/.config/kyogoku/plugins/` (user plugins)
- `./kyogoku-plugins/` (project plugins)

See [examples/csv-plugin/](examples/csv-plugin/) for a complete plugin example.

## 📁 Project Structure

```
kyogoku/
├── crates/
│   ├── kyogoku-cli/      # Command-line interface
│   ├── kyogoku-core/     # Config, API, cache, engine, RAG, plugins
│   ├── kyogoku-parser/   # Format parsers (TXT, SRT, JSON, EPUB, etc.)
│   ├── kyogoku-i18n/     # Internationalization (Fluent)
│   └── kyogoku-gui/      # Tauri 2.0 desktop application
├── examples/
│   └── csv-plugin/       # Example WASM plugin for CSV format
├── docs/                 # Documentation
├── models/               # ONNX models for RAG (optional)
└── Cargo.toml           # Workspace manifest
```

## 📜 License

MIT License - see [LICENSE](LICENSE) for details.

## 🙏 Acknowledgments

Inspired by [LinguaGacha](https://github.com/neavo/LinguaGacha) and the visual novel translation community.
