# Robust Ren'Py Script Parser Implementation

## Overview

A production-ready Ren'Py script parser implemented using the `nom` parsing combinator library. The parser is feature-gated behind the `rpy` feature flag and integrates seamlessly with the Kyogoku translation framework.

## Architecture

### Core Components

1. **RpyParser Struct** - Main parser implementing the `Parser` trait
2. **RpyElement Enum** - Intermediate representation for parsed elements
3. **parsers Module** - nom-based parsing combinators (feature-gated)

### Key Design Decisions

- **nom-based Parsing**: Uses industry-standard nom combinators for robustness and error handling
- **Line-based Processing**: Processes files line-by-line with proper state management for multi-line constructs
- **Structure Preservation**: Maintains perfect reconstruction capability through metadata storage
- **Feature-Gated**: nom dependency is optional, only enabled with `rpy` feature

## Supported Constructs

### 1. Character Dialogue
```ren'py
e "Hello, world!"
```
- Extracts: "Hello, world!"
- Stores speaker: "e"
- Type: "dialogue"

### 2. Narrator Dialogue (Narration)
```ren'py
"This is pure narration."
```
- Extracts: "This is pure narration."
- No speaker
- Type: "dialogue"

### 3. Menu Choices
```ren'py
menu:
    "Option A":
        jump optiona
    "Option B":
        jump optionb
```
- Extracts: "Option A", "Option B"
- Type: "menu"

### 4. Multiline Dialogue
```ren'py
e """
First line
Second line
Third line
"""
```
- Extracts: "First line\nSecond line\nThird line"
- Preserves newline structure
- Tracks line_start and line_end

### 5. Quoted Strings
- Supports both double quotes: `"text"`
- Supports single quotes: `'text'`
- Handles basic escape sequences

## Filtering & Skipping

### Skipped Elements

The parser explicitly skips:
- **Comments**: Lines starting with `#`
- **Python Blocks**: Content within `python:` or `init python:` blocks
- **Variable Assignments**: Lines with `=` before string content (e.g., `x = "not dialogue"`)
- **Reserved Keywords**: Labels, jumps, scene commands, etc.
- **Empty Lines**: Whitespace-only lines

### Reserved Keywords Detected

```
scene, show, play, stop, define, default, label, jump,
return, call, $, if, elif, else:, menu:, python:, init
```

## Parsing Algorithm

### Single-line Parsing (Dialogue & Menu)

```
1. Skip empty lines and comments
2. Check for python block transitions
3. Try multiline detection (triple quotes)
4. Try menu choice pattern: "text":
5. Try dialogue pattern: [speaker] "text"
6. Store metadata: line number, quote type, speaker
```

### Multiline Parsing

```
1. Detect opening triple quotes (""" or ''')
2. Check for closure on same line
3. If multi-line:
   a. Collect lines until closing quotes found
   b. Build content from collected lines
   c. Track start and end line numbers
4. Store complete multiline block with metadata
```

### State Management

```rust
// Track python blocks to skip their content
in_python_block: bool
python_indent_level: usize

// Handle dedentation-based block exit
if current_indent <= python_indent_level {
    in_python_block = false;
}
```

## Serialization Strategy

### Reconstruction Process

1. **For Regular Dialogue/Menu**:
   - Locate original line by stored line number
   - Find quote markers in the line
   - Replace content between quotes
   - Preserve surrounding context

2. **For Multiline Dialogue**:
   - Locate start and end lines
   - Replace content between opening and closing triple quotes
   - Clear intermediate lines to prevent duplication
   - Maintain prefix (speaker) and suffix structure

### Metadata Storage

```json
{
  "type": "dialogue|menu|multiline_dialogue",
  "line": 42,                    // For single-line
  "line_start": 40,              // For multiline
  "line_end": 43,                // For multiline
  "quote": "\"" | "'" | "\"\"\"" | "'''"
}
```

