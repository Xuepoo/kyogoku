# Kyogoku

> *"AI-powered translation engine for literature, light novels, and game scripts."*

[![Build Status](https://img.shields.io/github/actions/workflow/status/xuepoo/kyogoku/ci.yml?branch=main&style=flat-square)](https://github.com/xuepoo/kyogoku/actions)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024-orange?style=flat-square)](https://www.rust-lang.org/)

Kyogoku (京极) derives its name from "驱邪" (exorcism), aiming to banish the common "miasma" of machine translation: inconsistent terminology, lost context, and broken formatting. Through LLM deep inference and structured context management, Kyogoku delivers coherent, literary-quality translations.

## ✨ Features

- **Multi-format Support**: TXT, SRT, ASS/SSA, WebVTT subtitles, JSON (MTool format)
- **Intelligent Caching**: Blake3 content hashing with sled KV store for incremental translation
- **Context Window**: Maintains translation consistency with sliding window of previous translations
- **Glossary System**: Custom terminology enforcement for character names, locations, etc.
- **Multiple Providers**: OpenAI, DeepSeek, Anthropic, Google, local LLMs (Ollama)
- **XDG Compliant**: Clean configuration following freedesktop.org standards

## 🚀 Quick Start

### Installation (Arch Linux / Source)

```bash
# Clone repository
git clone https://github.com/xuepoo/kyogoku
cd kyogoku

# Build release binary
cargo build --release

# Install to PATH
sudo cp target/release/kyogoku /usr/local/bin/
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

## 🔧 CLI Commands

```
kyogoku [OPTIONS] <COMMAND>

Commands:
  init       Initialize configuration
  config     Manage configuration (show/set/test)
  translate  Translate files or directories
  cache      Cache management (stats/clear)

Options:
  -v, --verbose  Enable verbose logging
  -h, --help     Print help
  -V, --version  Print version
```

## 📁 Project Structure

```
kyogoku/
├── crates/
│   ├── kyogoku-cli/      # Command-line interface
│   ├── kyogoku-core/     # Config, API, cache, engine
│   └── kyogoku-parser/   # Format parsers (TXT, SRT, JSON)
├── docs/                 # Documentation
└── Cargo.toml           # Workspace manifest
```

## 📜 License

MIT License - see [LICENSE](LICENSE) for details.

## 🙏 Acknowledgments

Inspired by [LinguaGacha](https://github.com/neavo/LinguaGacha) and the visual novel translation community.

