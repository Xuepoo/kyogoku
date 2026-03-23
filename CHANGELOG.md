# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **GUI Application**: Cross-platform desktop interface using Tauri 2.0.
  * Real-time translation progress tracking.
  * Interactive configuration editor.
  * Drag-and-drop file processing.
- **EPUB Parser**: Support for translating `.epub` electronic books.
  * Extracts XHTML content while preserving structure.
  * Re-packages translated content into valid EPUB files.
- **Parsers**: Added support for `.rpy` (Ren'Py) script files.
  * Parsing of dialogue lines and menu choices.
  * Preservation of Python indentation and structure.
- **Testing**: Added comprehensive integration test suite with `insta` snapshot testing.
- **Testing**: Added `tests/fixtures` directory with real-world sample files.
- **ASS/SSA Parser** (.ass, .ssa): Advanced SubStation Alpha subtitle format
  * Parses dialogue with timing, styles, and speaker information
  * Strips override tags (bold, italic, positioning) while preserving text
  * Handles line breaks (\N, \n) correctly
- **WebVTT Parser** (.vtt): HTML5 video text tracks format
  * Parses cue timing and settings (align, position, etc.)
  * Strips HTML-like formatting tags
  * Supports optional cue identifiers

### Changed
- Added nom 8.0 dependency for advanced parsing capabilities
- Updated README and documentation to list 8 supported file extensions (txt, srt, json, ass, ssa, vtt, webvtt, rpy)

### Planned
- Tauri 2.0 GUI application
- RAG with local vector database

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
