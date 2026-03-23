# Kyogoku Configuration Reference

Complete reference for all configuration options in `config.toml`.

**Location:** `~/.config/kyogoku/config.toml` (XDG compliant)

---

## Quick Start

Generate default configuration:

```bash
kyogoku init
```

View current configuration:

```bash
kyogoku config show
```

Set individual values:

```bash
kyogoku config set api.provider deepseek
kyogoku config set translation.style literary
```

---

## Full Configuration Example

```toml
[api]
provider = "deepseek"
api_key = "ENV_VAR"
model = "deepseek-chat"
max_tokens = 4096
temperature = 0.3

[translation]
style = "literary"
context_size = 5

[advanced]
max_concurrency = 8
allocator = "mimalloc"

[project]
source_lang = "ja"
target_lang = "zh"
glossary_path = "./glossary.json"
input_dir = "./input"
output_dir = "./output"
```

---

## [api] Section

Configuration for the LLM API connection.

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `provider` | String | `"openai"` | LLM provider. See [Providers](#providers) below. |
| `api_key` | String | `null` | API key. Use `"ENV_VAR"` to load from environment. |
| `api_base` | String | Provider default | Custom API endpoint URL. Overrides provider default. |
| `model` | String | `"gpt-4o"` | Model identifier to use for translation. |
| `max_tokens` | Integer | `4096` | Maximum tokens in API response. |
| `temperature` | Float | `0.3` | Sampling temperature (0.0-2.0). Lower = more deterministic. |

### Providers

| Value | API Base (default) | Environment Variable |
|-------|-------------------|---------------------|
| `openai` | `https://api.openai.com/v1` | `OPENAI_API_KEY` |
| `deepseek` | `https://api.deepseek.com/v1` | `DEEPSEEK_API_KEY` |
| `anthropic` | `https://api.anthropic.com/v1` | `ANTHROPIC_API_KEY` |
| `google` | `https://generativelanguage.googleapis.com/v1beta` | `GOOGLE_API_KEY` |
| `local` | `http://localhost:11434/v1` | N/A |
| `custom` | `http://localhost:8080/v1` | `API_KEY` |

### Recommended Models

| Provider | Recommended Model | Notes |
|----------|------------------|-------|
| OpenAI | `gpt-4o` | Best quality, higher cost |
| DeepSeek | `deepseek-chat` | Good quality, low cost |
| Anthropic | `claude-3-5-sonnet-20241022` | Excellent for literary text |
| Google | `gemini-1.5-pro` | Good multilingual support |
| Local | `qwen2.5:14b` | Via Ollama, free |

### API Key Configuration

**Method 1: Environment Variable (Recommended)**

```bash
export DEEPSEEK_API_KEY="sk-your-key-here"
```

```toml
[api]
api_key = "ENV_VAR"  # Loads from environment
```

**Method 2: Direct Value (Less Secure)**

```toml
[api]
api_key = "sk-your-key-here"
```

### Custom API Endpoint

For OpenAI-compatible endpoints (e.g., Azure, vLLM):

```toml
[api]
provider = "custom"
api_base = "https://your-endpoint.com/v1"
api_key = "your-key"
model = "your-model-name"
```

---

## [translation] Section

Translation behavior and quality settings.

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `style` | String | `"literary"` | Translation style preset. See [Styles](#styles) below. |
| `context_size` | Integer | `5` | Number of previous translations to include as context. |

### Styles

| Value | Description | Best For |
|-------|-------------|----------|
| `literary` | Prose-focused, maintains literary quality and flow | Novels, light novels, web fiction |
| `casual` | Natural, conversational tone with appropriate slang | Game dialogue, visual novels, chat |
| `formal` | Professional, polished language | Official documents, business text |
| `technical` | Precise terminology, minimal stylistic changes | Manuals, documentation, UI strings |

### Context Window

The context window passes previous translations to the LLM for consistency:

```toml
[translation]
context_size = 10  # Include last 10 translations
```

**Trade-offs:**
- Larger context = Better consistency, higher token usage
- Smaller context = Lower cost, potentially inconsistent names/terms
- Recommended: 5-10 for novels, 3-5 for standalone dialogue

---

## [advanced] Section

Performance and system settings.

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `max_concurrency` | Integer | `8` | Maximum parallel API requests. |
| `allocator` | String | `null` | Memory allocator override (`"mimalloc"` for production). |

### Concurrency

Controls how many translations run in parallel:

```toml
[advanced]
max_concurrency = 4  # More conservative
```

**Guidelines:**
- `2-4`: Safe for rate-limited APIs
- `8`: Default, good balance
- `16+`: Only for high-rate-limit accounts or local models

### Memory Allocator

For production builds with high throughput:

```toml
[advanced]
allocator = "mimalloc"
```

*Note: Requires compiling with mimalloc feature enabled.*

---

## [project] Section

Per-project translation settings.

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `source_lang` | String | `"ja"` | Source language code (ISO 639-1). |
| `target_lang` | String | `"zh"` | Target language code (ISO 639-1). |
| `glossary_path` | Path | `null` | Default glossary file path. |
| `input_dir` | Path | `null` | Default input directory. |
| `output_dir` | Path | `null` | Default output directory. |

### Language Codes

Common language codes:

| Code | Language |
|------|----------|
| `ja` | Japanese |
| `zh` | Chinese (Simplified) |
| `zh-TW` | Chinese (Traditional) |
| `en` | English |
| `ko` | Korean |
| `de` | German |
| `fr` | French |

### Project Defaults

Set defaults to avoid repeating CLI arguments:

```toml
[project]
source_lang = "ja"
target_lang = "zh"
input_dir = "./raw"
output_dir = "./translated"
glossary_path = "./glossary.json"
```

Then simply run:

```bash
kyogoku translate  # Uses project defaults
```

---

## [rag] Section (Beta)

RAG (Retrieval-Augmented Generation) enables semantic search of past translations for improved consistency.

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `enabled` | Boolean | `false` | Enable RAG memory feature. |
| `model_path` | Path | `null` | Path to ONNX embedding model file. |
| `tokenizer_path` | Path | `null` | Path to tokenizer.json file. |
| `vector_store_path` | Path | `null` | Path to store/load vector embeddings. |

### Setup

RAG requires downloading an ONNX embedding model:

```bash
# Download model (e.g., all-MiniLM-L6-v2, ~86MB)
mkdir -p ~/.local/share/kyogoku/models
cd ~/.local/share/kyogoku/models
curl -L -o model.onnx https://huggingface.co/Xenova/all-MiniLM-L6-v2/resolve/main/onnx/model.onnx
curl -L -o tokenizer.json https://huggingface.co/Xenova/all-MiniLM-L6-v2/resolve/main/tokenizer.json
```

### Configuration

```toml
[rag]
enabled = true
model_path = "~/.local/share/kyogoku/models/model.onnx"
tokenizer_path = "~/.local/share/kyogoku/models/tokenizer.json"
vector_store_path = "~/.local/share/kyogoku/vectors.bin"
```

### How It Works

1. **Embedding**: Each translated text is converted to a vector embedding
2. **Storage**: Embeddings are stored in a simple vector database
3. **Retrieval**: Before translating new text, similar past translations are retrieved
4. **Context**: Retrieved translations are included in the LLM prompt for consistency

### Recommended Models

| Model | Size | Quality | Speed |
|-------|------|---------|-------|
| `all-MiniLM-L6-v2` | 86MB | Good | Fast |
| `all-mpnet-base-v2` | 420MB | Better | Medium |
| `multilingual-e5-base` | 1.1GB | Best for CJK | Slower |

*Note: RAG is optional and disabled by default. It increases memory usage but improves translation consistency for large projects.*

---

## File Paths

### XDG Base Directory Specification

Kyogoku follows the XDG spec:

| Path | Default Location | Contents |
|------|-----------------|----------|
| Config | `~/.config/kyogoku/` | `config.toml` |
| Data | `~/.local/share/kyogoku/` | Cache database |

### Override with Environment

```bash
export XDG_CONFIG_HOME="/custom/config"
export XDG_DATA_HOME="/custom/data"
```

### File Locations

| File | Path | Description |
|------|------|-------------|
| Config | `~/.config/kyogoku/config.toml` | Main configuration |
| Cache | `~/.local/share/kyogoku/cache/` | sled KV database |
| Glossary | User-defined | JSON glossary file |

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `OPENAI_API_KEY` | OpenAI API key (when `api_key = "ENV_VAR"`) |
| `DEEPSEEK_API_KEY` | DeepSeek API key |
| `ANTHROPIC_API_KEY` | Anthropic API key |
| `GOOGLE_API_KEY` | Google API key |
| `API_KEY` | Generic key for custom providers |
| `XDG_CONFIG_HOME` | Override config directory |
| `XDG_DATA_HOME` | Override data directory |
| `HTTPS_PROXY` | HTTP proxy for API requests |

---

## Configuration Validation

Test your configuration:

```bash
# Show current config
kyogoku config show

# Test API connection
kyogoku config test
```

### Common Issues

**Invalid provider:**
```
Error: unknown variant `chatgpt`
```
Use valid provider names: `openai`, `deepseek`, `anthropic`, `google`, `local`, `custom`

**Invalid style:**
```
Error: unknown variant `novel`
```
Use valid styles: `literary`, `casual`, `formal`, `technical`

**Missing API key:**
```
Error: API key not configured
```
Set `api.api_key` or export the appropriate environment variable.

---

*Last updated: 2026-03-23*