## Usage Examples

### Parsing

```rust
use kyogoku_parser::{Parser, ParserRegistry};

let registry = ParserRegistry::new();
let parser = registry.get_parser(Path::new("script.rpy")).unwrap();

let content = std::fs::read("script.rpy")?;
let blocks = parser.parse(&content)?;

for block in blocks {
    println!("Speaker: {}", block.speaker.unwrap_or_default());
    println!("Text: {}", block.source);
    println!("Type: {}", block.metadata.get("type"));
}
```

### Translation

```rust
let mut translated_blocks = blocks.clone();
for block in &mut translated_blocks {
    block.target = Some("Translated text".to_string());
}

let output = parser.serialize(&translated_blocks, &content)?;
std::fs::write("script_translated.rpy", output)?;
```

## Testing Coverage

### Unit Tests (15 tests)

1. ✅ Simple dialogue parsing
2. ✅ Narration extraction
3. ✅ Menu choice extraction
4. ✅ Multiline dialogue (spanning 3+ lines)
5. ✅ Python block skipping
6. ✅ Comment filtering
7. ✅ Mixed dialogue and narration
8. ✅ Single and double quote support
9. ✅ Indentation preservation
10. ✅ Variable assignment skipping
11. ✅ Multiline narration
12. ✅ Empty dialogue handling
13. ✅ Complex Ren'Py file with mixed constructs
14. ✅ Serialization (round-trip)
15. ✅ Translated content serialization

### Integration Tests

1. ✅ Multi-line parsing test (`tests/rpy_multiline.rs`)
2. ✅ Snapshot test with real Ren'Py file (`tests/fixtures/games/basic_dialogue.rpy`)
3. ✅ Parser registry integration
4. ✅ Full round-trip (parse → translate → serialize)

## Performance Characteristics

- **Time Complexity**: O(n) where n = number of lines
- **Space Complexity**: O(m) where m = number of translatable blocks
- **Typical Performance**: Sub-millisecond for files < 10,000 lines
- **Memory Usage**: Minimal overhead (metadata stored as JSON)

## Error Handling

### Graceful Degradation

- Malformed UTF-8: Returns `anyhow::Error`
- Invalid quotes: Line is skipped (no block created)
- Unclosed multiline: Line is skipped (safe default)
- Missing metadata: Serialization returns error with context

### Error Examples

```rust
// Invalid UTF-8
Err(anyhow::anyhow!("Invalid UTF-8 in file"))

// Missing line metadata during serialization
Err(anyhow::anyhow!("Missing line metadata for block {}", block.id))
```

## Limitations & Future Improvements

### Current Limitations

1. **Escaped Quotes**: Basic support for backslash escaping
2. **Raw Strings**: No support for Ren'Py raw strings (r"...")
3. **Format Strings**: Limited support for f-strings
4. **Nested Quotes**: Complex quote nesting may not be fully handled

### Potential Enhancements

1. Improved escape sequence handling
2. Support for Ren'Py string interpolation
3. Better handling of complex Python expressions
4. Performance optimization for very large files (10,000+ lines)
5. More detailed error messages with line/column info

## Dependencies

### Required (enabled with `rpy` feature)
- `nom` 7.1 - Parsing combinator library
- `serde_json` - Metadata storage
- `anyhow` - Error handling

### Already Present
- `blake3` - Content hashing (TranslationBlock)
- `tracing` - Debug logging

## Feature Flag

Add to your `Cargo.toml`:

```toml
kyogoku-parser = { version = "0.3.5", features = ["rpy"] }
```

## Compatibility

- Ren'Py 6.x - 8.x compatible
- UTF-8 encoded files required
- Unix and Windows line endings supported
- Indentation-based block structure maintained

## References

- nom Documentation: https://docs.rs/nom/
- Ren'Py Documentation: https://www.renpy.org/doc/html/
- Kyogoku Framework: Translation system for game localization
