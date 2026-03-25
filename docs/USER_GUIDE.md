# Kyogoku User Guide

Complete guide for using Kyogoku to translate literature, visual novels, and game scripts.

## Table of Contents

- [Installation](#installation)
- [Getting Started](#getting-started)
- [Configuration](#configuration)
- [Translating Files](#translating-files)
- [GUI Application](#gui-application)
- [Glossary System](#glossary-system)
- [Troubleshooting](#troubleshooting)

## Installation

### From Release (Recommended)

Download pre-built binaries from [GitHub Releases](https://github.com/xuepoo/kyogoku/releases):

- **Linux**: `kyogoku-linux-x86_64.tar.gz`
- **macOS**: `kyogoku-macos-universal.dmg`
- **Windows**: `kyogoku-windows-x64.msi`

### From Source

```bash
# Install Rust if not already installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/xuepoo/kyogoku
cd kyogoku
cargo build --release --bin kyogoku

# Install to PATH (Linux/macOS)
sudo cp target/release/kyogoku /usr/local/bin/

# Or add to PATH manually
export PATH="$PATH:$(pwd)/target/release"
```

## Getting Started

### 1. Initialize Configuration

```bash
# Create default configuration file
kyogoku init

# Configuration is created at:
# Linux/macOS: ~/.config/kyogoku/config.toml
# Windows: %APPDATA%\kyogoku\config.toml
```

### 2. Set Up API Access

Kyogoku supports multiple LLM providers:

#### Option A: OpenAI

```bash
export OPENAI_API_KEY="sk-..."
kyogoku config set api.provider openai
kyogoku config set api.api_key ENV_VAR
kyogoku config set api.model gpt-4o
```

#### Option B: DeepSeek

```bash
export DEEPSEEK_API_KEY="sk-..."
kyogoku config set api.provider deepseek
kyogoku config set api.api_key ENV_VAR
kyogoku config set api.model deepseek-chat
```

#### Option C: Anthropic (Claude)

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
kyogoku config set api.provider anthropic
kyogoku config set api.api_key ENV_VAR
kyogoku config set api.model claude-sonnet-4
```

#### Option D: Local LLM (Ollama)

```bash
# Start Ollama server first: ollama serve
kyogoku config set api.provider local
kyogoku config set api.model llama3.1:8b
kyogoku config set api.api_base http://localhost:11434/v1
```

### 3. Test Configuration

```bash
# Verify API connectivity and authentication
kyogoku config test
```

## Configuration

### View Current Config

```bash
kyogoku config show
```

### Modify Settings

```bash
# API settings
kyogoku config set api.provider openai
kyogoku config set api.model gpt-4o-mini
kyogoku config set api.max_tokens 4096
kyogoku config set api.temperature 0.3

# Project settings
kyogoku config set project.source_lang ja
kyogoku config set project.target_lang en
kyogoku config set project.output_dir ./translated

# Translation style
kyogoku config set translation.style literary  # or: casual, formal, technical
kyogoku config set translation.context_size 5

# Advanced settings
kyogoku config set advanced.max_concurrency 8  # Parallel requests
kyogoku config set advanced.batch_size 5       # Blocks per batch
```

### Config File Format

`~/.config/kyogoku/config.toml`:

```toml
[api]
provider = "deepseek"
api_key = "ENV_VAR"
model = "deepseek-chat"
max_tokens = 4096
temperature = 0.3

[project]
source_lang = "ja"
target_lang = "en"
output_dir = "./output"

[translation]
style = "literary"
context_size = 5

[advanced]
max_concurrency = 8
batch_size = 5
```

## Translating Files

### Basic Translation

```bash
# Single file
kyogoku translate novel.txt

# Directory (recursive)
kyogoku translate ./input_folder

# Custom output directory
kyogoku translate input.json -o ./translated
```

### Supported Formats

| Format | Extension | Notes |
|--------|-----------|-------|
| Plain Text | `.txt` | Line-by-line |
| SRT Subtitles | `.srt` | Preserves timestamps |
| ASS/SSA Subtitles | `.ass`, `.ssa` | Preserves styling tags |
| WebVTT | `.vtt` | HTML5 video subtitles |
| EPUB | `.epub` | E-book format |
| Markdown | `.md` | Preserves formatting |
| JSON | `.json` | MTool game format |
| Ren'Py | `.rpy` | Visual novel scripts |

### Advanced Options

#### Preview Before Translating

```bash
# Dry run - show blocks without API calls
kyogoku translate novel.txt --dry-run
```

#### Custom Language Pair

```bash
# Override config languages
kyogoku translate script.rpy --from ja --to zh
```

#### Skip Cache

```bash
# Force fresh translation (ignore cache)
kyogoku translate input.txt --no-cache
```

#### Use Glossary

```bash
# Apply custom terminology
kyogoku translate game.json --glossary ./character_names.json
```

#### JSON Output

```bash
# Machine-readable results
kyogoku translate input.txt --json > result.json
```

### Debug Mode

```bash
# Verbose logging (info level)
kyogoku -v translate input.txt

# Debug logging (with tracing spans)
kyogoku -d translate input.txt

# Quiet mode (errors only)
kyogoku -q translate input.txt
```

## GUI Application

### Launching the GUI

```bash
cd crates/kyogoku-gui
pnpm install
pnpm tauri dev
```

### GUI Features

- **Drag & Drop**: Add files by dragging into the app
- **Batch Processing**: Queue multiple files for translation
- **Real-time Preview**: Watch translations appear live
- **Cost Estimation**: Estimate API costs before translating
- **Virtual Scrolling**: Handle large documents efficiently
- **Theme System**: Light/dark mode support
- **Keyboard Shortcuts**:
  - `Ctrl+O`: Add files
  - `Ctrl+S`: Save configuration
  - `Ctrl+Q`: Clear queue

## Glossary System

### Creating a Glossary

Create a JSON file with custom terminology:

```json
{
  "entries": [
    {
      "source": "桜",
      "target": "Sakura",
      "context": "Character name"
    },
    {
      "source": "異世界",
      "target": "isekai",
      "context": "Keep as romanized term"
    },
    {
      "source": "魔法",
      "target": "magic",
      "context": ""
    }
  ]
}
```

### Using a Glossary

```bash
# CLI
kyogoku translate input.txt --glossary ./glossary.json

# Config file
kyogoku config set project.glossary_path ./glossary.json
```

### Glossary Benefits

- **Consistent Terminology**: Same source always maps to same target
- **Character Names**: Preserve romanization choices
- **Domain Terms**: Technical or setting-specific vocabulary
- **Context Hints**: Optional notes for the translator

## Troubleshooting

### API Errors

#### Authentication Failed (401)

```
Error: Authentication Failed: Check your API key in Settings
```

**Solution**: Verify your API key is correct and not expired.

```bash
# Check current key
kyogoku config show

# Update key
export OPENAI_API_KEY="sk-..."
kyogoku config set api.api_key ENV_VAR
```

#### Rate Limited (429)

```
Error: Rate Limited: Reduce batch size or wait
```

**Solution**: Reduce concurrency or wait before retrying.

```bash
kyogoku config set advanced.max_concurrency 2
kyogoku config set advanced.batch_size 3
```

#### Token Limit Exceeded

```
Error: Token Limit Exceeded: Your text is too long
```

**Solution**: Split large files into smaller chunks or reduce max_tokens.

```bash
kyogoku config set api.max_tokens 2048
```

### Cache Issues

#### Corrupted Cache

```
⚠️  Warning: 5 corrupted entries detected
```

**Solution**: Clear and rebuild cache.

```bash
kyogoku cache clear
```

#### Check Cache Health

```bash
kyogoku cache stats
```

Output:
```
Cache Statistics:
  Status:  Healthy
  Entries: 1234
  Size:    45.2 MB
  Path:    /home/user/.local/share/kyogoku/cache
```

### Network Errors

#### Connection Timeout

**Solution**: Check internet connection, increase timeout, or use local LLM.

#### Firewall/Proxy

If behind a corporate firewall:

```bash
export HTTP_PROXY=http://proxy.example.com:8080
export HTTPS_PROXY=http://proxy.example.com:8080
```

### Format Issues

#### Unsupported File Format

```
Error: No supported files found
```

**Solution**: Check file extension or use `--format` to force parser.

```bash
kyogoku translate input.txt --format txt
```

#### Malformed Input

```
Error: Failed to parse input file
```

**Solution**: Validate file format. For JSON, use a JSON validator. For EPUB, check with `epubcheck`.

### Getting Help

```bash
# General help
kyogoku --help

# Command-specific help
kyogoku translate --help
kyogoku config --help
```

For bugs and feature requests: https://github.com/xuepoo/kyogoku/issues
