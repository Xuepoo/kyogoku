# Kyogoku Architecture

Technical architecture documentation for core developers and contributors.

## Table of Contents

1. [System Overview](#system-overview)
2. [Module Architecture](#module-architecture)
3. [Data Models](#data-models)
4. [Translation Pipeline](#translation-pipeline)
5. [Core Algorithms](#core-algorithms)
6. [Extension Points](#extension-points)

---

## System Overview

Kyogoku is structured as a Cargo workspace with three crates following the **hexagonal architecture** pattern:

```
┌─────────────────────────────────────────────────────────────┐
│                      kyogoku-gui                            │
│                  (Graphical Interface Layer)                │
│  ┌──────────┐  ┌──────────┐  ┌───────────┐  ┌─────────┐     │
│  │ Frontend │  │ Commands │  │  Events   │  │  State  │     │
│  └──────────┘  └──────────┘  └───────────┘  └─────────┘     │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                      kyogoku-cli                            │
│                   (User Interface Layer)                    │
│  ┌─────────┐  ┌─────────┐  ┌───────────┐  ┌──────────┐    │
│  │  init   │  │ config  │  │ translate │  │  cache   │    │
│  └─────────┘  └─────────┘  └───────────┘  └──────────┘    │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      kyogoku-core                           │
│                   (Business Logic Layer)                    │
│  ┌──────────┐  ┌─────────┐  ┌─────────┐  ┌───────────┐    │
│  │  Engine  │  │   API   │  │  Cache  │  │ Glossary  │    │
│  └──────────┘  └─────────┘  └─────────┘  └───────────┘    │
│  ┌──────────┐                                              │
│  │  Config  │                                              │
│  └──────────┘                                              │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                     kyogoku-parser                          │
│                   (Data Transformation Layer)               │
│  ┌───────────────────┐  ┌─────────────────────────────┐    │
│  │ TranslationBlock  │  │      Parser Trait           │    │
│  │   (Core IR)       │  │  ┌─────┐ ┌─────┐ ┌──────┐  │    │
│  └───────────────────┘  │  │ TXT │ │ SRT │ │ JSON │  │    │
│                         │  └─────┘ └─────┘ └──────┘  │    │
│                         └─────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

### Design Principles

1. **Separation of Concerns**: CLI and GUI are distinct consumers of the Core library
2. **Dependency Inversion**: Core depends on abstractions (Parser trait), not concrete implementations
3. **Composition over Inheritance**: Engine composed of API, Cache, Glossary components
4. **Incremental Processing**: Content-addressed caching enables resume from any point

---

## Module Architecture

### kyogoku-gui

**Purpose**: Cross-platform graphical user interface using Tauri 2.0.

```
crates/kyogoku-gui/
├── src-tauri/
│   ├── src/
│   │   ├── lib.rs       # Tauri commands & event emission
│   │   └── main.rs      # Application entry point
│   └── Cargo.toml       # Tauri dependencies
├── src/
│   ├── main.ts          # Frontend logic (TypeScript)
│   ├── index.html       # UI layout (Tailwind CSS)
│   └── style.css        # Global styles
└── package.json         # Frontend dependencies
```

**Key Components:**
- **Commands**: `get_config`, `save_config`, `translate_file` - Rust functions callable from JS.
- **Events**: `translate-start`, `translate-progress`, `translate-complete` - Real-time updates.
- **State**: `Mutex<Config>` managed by Tauri's state system.

### kyogoku-parser

**Purpose**: Parse various file formats into a unified intermediate representation.

```
src/
├── lib.rs           # Public exports
├── block.rs         # TranslationBlock definition
├── parser.rs        # Parser trait + ParserRegistry
├── txt.rs           # Plain text parser
├── srt.rs           # SRT subtitle parser
└── json.rs          # JSON/MTool parser
```

**Key Types:**
- `TranslationBlock`: Unified translation unit with content-addressed ID
- `Parser`: Trait defining parse/serialize interface
- `ParserRegistry`: Factory for selecting parsers by file extension

### kyogoku-core

**Purpose**: Business logic including API calls, caching, and translation orchestration.

```
src/
├── lib.rs           # Public exports
├── config.rs        # Configuration system (XDG-compliant)
├── api.rs           # LLM API client (OpenAI-compatible)
├── cache.rs         # sled-based translation cache
├── glossary.rs      # Terminology management
└── engine.rs        # Translation engine orchestrator
```

**Key Types:**
- `Config`: Full configuration state
- `ApiClient`: Async HTTP client for LLM APIs
- `TranslationCache`: Content-addressed cache using sled
- `Glossary`: Term matching and formatting
- `TranslationEngine`: Pipeline orchestrator

### kyogoku-cli

**Purpose**: User interface via command-line.

```
src/
├── main.rs          # Entry point + clap definitions
└── commands/
    ├── mod.rs       # Command exports
    ├── init.rs      # `kyogoku init`
    ├── config.rs    # `kyogoku config`
    ├── translate.rs # `kyogoku translate`
    └── cache.rs     # `kyogoku cache`
```

---

## Data Models

### TranslationBlock

The core intermediate representation (IR) for all formats:

```rust
pub struct TranslationBlock {
    pub id: String,                    // Blake3 hash of source content
    pub speaker: Option<String>,       // Speaker/character identifier
    pub source: String,                // Original text
    pub target: Option<String>,        // Translation result
    pub metadata: serde_json::Value,   // Format-specific metadata
}
```

**ID Generation:**

```rust
impl TranslationBlock {
    pub fn new(source: String) -> Self {
        let id = blake3::hash(source.as_bytes()).to_hex().to_string();
        Self {
            id,
            speaker: None,
            source,
            target: None,
            metadata: serde_json::Value::Null,
        }
    }
}
```

**Metadata Examples:**

| Format | Metadata Fields |
|--------|-----------------|
| TXT | `{ "line": 42 }` |
| SRT | `{ "index": 1, "start": "00:00:01,000", "end": "00:00:04,000" }` |
| JSON | `{ "key": "0001", "format": "mtool" }` |

### Configuration

Hierarchical configuration structure:

```rust
pub struct Config {
    pub api: ApiConfig,           // API connection settings
    pub translation: TranslationConfig,  // Translation behavior
    pub advanced: AdvancedConfig, // Performance settings
    pub project: ProjectConfig,   // Per-project defaults
}
```

---

## Translation Pipeline

### High-Level Flow

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Read File  │────▶│    Parse    │────▶│   Blocks    │
└─────────────┘     └─────────────┘     └──────┬──────┘
                                               │
                                               ▼
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│ Write File  │◀────│  Serialize  │◀────│  Translate  │
└─────────────┘     └─────────────┘     └─────────────┘
```

### Detailed Flow

```
1. INGESTION
   ┌─────────────────────────────────────────────────────┐
   │ File → Parser.parse() → Vec<TranslationBlock>      │
   │                                                     │
   │ Input: "script.json"                                │
   │ Output: [Block{id: "abc...", source: "こんにちは"}] │
   └─────────────────────────────────────────────────────┘
                              │
                              ▼
2. HASHING (already done in parse)
   ┌─────────────────────────────────────────────────────┐
   │ Blake3::hash(source) → content-addressed ID        │
   │                                                     │
   │ "こんにちは" → "abc123def456..."                    │
   └─────────────────────────────────────────────────────┘
                              │
                              ▼
3. CACHE LOOKUP
   ┌─────────────────────────────────────────────────────┐
   │ cache.get(id) → Option<String>                     │
   │                                                     │
   │ Hit:  Return cached translation                     │
   │ Miss: Continue to LLM inference                     │
   └─────────────────────────────────────────────────────┘
                              │
                              ▼
4. CONTEXT ASSEMBLY
   ┌─────────────────────────────────────────────────────┐
   │ a. Glossary lookup: glossary.find_matches(source)  │
   │ b. Context window: last N (source, target) pairs   │
   │ c. Build prompt with instructions + context        │
   └─────────────────────────────────────────────────────┘
                              │
                              ▼
5. LLM INFERENCE
   ┌─────────────────────────────────────────────────────┐
   │ api.chat(prompt) → translation                     │
   │                                                     │
   │ Concurrent: Semaphore limits parallel requests     │
   │ Retry: Automatic retry on rate limits              │
   └─────────────────────────────────────────────────────┘
                              │
                              ▼
6. VALIDATION
   ┌─────────────────────────────────────────────────────┐
   │ - Check control characters preserved               │
   │ - Verify length within bounds                      │
   │ - (Future: format-specific validation)             │
   └─────────────────────────────────────────────────────┘
                              │
                              ▼
7. CACHING
   ┌─────────────────────────────────────────────────────┐
   │ cache.set(id, translation)                         │
   └─────────────────────────────────────────────────────┘
                              │
                              ▼
8. RE-SERIALIZATION
   ┌─────────────────────────────────────────────────────┐
   │ Parser.serialize(blocks, template) → output file   │
   └─────────────────────────────────────────────────────┘
```

---

## Core Algorithms

### Content-Addressed Caching

Uses Blake3 hash of source content as cache key:

```rust
// Generate ID
let id = blake3::hash(source.as_bytes()).to_hex().to_string();

// Cache operations
cache.get(&id)  // O(1) lookup
cache.set(&id, &translation)  // Persist to sled
```

**Benefits:**
- Automatic deduplication across files
- Resume support (already-translated blocks skipped)
- No manual cache invalidation needed

### Context Window

Sliding window of previous translations for consistency:

```rust
fn build_context(&self, context_window: &[(String, String)]) -> String {
    let mut context = String::new();
    for (source, target) in context_window.iter().rev().take(context_size) {
        context.push_str(&format!("{} → {}\n", source, target));
    }
    context
}
```

**Algorithm:**
1. Maintain FIFO queue of (source, target) pairs
2. Include last N pairs in prompt
3. Update queue after each translation

### Glossary Matching

Simple substring matching for term enforcement:

```rust
impl Glossary {
    pub fn find_matches(&self, text: &str) -> Vec<&GlossaryTerm> {
        self.terms
            .iter()
            .filter(|term| text.contains(&term.source))
            .collect()
    }
}
```

**Future Improvements:**
- Aho-Corasick for efficient multi-pattern matching
- Fuzzy matching for variants
- Priority/weighting for overlapping terms

### Concurrency Control

Semaphore-based rate limiting:

```rust
let semaphore = Arc::new(Semaphore::new(max_concurrency));

async fn translate_block(&self, block: &mut TranslationBlock) {
    let _permit = self.semaphore.acquire().await;
    // API call happens here
}
```

---

## Extension Points

### Adding a New Parser

1. Implement the `Parser` trait:

```rust
pub trait Parser {
    fn parse(&self, content: &str) -> Result<Vec<TranslationBlock>>;
    fn serialize(&self, blocks: &[TranslationBlock], template: &str) -> Result<String>;
}
```

2. Register in `ParserRegistry`:

```rust
registry.register("ext", Box::new(MyParser));
```

### Adding a New API Provider

1. Add variant to `ApiProvider` enum:

```rust
pub enum ApiProvider {
    OpenAI,
    DeepSeek,
    // ...
    NewProvider,
}
```

2. Add API base URL in `get_api_base()`:

```rust
ApiProvider::NewProvider => "https://api.newprovider.com/v1",
```

3. Add environment variable in `resolve_api_key()`:

```rust
ApiProvider::NewProvider => "NEWPROVIDER_API_KEY",
```

### Adding a New Translation Style

1. Add variant to `TranslationStyle`:

```rust
pub enum TranslationStyle {
    Literary,
    Casual,
    // ...
    Poetic,
}
```

2. Update prompt generation in engine to use new style.

---

## Future Architecture (Roadmap)

### Q4 2026: RAG Integration

```
kyogoku-core/
└── src/
    ├── rag/
    │   ├── mod.rs        # RAG module
    │   ├── embeddings.rs # Embedding generation
    │   ├── vectordb.rs   # Vector database (qdrant/milvus)
    │   └── retriever.rs  # Context retrieval
    └── ...
```

---

*Last updated: 2026-03-23*
