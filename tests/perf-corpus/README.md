# Performance Test Corpus

Test files for validating translation quality and performance across different file formats and sizes.

## Structure

```
perf-corpus/
├── short/     # ~8-10 dialogue blocks each
├── medium/    # ~40-55 dialogue blocks each  
└── long/      # ~200-220 dialogue blocks each
```

## Formats Tested

| Format | Extension | Short | Medium | Long |
|--------|-----------|-------|--------|------|
| Plain Text | .txt | ✓ | ✓ | ✓ |
| SRT Subtitles | .srt | ✓ | ✓ | ✓ |
| ASS Subtitles | .ass | ✓ | ✓ | ✓ |
| WebVTT | .vtt | ✓ | ✓ | ✓ |
| JSON | .json | ✓ | ✓ | ✓ |
| Ren'Py | .rpy | ✓ | ✓ | ✓ |

## Running Performance Tests

Use the test runner script:

```bash
# Test all sizes with default model (google/gemini-2.5-flash)
python3 tests/perf_test.py

# Test specific size
python3 tests/perf_test.py --size short
python3 tests/perf_test.py --size medium
python3 tests/perf_test.py --size long

# Use different model
python3 tests/perf_test.py --model gpt-4o-mini

# Custom paths
python3 tests/perf_test.py \
  --corpus-dir ./custom-corpus \
  --output-dir ./custom-output \
  --csv ./custom-results.csv
```

## Results

Performance metrics are logged to `tests/perf-results.csv` with:
- File info (name, format, size, dialogue block count)
- Model used
- Timing (elapsed seconds)
- Token counts (input, output, total)
- Cost in USD
- Memory usage (before, after, delta)

## Expected Performance

With `google/gemini-2.5-flash` on typical hardware:
- **Short files** (~10 blocks): < 5 seconds, < $0.001
- **Medium files** (~50 blocks): < 20 seconds, < $0.005
- **Long files** (~220 blocks): < 90 seconds, < $0.02

## Content

All corpus files contain Japanese text (light novel/visual novel style dialogue) suitable for translation testing. Content is original and created for testing purposes.
