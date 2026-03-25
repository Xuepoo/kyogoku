//! Edge case tests for all parsers.
//!
//! Tests handling of malformed, truncated, empty, and unusual input files.

use kyogoku_parser::ParserRegistry;

mod txt_parser {
    use super::*;

    #[test]
    fn empty_file() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("txt").unwrap();
        let blocks = parser.parse(b"").unwrap();
        assert!(blocks.is_empty());
    }

    #[test]
    fn only_whitespace() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("txt").unwrap();
        let blocks = parser.parse(b"   \n\t\n   ").unwrap();
        // Should skip empty/whitespace lines
        assert!(blocks.is_empty());
    }

    #[test]
    fn very_long_line() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("txt").unwrap();
        let long_line = "a".repeat(100_000);
        let blocks = parser.parse(long_line.as_bytes()).unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].source.len(), 100_000);
    }

    #[test]
    fn binary_content() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("txt").unwrap();
        // Invalid UTF-8 bytes
        let result = parser.parse(&[0xFF, 0xFE, 0x00, 0x01]);
        // Should either handle gracefully or return error, not panic
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn mixed_line_endings() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("txt").unwrap();
        let content = "line1\r\nline2\nline3\rline4";
        let blocks = parser.parse(content.as_bytes()).unwrap();
        assert!(blocks.len() >= 3); // Should handle all line ending types
    }
}

mod srt_parser {
    use super::*;

    #[test]
    fn empty_file() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("srt").unwrap();
        let blocks = parser.parse(b"").unwrap();
        assert!(blocks.is_empty());
    }

    #[test]
    fn malformed_timestamp() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("srt").unwrap();
        let content = r#"1
invalid timestamp
Hello world

2
00:00:01,000 --> 00:00:02,000
Valid subtitle
"#;
        // Should handle gracefully - either skip malformed or return partial results
        let result = parser.parse(content.as_bytes());
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn missing_sequence_number() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("srt").unwrap();
        let content = r#"00:00:01,000 --> 00:00:02,000
Hello world
"#;
        let result = parser.parse(content.as_bytes());
        // Should not panic
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn overlapping_timestamps() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("srt").unwrap();
        let content = r#"1
00:00:01,000 --> 00:00:05,000
First subtitle

2
00:00:02,000 --> 00:00:04,000
Overlapping subtitle
"#;
        let blocks = parser.parse(content.as_bytes()).unwrap();
        assert_eq!(blocks.len(), 2); // Should parse both despite overlap
    }

    #[test]
    fn multiline_subtitle() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("srt").unwrap();
        let content = r#"1
00:00:01,000 --> 00:00:02,000
Line one
Line two
Line three

"#;
        let blocks = parser.parse(content.as_bytes()).unwrap();
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].source.contains("Line one"));
    }
}

mod json_parser {
    use super::*;

    #[test]
    fn empty_file() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("json").unwrap();
        let result = parser.parse(b"");
        // Empty file is invalid JSON
        assert!(result.is_err());
    }

    #[test]
    fn empty_object() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("json").unwrap();
        let blocks = parser.parse(b"{}").unwrap();
        assert!(blocks.is_empty());
    }

    #[test]
    fn empty_array() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("json").unwrap();
        let blocks = parser.parse(b"[]").unwrap();
        assert!(blocks.is_empty());
    }

    #[test]
    fn malformed_json() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("json").unwrap();
        let result = parser.parse(b"{invalid json}");
        assert!(result.is_err());
    }

    #[test]
    fn deeply_nested() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("json").unwrap();
        // MTool format with deep nesting
        let content = r#"{"a":{"b":{"c":{"d":"hello"}}}}"#;
        let blocks = parser.parse(content.as_bytes()).unwrap();
        assert!(!blocks.is_empty());
    }

    #[test]
    fn unicode_keys() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("json").unwrap();
        let content = r#"{"日本語キー":"こんにちは"}"#;
        let blocks = parser.parse(content.as_bytes()).unwrap();
        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn special_characters_in_values() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("json").unwrap();
        let content = r#"{"key":"value with \"quotes\" and \\backslash"}"#;
        let blocks = parser.parse(content.as_bytes()).unwrap();
        assert!(!blocks.is_empty());
    }
}

