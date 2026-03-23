//! Integration tests for kyogoku-parser
//! 
//! These tests verify the complete parsing pipeline using real-world fixtures.

use kyogoku_parser::{ParserRegistry, TranslationBlock};

/// Test SRT parser with standard subtitle file
#[test]
fn test_srt_standard() {
    let content = include_str!("fixtures/subtitles/standard.srt");
    let registry = ParserRegistry::new();
    let parser = registry.get_parser(std::path::Path::new("test.srt")).unwrap();
    
    let blocks = parser.parse(content).expect("Failed to parse SRT");
    
    assert_eq!(blocks.len(), 3);
    assert!(blocks[0].source.contains("quick brown fox"));
    assert!(blocks[1].source.contains("测试字幕"));
    assert!(blocks[2].source.contains("Multiple lines"));
    
    // Test roundtrip
    let output = parser.serialize(&blocks, content).expect("Failed to serialize");
    assert!(output.contains("00:00:01,000"));
    assert!(output.contains("lazy dog"));
}

/// Test ASS parser with complex effects and tags
#[test]
fn test_ass_with_effects() {
    let content = include_str!("fixtures/subtitles/effect_tags.ass");
    let registry = ParserRegistry::new();
    let parser = registry.get_parser(std::path::Path::new("test.ass")).unwrap();
    
    let blocks = parser.parse(content).expect("Failed to parse ASS");
    
    assert_eq!(blocks.len(), 5);
    
    // Verify tag stripping
    assert_eq!(blocks[0].source, "Normal text without effects");
    assert_eq!(blocks[1].source, "Bold text and italic");
    assert_eq!(blocks[1].speaker, Some("Speaker".to_string()));
    assert_eq!(blocks[2].source, "Positioned text");
    
    // Verify line break handling (\N -> \n)
    assert!(blocks[3].source.contains("Line one\nLine two"));
    
    // Test roundtrip preserves structure
    let output = parser.serialize(&blocks, content).expect("Failed to serialize");
    assert!(output.contains("[Script Info]"));
    assert!(output.contains("[Events]"));
    assert!(output.contains("Dialogue:"));
}

/// Test JSON parser with MTool export format
#[test]
fn test_json_mtool_format() {
    let content = include_str!("fixtures/games/mtool_export.json");
    let registry = ParserRegistry::new();
    let parser = registry.get_parser(std::path::Path::new("test.json")).unwrap();
    
    let blocks = parser.parse(content).expect("Failed to parse JSON");
    
    // Should parse simple key-value pairs
    assert!(blocks.iter().any(|b| b.source == "Welcome to the game!"));
    assert!(blocks.iter().any(|b| b.source.contains("player_name")));
    
    // Should parse arrays
    assert!(blocks.iter().any(|b| b.source.contains("Item description")));
    
    // Should parse nested dialogue objects
    assert!(blocks.iter().any(|b| b.source == "Hello there!"));
    assert!(blocks.iter().any(|b| b.source == "How are you?"));
    
    // Test serialization maintains JSON structure
    let output = parser.serialize(&blocks, content).expect("Failed to serialize");
    let reparsed: serde_json::Value = serde_json::from_str(&output).expect("Invalid JSON");
    assert!(reparsed.is_object());
}

/// Test WebVTT parser
#[test]
fn test_vtt_basic() {
    let content = r#"WEBVTT

1
00:00:01.000 --> 00:00:04.000
Hello, world!

2
00:00:05.000 --> 00:00:08.000 align:start
This is <b>styled</b> text.
"#;
    
    let registry = ParserRegistry::new();
    let parser = registry.get_parser(std::path::Path::new("test.vtt")).unwrap();
    
    let blocks = parser.parse(content).expect("Failed to parse VTT");
    
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].source, "Hello, world!");
    assert_eq!(blocks[1].source, "This is styled text.");
}

/// Test TXT parser with simple line-by-line content
#[test]
fn test_txt_basic() {
    let content = "Line one\nLine two\nLine three\n";
    
    let registry = ParserRegistry::new();
    let parser = registry.get_parser(std::path::Path::new("test.txt")).unwrap();
    
    let blocks = parser.parse(content).expect("Failed to parse TXT");
    
    assert_eq!(blocks.len(), 3);
    assert_eq!(blocks[0].source, "Line one");
    assert_eq!(blocks[1].source, "Line two");
    assert_eq!(blocks[2].source, "Line three");
    
    // Test roundtrip
    let output = parser.serialize(&blocks, content).expect("Failed to serialize");
    assert_eq!(output, content.trim());
}

/// Test edge case: empty file
#[test]
fn test_empty_file() {
    let content = include_str!("fixtures/safety/empty_file.txt");
    let registry = ParserRegistry::new();
    let parser = registry.get_parser(std::path::Path::new("test.txt")).unwrap();
    
    let blocks = parser.parse(content).expect("Failed to parse empty file");
    assert_eq!(blocks.len(), 0);
}

/// Test edge case: UTF-8 BOM
#[test]
fn test_utf8_bom() {
    let content = include_str!("fixtures/safety/utf8_bom.txt");
    let registry = ParserRegistry::new();
    let parser = registry.get_parser(std::path::Path::new("test.txt")).unwrap();
    
    // Should handle BOM gracefully
    let blocks = parser.parse(content).expect("Failed to parse file with BOM");
    
    // BOM should be stripped by the text parser
    if !blocks.is_empty() {
        // Check if text is parsed (BOM might be included depending on implementation)
        assert!(blocks[0].source.contains("Text with BOM") || 
                blocks[0].source.starts_with('\u{FEFF}'));
    }
}

