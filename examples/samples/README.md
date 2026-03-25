# Sample Files for Translation Testing

This directory contains sample files demonstrating various supported formats for translation testing.

## Available Samples

### `novel.txt`
A literary text sample with mixed Japanese and English dialogue. Tests:
- Line-by-line text parsing
- Japanese dialogue markers (「」)
- Mixed language content

### `game_dialogue.json`
RPG-style game dialogue in JSON format. Tests:
- Nested JSON structures
- Speaker identification
- Scene organization

### `subtitles.srt`
Standard SRT subtitle file. Tests:
- Timestamp parsing and preservation
- Multi-line subtitle entries
- Subtitle numbering

### `subtitles.ass`
Advanced SubStation Alpha subtitle file. Tests:
- ASS tag preservation ({\i1}, {\b1}, etc.)
- Speaker/Name field extraction
- Style metadata retention

### `visual_novel.rpy`
Ren'Py visual novel script. Tests:
- Dialogue and narration extraction
- Menu choice handling
- Multiline dialogue blocks (triple quotes)
- Speaker identification

## Usage

```bash
# Translate a single file
kyogoku translate examples/samples/novel.txt -o ./output --from ja --to en

# Translate all samples
kyogoku translate examples/samples/ -o ./output --from ja --to en

# Dry run to preview blocks
kyogoku translate examples/samples/game_dialogue.json --dry-run
```

## Expected Output

Each format should produce output in the same format as input, with:
- Original structure preserved
- Timestamps/tags intact
- Only translatable text modified