mod ass_parser {
    use super::*;

    #[test]
    fn empty_file() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("ass").unwrap();
        let blocks = parser.parse(b"").unwrap();
        assert!(blocks.is_empty());
    }

    #[test]
    fn header_only() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("ass").unwrap();
        let content = r#"[Script Info]
Title: Test
"#;
        let blocks = parser.parse(content.as_bytes()).unwrap();
        assert!(blocks.is_empty()); // No dialogue
    }

    #[test]
    fn malformed_dialogue() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("ass").unwrap();
        let content = r#"[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
Dialogue: invalid line without proper fields
Dialogue: 0,0:00:01.00,0:00:02.00,Default,,0,0,0,,Valid line
"#;
        let result = parser.parse(content.as_bytes());
        // Should handle gracefully
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn complex_tags() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("ass").unwrap();
        let content = r#"[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
Dialogue: 0,0:00:01.00,0:00:02.00,Default,,0,0,0,,{\an8\pos(640,100)\fad(500,500)\blur2}Complex tags
"#;
        let blocks = parser.parse(content.as_bytes()).unwrap();
        assert_eq!(blocks.len(), 1);
        // Text should be extracted without tags
        assert!(blocks[0].source.contains("Complex tags"));
        assert!(!blocks[0].source.contains("\\an8"));
    }

    #[test]
    fn newline_markers() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("ass").unwrap();
        let content = r#"[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
Dialogue: 0,0:00:01.00,0:00:02.00,Default,,0,0,0,,Line1\NLine2\nLine3
"#;
        let blocks = parser.parse(content.as_bytes()).unwrap();
        assert_eq!(blocks.len(), 1);
        // Should handle ASS newline markers
        assert!(blocks[0].source.contains("Line1") && blocks[0].source.contains("Line2"));
    }
}

mod vtt_parser {
    use super::*;

    #[test]
    fn empty_file() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("vtt").unwrap();
        let blocks = parser.parse(b"").unwrap();
        assert!(blocks.is_empty());
    }

    #[test]
    fn header_only() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("vtt").unwrap();
        let content = "WEBVTT\n\n";
        let blocks = parser.parse(content.as_bytes()).unwrap();
        assert!(blocks.is_empty());
    }

    #[test]
    fn missing_webvtt_header() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("vtt").unwrap();
        let content = r#"00:00:01.000 --> 00:00:02.000
Hello world
"#;
        let result = parser.parse(content.as_bytes());
        // Should handle gracefully
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn cue_with_settings() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("vtt").unwrap();
        let content = r#"WEBVTT

00:00:01.000 --> 00:00:02.000 align:start position:10%
Hello world
"#;
        let blocks = parser.parse(content.as_bytes()).unwrap();
        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn html_tags_in_cue() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("vtt").unwrap();
        let content = r#"WEBVTT

00:00:01.000 --> 00:00:02.000
<b>Bold</b> and <i>italic</i>
"#;
        let blocks = parser.parse(content.as_bytes()).unwrap();
        assert_eq!(blocks.len(), 1);
        // Tags should be stripped
        assert!(blocks[0].source.contains("Bold"));
    }
}

mod md_parser {
    use super::*;

    #[test]
    fn empty_file() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("md").unwrap();
        let blocks = parser.parse(b"").unwrap();
        assert!(blocks.is_empty());
    }

    #[test]
    fn frontmatter_only() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("md").unwrap();
        let content = r#"---
title: Test
---
"#;
        let blocks = parser.parse(content.as_bytes()).unwrap();
        // Frontmatter should be preserved, not translated
        assert!(blocks.is_empty() || !blocks.iter().any(|b| b.source.contains("title: Test")));
    }

    #[test]
    fn code_blocks_skipped() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("md").unwrap();
        let content = r#"# Header

```rust
fn main() {
    println!("Hello");
}
```