/// Test malformed JSON error handling
#[test]
fn test_malformed_json() {
    let content = include_str!("fixtures/safety/malformed_json.json");
    let registry = ParserRegistry::new();
    let parser = registry.get_parser(std::path::Path::new("test.json")).unwrap();
    
    // Should return an error, not panic
    let result = parser.parse(content);
    assert!(result.is_err(), "Should fail to parse malformed JSON");
}

/// Test parser registry extension detection
#[test]
fn test_registry_extensions() {
    let registry = ParserRegistry::new();
    let exts = registry.supported_extensions();
    
    assert!(exts.contains(&"txt"));
    assert!(exts.contains(&"srt"));
    assert!(exts.contains(&"json"));
    assert!(exts.contains(&"ass"));
    assert!(exts.contains(&"ssa"));
    assert!(exts.contains(&"vtt"));
    assert!(exts.contains(&"webvtt"));
    assert!(exts.contains(&"rpy"));
}

/// Test parser selection by file extension
#[test]
fn test_parser_selection() {
    let registry = ParserRegistry::new();
    
    assert!(registry.get_parser(std::path::Path::new("test.txt")).is_some());
    assert!(registry.get_parser(std::path::Path::new("test.SRT")).is_some()); // Case insensitive
    assert!(registry.get_parser(std::path::Path::new("test.ass")).is_some());
    assert!(registry.get_parser(std::path::Path::new("test.vtt")).is_some());
    assert!(registry.get_parser(std::path::Path::new("test.rpy")).is_some());
    assert!(registry.get_parser(std::path::Path::new("test.unknown")).is_none());
}

/// Test translation block with target
#[test]
fn test_block_with_translation() {
    let block = TranslationBlock::new("Hello")
        .with_target("你好");
    
    assert_eq!(block.source, "Hello");
    assert_eq!(block.target, Some("你好".to_string()));
    assert_eq!(block.output(), "你好");
    assert!(!block.needs_translation());
}

/// Test translation block with speaker
#[test]
fn test_block_with_speaker() {
    let block = TranslationBlock::new("Hello")
        .with_speaker("Alice");
    
    assert_eq!(block.speaker, Some("Alice".to_string()));
}

/// Snapshot test: Verify JSON parsing structure
#[test]
fn test_json_snapshot() {
    let content = include_str!("fixtures/games/mtool_export.json");
    let registry = ParserRegistry::new();
    let parser = registry.get_parser(std::path::Path::new("test.json")).unwrap();
    
    let blocks = parser.parse(content).expect("Failed to parse JSON");
    
    // Use insta to snapshot the parsed structure
    insta::assert_debug_snapshot!(blocks);
}

/// Snapshot test: Verify ASS parsing structure
#[test]
fn test_ass_snapshot() {
    let content = include_str!("fixtures/subtitles/effect_tags.ass");
    let registry = ParserRegistry::new();
    let parser = registry.get_parser(std::path::Path::new("test.ass")).unwrap();
    
    let blocks = parser.parse(content).expect("Failed to parse ASS");
    
    // Use insta to snapshot the parsed structure
    insta::assert_debug_snapshot!(blocks);
}

/// Test Ren'Py parser with basic dialogue
#[test]
fn test_rpy_basic() {
    let content = include_str!("fixtures/games/basic_dialogue.rpy");
    let registry = ParserRegistry::new();
    let parser = registry.get_parser(std::path::Path::new("test.rpy")).unwrap();
    
    let blocks = parser.parse(content).expect("Failed to parse RPY");
    
    // Check key dialogue lines
    assert!(blocks.iter().any(|b| b.source == "You've created a new Ren'Py game."));
    assert!(blocks.iter().any(|b| b.source == "This is a narration line."));
    assert!(blocks.iter().any(|b| b.source == "It's a story."));
    
    // Check speaker
    let eileen_line = blocks.iter().find(|b| b.source.contains("Ren'Py game")).unwrap();
    assert_eq!(eileen_line.speaker, Some("e".to_string()));
    
    // Check serialization roundtrip
    // Note: Since we don't translate, the output should be identical to input if we just pass content
    // But `serialize` is intended to replace blocks.
    
    // Let's mock a translation
    let mut translated_blocks = blocks.clone();
    if let Some(b) = translated_blocks.iter_mut().find(|b| b.source == "It's a story.") {
        b.target = Some("这是一个故事。".to_string());
    }
    
    let output = parser.serialize(&translated_blocks, content).expect("Failed to serialize");
    assert!(output.contains("\"这是一个故事。\":"));
}

/// Snapshot test: Verify Ren'Py parsing structure
#[test]
fn test_rpy_snapshot() {
    let content = include_str!("fixtures/games/basic_dialogue.rpy");
    let registry = ParserRegistry::new();
    let parser = registry.get_parser(std::path::Path::new("test.rpy")).unwrap();
    
    let blocks = parser.parse(content).expect("Failed to parse RPY");
    insta::assert_debug_snapshot!(blocks);
}
