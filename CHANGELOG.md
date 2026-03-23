# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned
- nom-based Ren'Py (.rpy) parser
- ASS/SSA subtitle parser
- Tauri 2.0 GUI application

---

## [0.1.0] - 2026-03-23

### Added

#### Core Features
- **Translation Engine**: Full translation pipeline with context window support
- **Multi-provider API Client**: OpenAI, DeepSeek, Anthropic, Google, local (Ollama)
- **Content-addressed Caching**: Blake3 hashing with sled KV store for incremental translation
- **Glossary System**: JSON-based terminology enforcement with context hints

#### Parsers
- **TXT Parser**: Line-by-line plain text translation
- **SRT Parser**: Subtitle translation with timestamp preservation
- **JSON Parser**: MTool-compatible format with nested object support

#### CLI Commands
- `kyogoku init` - Initialize configuration with interactive prompts
- `kyogoku config show` - Display current configuration
- `kyogoku config set <key> <value>` - Set configuration values
- `kyogoku config test` - Test API connection
- `kyogoku translate <input> [-o output]` - Translate files or directories
- `kyogoku cache stats` - Show cache statistics
- `kyogoku cache clear` - Clear translation cache

#### Configuration
- XDG-compliant configuration paths (`~/.config/kyogoku/config.toml`)
- Environment variable support for API keys (`ENV_VAR` placeholder)
- Four translation styles: literary, casual, formal, technical
- Configurable context window size and max concurrency

#### Documentation
- README with quick start guide
- USER_GUIDE.md with detailed usage instructions
- CONFIG.md with complete parameter reference
- DEVELOPER.md with contributing guidelines
- ARCHITECTURE.md with system design documentation

### Technical Details
- Rust 2024 edition
- Async runtime with Tokio
- Release profile optimized for binary size (LTO, strip, abort on panic)
- Workspace structure with three crates: cli, core, parser

---

## Version History

| Version | Date | Highlights |
|---------|------|------------|
| 0.1.0 | 2026-03-23 | Initial release with CLI MVP |

---

[Unreleased]: https://github.com/xuepoo/kyogoku/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/xuepoo/kyogoku/releases/tag/v0.1.0
