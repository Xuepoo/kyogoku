# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **CLI Enhancements**:
  * `--dry-run` flag to preview translation without API calls.
  * `--format` flag to force specific parser format.
  * `--json` flag for machine-readable JSON output.
- **Retry Logic**: Exponential backoff for transient API failures (rate limits, 5xx errors).
- **Markdown Parser** (.md, .markdown):
  * Preserves YAML/TOML frontmatter unchanged.
  * Skips code blocks (``` and ~~~) from translation.
  * Extracts headers and paragraphs for translation.

## [0.2.0] - 2026-03-23

### Added
- **GUI Application**: Cross-platform desktop interface using Tauri 2.0.
  * Real-time translation progress tracking with live preview.
  * Interactive configuration editor.
  * Drag-and-drop file processing.
  * Translation history with localStorage persistence.
  * RAG configuration panel (beta).
- **EPUB Parser**: Support for translating `.epub` electronic books.
  * Extracts XHTML content while preserving structure.
  * Re-packages translated content into valid EPUB files.
- **Ren'Py Parser** (.rpy): Visual novel script format.
  * Parsing of dialogue lines and menu choices.
  * Preservation of Python indentation and structure.
- **RAG Memory** (Beta): Semantic search for context-aware translation.
  * ONNX Runtime integration for local embedding generation.
  * Simple vector store with cosine similarity search.
  * Automatic context retrieval from past translations.
  * Optional feature, disabled by default.
- **ASS/SSA Parser** (.ass, .ssa): Advanced SubStation Alpha subtitle format.
  * Parses dialogue with timing, styles, and speaker information.
  * Strips override tags while preserving text.
- **WebVTT Parser** (.vtt): HTML5 video text tracks format.
  * Parses cue timing and settings.
  * Strips HTML-like formatting tags.
- **Testing**: Comprehensive integration test suite with `insta` snapshot testing.
- **CI/CD**: GitHub Actions workflow with Tauri build support.

### Changed
- Updated Rust edition to 2024 for all crates.
- Added `nom` 8.0 dependency for advanced parsing capabilities.
- Added `ort` (ONNX Runtime) and `tokenizers` for RAG.
- Updated README and documentation with GUI and RAG features.
- Extended supported formats to 8 types (txt, srt, json, ass, ssa, vtt, epub, rpy).

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
| 0.2.0 | 2026-03-23 | GUI application, RAG memory, EPUB support |
| 0.1.0 | 2026-03-23 | Initial release with CLI MVP |

---

[Unreleased]: https://github.com/xuepoo/kyogoku/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/xuepoo/kyogoku/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/xuepoo/kyogoku/releases/tag/v0.1.0
