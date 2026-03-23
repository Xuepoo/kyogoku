# Kyogoku User Guide

This guide covers installation, configuration, and daily usage of Kyogoku for translating novels, game scripts, and subtitles.

## Table of Contents

1. [Installation](#installation)
2. [Authentication](#authentication)
3. [Basic Usage](#basic-usage)
4. [Advanced Usage](#advanced-usage)
5. [Supported Formats](#supported-formats)
6. [Troubleshooting](#troubleshooting)

---

## Installation

### Prerequisites

- Rust 1.85+ (2024 edition)
- A supported LLM API key (OpenAI, DeepSeek, Anthropic, Google, or local Ollama)

### From Source (Recommended)

```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable

# Clone and build
git clone https://github.com/xuepoo/kyogoku
cd kyogoku
cargo build --release

# Install to system PATH
sudo cp target/release/kyogoku /usr/local/bin/

# Verify installation
kyogoku --version
```

### Arch Linux

```bash
# Using paru/yay (once published to AUR)
paru -S kyogoku
```

---

## Authentication

Kyogoku supports multiple LLM providers. You need an API key from at least one provider.

### Supported Providers

| Provider | Environment Variable | API Base URL |
|----------|---------------------|--------------|
| OpenAI | `OPENAI_API_KEY` | `https://api.openai.com/v1` |
| DeepSeek | `DEEPSEEK_API_KEY` | `https://api.deepseek.com/v1` |
| Anthropic | `ANTHROPIC_API_KEY` | `https://api.anthropic.com/v1` |
| Google | `GOOGLE_API_KEY` | `https://generativelanguage.googleapis.com/v1beta` |
| Local (Ollama) | N/A | `http://localhost:11434/v1` |

### Method 1: Environment Variables (Recommended)

```bash
# Add to your shell profile (~/.bashrc, ~/.zshrc, etc.)
export DEEPSEEK_API_KEY="sk-your-key-here"

# Configure Kyogoku to use environment variable
kyogoku config set api.api_key ENV_VAR
kyogoku config set api.provider deepseek
```

### Method 2: Direct Configuration

```bash
# Store key directly in config (less secure)
kyogoku config set api.api_key "sk-your-key-here"
```

### Method 3: Edit Config File

Edit `~/.config/kyogoku/config.toml`:

```toml
[api]
provider = "deepseek"
api_key = "ENV_VAR"  # Or paste key directly
model = "deepseek-chat"
```

### Verify Connection

```bash
kyogoku config test
```

---

## Basic Usage

### Workflow Overview

1. **Prepare**: Organize source files in a directory
2. **Configure**: Set up API and translation preferences
3. **Translate**: Run translation command
4. **Review**: Check output in the output directory

### Translating a Single File

```bash
# Translate to ./output directory
kyogoku translate ./input/script.json

# Specify output directory
kyogoku translate ./input/script.json -o ./translated

# Specify languages explicitly
kyogoku translate ./novel.txt --from ja --to zh
```

### Translating a Directory

```bash
# Translate all supported files in a directory
kyogoku translate ./game_scripts/ -o ./game_scripts_zh/

# With verbose logging
kyogoku translate ./input/ -o ./output/ -v
```

### Checking Cache

Kyogoku caches translations to avoid redundant API calls:

```bash
# View cache statistics
kyogoku cache stats

# Clear all cached translations
kyogoku cache clear
```

---

## Advanced Usage

### Using a Glossary

Glossaries ensure consistent translation of character names, locations, and terminology.

**Create `glossary.json`:**

```json
{
  "terms": [
    {
      "source": "京极堂",
      "target": "Kyogokudo",
      "context": "Character name - the protagonist detective"
    },
    {
      "source": "魍魎",
      "target": "Mouryou",
      "context": "Supernatural creature, keep romanized"
    },
    {
      "source": "古書店",
      "target": "antiquarian bookshop",
      "context": "Setting location"
    }
  ]
}
```

**Use the glossary:**

```bash
kyogoku translate ./novel.txt --glossary ./glossary.json -o ./output
```

### Translation Styles

Configure the translation style for different content types:

| Style | Use Case | Description |
|-------|----------|-------------|
| `literary` | Novels, light novels | Prose-focused, maintains literary quality |
| `casual` | Game dialogue, chat | Natural, conversational tone |
| `formal` | Business, official docs | Polished, professional language |
| `technical` | Manuals, documentation | Precise, terminology-focused |

```bash
kyogoku config set translation.style literary
```

### Context Window

The context window includes previous translations for consistency:

```bash
# Set context window size (default: 5)
kyogoku config set translation.context_size 10
```

Larger context = better consistency, but higher token usage.

### Concurrency Control

Control the number of parallel API requests:

```bash
# Set max concurrent requests (default: 8)
kyogoku config set advanced.max_concurrency 4
```

Lower values reduce rate-limiting risk; higher values increase speed.

### Skip Cache

Force fresh translations, ignoring cache:

```bash
kyogoku translate ./input.json --no-cache
```

---

## Supported Formats

### Plain Text (.txt)

Simple line-by-line translation:

**Input:**
```
彼女は窓の外を見つめていた。
雨が降り始めた。
```

**Output:**
```
她凝视着窗外。
雨开始下了。
```

### SRT Subtitles (.srt)

Preserves timestamps and subtitle structure:

**Input:**
```srt
1
00:00:01,000 --> 00:00:04,000
おはようございます。

2
00:00:05,000 --> 00:00:08,000
今日はいい天気ですね。
```

**Output:**
```srt
1
00:00:01,000 --> 00:00:04,000
早上好。

2
00:00:05,000 --> 00:00:08,000
今天天气真好啊。
```

### JSON (MTool Format)

Supports nested objects and MTool-style dialogue:

**Input:**
```json
{
  "0001": "彼女は静かに微笑んだ。",
  "0002": "「ありがとう」と彼女は言った。"
}
```

**Output:**
```json
{
  "0001": "她静静地微笑着。",
  "0002": ""谢谢你，"她说。"
}
```

---

## Troubleshooting

### Common Errors

#### "API key not found"

```
Error: API key not configured
```

**Solution:** Set your API key:
```bash
export DEEPSEEK_API_KEY="your-key"
kyogoku config set api.api_key ENV_VAR
```

#### "Rate limit exceeded"

```
Error: 429 Too Many Requests
```

**Solution:** Reduce concurrency:
```bash
kyogoku config set advanced.max_concurrency 2
```

#### "Failed to parse file"

```
Error: Failed to parse input file
```

**Solution:** Check file format and encoding (UTF-8 required):
```bash
file -i ./input.json  # Check encoding
iconv -f SHIFT_JIS -t UTF-8 input.txt > input_utf8.txt
```

### Network Issues

#### Timeout errors

For slow connections or large files:

```bash
# The engine has built-in retry logic
# For persistent issues, check your network connection
curl -I https://api.deepseek.com
```

#### Proxy configuration

If behind a proxy:

```bash
export HTTPS_PROXY="http://proxy:8080"
kyogoku translate ./input.json
```

### Cache Issues

If translations seem stale:

```bash
# Clear and rebuild cache
kyogoku cache clear
kyogoku translate ./input.json
```

### Debug Mode

Enable verbose logging for troubleshooting:

```bash
kyogoku translate ./input.json -v
```

---

## Getting Help

- **GitHub Issues**: [Report bugs or request features](https://github.com/xuepoo/kyogoku/issues)
- **Configuration Reference**: See [CONFIG.md](CONFIG.md) for all options
- **Developer Guide**: See [DEVELOPER.md](DEVELOPER.md) for contributing

---

*Last updated: 2026-03-23*
