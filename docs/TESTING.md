# Testing Guide (测试指南)

## Overview

Kyogoku uses a multi-tier testing strategy to ensure code quality and prevent regressions.

```
📁 tests/
├── Unit Tests        # In src/**/*.rs with #[cfg(test)]
├── Integration Tests # In tests/*.rs
└── Fixtures          # In tests/fixtures/
```

## Test Structure

### 1. Unit Tests (单元测试)

Located in source files under `#[cfg(test)]` modules. Test individual functions and components.

**Location**: `crates/*/src/**/*.rs`

**Example**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_timestamp() {
        let ts = SrtTimestamp::parse("00:01:23,456").unwrap();
        assert_eq!(ts.to_milliseconds(), 83456);
    }
}
```

**Run**:
```bash
cargo test --package kyogoku-parser
```

### 2. Integration Tests (集成测试)

Located in `tests/` directory. Test complete workflows using public APIs.

**Location**: `crates/*/tests/*.rs`

**Features**:
- Use `include_str!()` to embed test fixtures at compile time
- Test parser roundtrips (parse → serialize → parse)
- Verify error handling with malformed inputs
- Snapshot testing with `insta` crate

**Example**:
```rust
#[test]
fn test_srt_standard() {
    let content = include_str!("fixtures/subtitles/standard.srt");
    let parser = registry.get_parser(Path::new("test.srt")).unwrap();
    
    let blocks = parser.parse(content).expect("Failed to parse");
    let output = parser.serialize(&blocks, content).expect("Failed to serialize");
    
    assert!(output.contains("00:00:01,000"));
}
```

**Run**:
```bash
cargo test --test integration_test
```

### 3. Snapshot Tests (快照测试)

Use `insta` crate to capture complex struct outputs as snapshots.

**Workflow**:
1. Write test with `insta::assert_debug_snapshot!()`
2. Run test → generates `.snap.new` file
3. Review with `cargo insta review` or accept with `cargo insta accept`
4. Snapshots are committed to version control
5. Future runs compare against committed snapshots

**Example**:
```rust
#[test]
fn test_json_snapshot() {
    let blocks = parser.parse(content).unwrap();
    insta::assert_debug_snapshot!(blocks);  // Auto-generates snapshot
}
```

**Commands**:
```bash
cargo test                    # Run tests (fails on snapshot mismatch)
cargo insta review            # Interactive snapshot review
cargo insta accept            # Accept all pending snapshots
cargo insta test              # Run all snapshot tests
```

## Test Fixtures (测试夹具)

### Directory Structure

```
tests/fixtures/
├── subtitles/
│   ├── standard.srt           # Basic SRT subtitle
│   └── effect_tags.ass        # ASS with styles, tags, effects
├── games/
│   └── mtool_export.json      # MTool translation format
└── safety/
    ├── empty_file.txt         # Edge case: empty file
    ├── malformed_json.json    # Error case: invalid syntax
    └── utf8_bom.txt           # Edge case: UTF-8 BOM header
```

### Fixture Guidelines

1. **Small and Focused**: Each fixture tests one specific scenario
2. **Real-World Examples**: Use actual formats from target applications
3. **Edge Cases**: Include boundary conditions, malformed inputs, unusual encodings
4. **Embedded at Compile Time**: Use `include_str!()` for zero runtime dependencies

### Adding New Fixtures

```bash
# Download real sample
curl -o tests/fixtures/games/sample.rpy https://example.com/sample.rpy

# Or create manually
cat > tests/fixtures/safety/special_chars.txt << 'EOF'
Special characters: «», ‹›, —, …
EOF

# Add test
#[test]
fn test_special_chars() {
    let content = include_str!("fixtures/safety/special_chars.txt");
    // ...
}
```

## Test Commands

```bash
# Run all tests in workspace
cargo test --workspace

# Run specific crate tests
cargo test --package kyogoku-parser

# Run specific test
cargo test test_srt_standard

# Run with output (--nocapture)
cargo test -- --nocapture

# Run tests matching pattern
cargo test json

# List all tests
cargo test -- --list
```

## Coverage (覆盖率)

### Current Test Count

- **Total**: 45 tests
  - kyogoku-parser unit tests: 23
  - kyogoku-parser integration tests: 14
  - kyogoku-core tests: 8

### Coverage Strategy

We aim for:
- ✅ **100%** coverage on parsers (critical path)
- ✅ **Edge cases**: empty, malformed, encoding issues
- 🚧 **API mocking**: kyogoku-core LLM calls (TODO: use `wiremock`)
- 🚧 **End-to-end**: CLI tests (TODO: integration tests)

## Continuous Integration

GitHub Actions will run:
```yaml
- cargo test --workspace --all-targets
- cargo clippy --workspace --all-targets -- -D warnings
- cargo fmt --check
```

## Best Practices

### DO ✅

- Write tests before fixing bugs (TDD for bug fixes)
- Use `include_str!()` for test data embedding
- Test both happy path and error cases
- Use snapshot tests for complex structs
- Keep fixtures small (<100 lines)
- Test parser roundtrips (parse → serialize → parse)

### DON'T ❌

- Don't commit `.snap.new` files (always review/accept first)
- Don't use large fixtures (>1MB)
- Don't test private implementation details (test behavior)
- Don't skip error case testing
- Don't use real API keys in tests

## Debugging Tests

```bash
# Show test output
cargo test -- --nocapture

# Run single test with backtrace
RUST_BACKTRACE=1 cargo test test_name

# Run with debug logging
RUST_LOG=debug cargo test

# Show which tests are running
cargo test -- --show-output
```

## Tools

| Tool | Purpose |
|------|---------|
| `cargo test` | Built-in test runner |
| `insta` | Snapshot testing |
| `tempfile` | Temporary files in tests |
| `wiremock` | HTTP mock server (future) |
| `cargo-insta` | CLI for snapshot management |

## Future Enhancements

- [ ] Add `wiremock` for API mocking in kyogoku-core
- [ ] Add CLI integration tests with temp directories
- [ ] Add property-based testing with `proptest` or `quickcheck`
- [ ] Add mutation testing with `cargo-mutants`
- [ ] Set up code coverage reporting with `tarpaulin`