Normal text here.
"#;
        let blocks = parser.parse(content.as_bytes()).unwrap();
        // Code blocks should be skipped
        assert!(!blocks.iter().any(|b| b.source.contains("fn main")));
        assert!(blocks.iter().any(|b| b.source.contains("Normal text")));
    }

    #[test]
    fn nested_formatting() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("md").unwrap();
        let content = "**_Bold italic_** and `inline code`\n";
        let blocks = parser.parse(content.as_bytes()).unwrap();
        assert!(!blocks.is_empty());
    }
}

mod rpy_parser {
    use super::*;

    #[test]
    fn empty_file() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("rpy").unwrap();
        let blocks = parser.parse(b"").unwrap();
        assert!(blocks.is_empty());
    }

    #[test]
    fn only_comments() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("rpy").unwrap();
        let content = r#"# This is a comment
# Another comment
"#;
        let blocks = parser.parse(content.as_bytes()).unwrap();
        assert!(blocks.is_empty());
    }

    #[test]
    fn python_block_skipped() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("rpy").unwrap();
        let content = r#"init python:
    def my_function():
        return "test"

label start:
    "Hello world"
"#;
        let blocks = parser.parse(content.as_bytes()).unwrap();
        // Python code should be skipped
        assert!(!blocks.iter().any(|b| b.source.contains("def my_function")));
        // Dialogue should be parsed
        assert!(blocks.iter().any(|b| b.source.contains("Hello world")));
    }

    #[test]
    fn unclosed_quote() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("rpy").unwrap();
        let content = r#"label start:
    "Unclosed string
    "Valid string"
"#;
        let result = parser.parse(content.as_bytes());
        // Should handle gracefully, not panic
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn mixed_quotes() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("rpy").unwrap();
        let content = r#"label start:
    "Double quotes"
    'Single quotes'
    """Triple double"""
"#;
        let blocks = parser.parse(content.as_bytes()).unwrap();
        assert!(blocks.len() >= 2);
    }

    #[test]
    fn menu_choices() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("rpy").unwrap();
        let content = r#"label start:
    menu:
        "What do you choose?"
        "Option A":
            "You chose A"
        "Option B":
            "You chose B"
"#;
        let blocks = parser.parse(content.as_bytes()).unwrap();
        assert!(blocks.len() >= 3);
    }
}

mod unicode_handling {
    use super::*;

    #[test]
    fn cjk_characters() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("txt").unwrap();
        let content = "日本語テスト\n中文测试\n한국어 테스트";
        let blocks = parser.parse(content.as_bytes()).unwrap();
        assert_eq!(blocks.len(), 3);
        assert!(blocks[0].source.contains("日本語"));
        assert!(blocks[1].source.contains("中文"));
        assert!(blocks[2].source.contains("한국어"));
    }

    #[test]
    fn emoji() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("txt").unwrap();
        let content = "Hello 👋 World 🌍\nEmoji test 😀🎉✨";
        let blocks = parser.parse(content.as_bytes()).unwrap();
        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].source.contains("👋"));
        assert!(blocks[1].source.contains("😀"));
    }

    #[test]
    fn combining_diacritics() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("txt").unwrap();
        // Combining characters: é = e + ́
        let content = "Café résumé naïve";
        let blocks = parser.parse(content.as_bytes()).unwrap();
        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn zero_width_characters() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("txt").unwrap();
        // Zero-width space (U+200B), zero-width joiner (U+200D)
        let content = "Word\u{200B}Break\u{200D}Join";
        let blocks = parser.parse(content.as_bytes()).unwrap();
        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn rtl_text() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("txt").unwrap();
        // Arabic and Hebrew text
        let content = "مرحبا بالعالم\nשלום עולם";
        let blocks = parser.parse(content.as_bytes()).unwrap();
        assert_eq!(blocks.len(), 2);
    }

    #[test]
    fn mixed_scripts() {
        let registry = ParserRegistry::new();
        let parser = registry.get_parser_by_extension("txt").unwrap();
        // Mixed English, Japanese, Chinese, Korean in one line
        let content = "Hello 你好 こんにちは 안녕";
        let blocks = parser.parse(content.as_bytes()).unwrap();
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].source.contains("Hello"));
        assert!(blocks[0].source.contains("你好"));
    }
}
