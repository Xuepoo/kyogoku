//! ASS/SSA (Advanced SubStation Alpha) subtitle parser.
//!
//! Supports both ASS (v4.00+) and SSA (v4.00) formats.
//! Preserves styling information and metadata while extracting dialogue text.
//!
//! # Format Overview
//! ASS files contain:
//! - `[Script Info]` - metadata (title, original script, etc.)
//! - `[V4+ Styles]` or `[Styles]` - style definitions
//! - `[Events]` - dialogue and comment lines with timestamps
//!
//! This parser specifically extracts dialogue from the Events section,
//! preserving timing and styling information while extracting translatable text.

use anyhow::Result;
use nom::{
    bytes::complete::{is_not, tag, take_while},
    character::complete::{char, digit1, not_line_ending},
    combinator::map,
    sequence::delimited,
    IResult,
};
use serde_json::json;

use crate::block::TranslationBlock;
use crate::parser::Parser;

/// ASS/SSA subtitle parser using nom for robust parsing.
pub struct AssParser;

/// Represents an ASS timestamp (H:MM:SS.CC)
#[derive(Debug, Clone, PartialEq, Eq)]
struct AssTimestamp {
    hours: u32,
    minutes: u32,
    seconds: u32,
    centiseconds: u32,
}

impl AssTimestamp {
    /// Parse a timestamp in ASS format: H:MM:SS.CC
    #[allow(dead_code)]
    fn parse(s: &str) -> Option<Self> {
        let (_, timestamp) = parse_timestamp(s).ok()?;
        Some(timestamp)
    }

    /// Convert to string in ASS format
    fn to_string(&self) -> String {
        format!(
            "{}:{:02}:{:02}.{:02}",
            self.hours, self.minutes, self.seconds, self.centiseconds
        )
    }
}

impl std::fmt::Display for AssTimestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{:02}:{:02}.{:02}",
            self.hours, self.minutes, self.seconds, self.centiseconds
        )
    }
}

/// Represents a dialogue line from the [Events] section
#[derive(Debug, Clone)]
struct DialogueLine {
    layer: u32,
    start: AssTimestamp,
    end: AssTimestamp,
    style: String,
    name: String,
    margin_l: u32,
    margin_r: u32,
    margin_v: u32,
    effect: String,
    text: String,
}

// ============================================================================
// NOM PARSERS
// ============================================================================

/// Parse unsigned integer
fn parse_uint<'a>(input: &'a str) -> IResult<&'a str, u32> {
    map(digit1, |s: &str| s.parse::<u32>().unwrap_or(0))(input)
}

/// Parse a timestamp in ASS format: H:MM:SS.CC
fn parse_timestamp(input: &str) -> IResult<&str, AssTimestamp> {
    let (input, hours) = parse_uint(input)?;
    let (input, _) = char(':')(input)?;
    let (input, minutes) = parse_uint(input)?;
    let (input, _) = char(':')(input)?;
    let (input, seconds) = parse_uint(input)?;
    let (input, _) = char('.')(input)?;
    let (input, centiseconds) = parse_uint(input)?;

    Ok((
        input,
        AssTimestamp {
            hours,
            minutes,
            seconds,
            centiseconds,
        },
    ))
}

/// Parse a field value (trimmed)
fn parse_field<'a>(input: &'a str) -> IResult<&'a str, String> {
    let (input, value) = take_while(|c| c != ',')(input)?;
    Ok((input, value.trim().to_string()))
}

/// Parse a dialogue line: "Dialogue: layer,start,end,style,name,marginL,marginR,marginV,effect,text"
fn parse_dialogue_line(input: &str) -> IResult<&str, DialogueLine> {
    // Skip "Dialogue:" prefix
    let (input, _) = tag("Dialogue:")(input)?;

    // Parse the 9 fixed fields before text
    let (input, layer_str) = delimited(
        take_while(|c| c == ' ' || c == '\t'),
        is_not(","),
        tag(","),
    )(input)?;
    let layer = layer_str.trim().parse::<u32>().unwrap_or(0);

    // Start timestamp
    let (input, _) = take_while(|c| c == ' ' || c == '\t')(input)?;
    let (input, start) = parse_timestamp(input)?;
    let (input, _) = tag(",")(input)?;

    // End timestamp
    let (input, _) = take_while(|c| c == ' ' || c == '\t')(input)?;
    let (input, end) = parse_timestamp(input)?;
    let (input, _) = tag(",")(input)?;

    // Style
    let (input, style) = parse_field(input)?;
    let (input, _) = tag(",")(input)?;

    // Name
    let (input, name) = parse_field(input)?;
    let (input, _) = tag(",")(input)?;

    // MarginL
    let (input, margin_l_str) = take_while(|c| c != ',')(input)?;
    let margin_l = margin_l_str.trim().parse::<u32>().unwrap_or(0);
    let (input, _) = tag(",")(input)?;

    // MarginR
    let (input, margin_r_str) = take_while(|c| c != ',')(input)?;
    let margin_r = margin_r_str.trim().parse::<u32>().unwrap_or(0);
    let (input, _) = tag(",")(input)?;

    // MarginV
    let (input, margin_v_str) = take_while(|c| c != ',')(input)?;
    let margin_v = margin_v_str.trim().parse::<u32>().unwrap_or(0);
    let (input, _) = tag(",")(input)?;

    // Effect
    let (input, effect) = parse_field(input)?;
    let (input, _) = tag(",")(input)?;

    // Text (can contain commas)
    let (input, _) = take_while(|c| c == ' ' || c == '\t')(input)?;
    let (input, text) = not_line_ending(input)?;

    Ok((
        input,
        DialogueLine {
            layer,
            start,
            end,
            style,
            name,
            margin_l,
            margin_r,
            margin_v,
            effect,
            text: text.to_string(),
        },
    ))
}

/// Parse section header like "[Events]"
fn parse_section_header(input: &str) -> IResult<&str, String> {
    let (input, header) = delimited(char('['), is_not("]"), char(']'))(input)?;
    Ok((input, header.to_string()))
}

// ============================================================================
// TEXT PROCESSING UTILITIES
// ============================================================================

/// Strip ASS override tags `{...}` while preserving text and converting ASS line breaks
pub fn strip_ass_tags(text: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '{' => in_tag = true,
            '}' => in_tag = false,
            '\\' if !in_tag => {
                if let Some(&next) = chars.peek() {
                    match next {
                        'N' => {
                            result.push('\n');
                            chars.next();
                        }
                        'n' => {
                            result.push(' ');
                            chars.next();
                        }
                        'h' => {
                            result.push(' ');
                            chars.next();
                        }
                        _ => result.push(c),
                    }
                } else {
                    result.push(c);
                }
            }
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }

    result
}

/// Extract styled tags and text content separately
/// Returns (text_with_preserved_tags, plain_text)
pub fn preserve_ass_tags(text: &str) -> (String, String) {
    let plain = strip_ass_tags(text);
    (text.to_string(), plain)
}

/// Reinsert styling tags into translated text
/// Simple heuristic: if original had tags, apply them to the start
pub fn reinsert_ass_tags(original: &str, translated: &str) -> String {
    // Extract leading tags before first text character
    let mut leading_tags = String::new();
    let mut in_tag = false;
    let mut found_text = false;

    for c in original.chars() {
        if found_text {
            break;
        }
        match c {
            '{' => {
                in_tag = true;
                leading_tags.push(c);
            }
            '}' => {
                in_tag = false;
                leading_tags.push(c);
            }
            ' ' | '\t' | '\n' | '\r' => {
                leading_tags.push(c);
            }
            _ if in_tag => {
                leading_tags.push(c);
            }
            _ => {
                found_text = true;
            }
        }
    }

    if leading_tags.is_empty() {
        translated.to_string()
    } else {
        format!("{}{}", leading_tags, translated)
    }
}


impl Parser for AssParser {
    fn extensions(&self) -> &[&str] {
        &["ass", "ssa"]
    }

    fn parse(&self, content: &[u8]) -> Result<Vec<TranslationBlock>> {
        let content_str = std::str::from_utf8(content)?;
        let mut blocks = Vec::new();
        let mut in_events = false;
        let mut dialogue_index = 0u32;

        for line in content_str.lines() {
            let trimmed = line.trim();

            // Check for section headers
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                if let Ok((_, header)) = parse_section_header(trimmed) {
                    in_events = header.eq_ignore_ascii_case("Events");
                }
                continue;
            }

            // Only process dialogue lines in Events section
            if in_events && trimmed.starts_with("Dialogue:") {
                if let Ok((_, dialogue)) = parse_dialogue_line(trimmed) {
                    let plain_text = strip_ass_tags(&dialogue.text);

                    // Skip empty dialogues
                    if plain_text.trim().is_empty() {
                        continue;
                    }

                    let speaker = if dialogue.name.is_empty() {
                        None
                    } else {
                        Some(dialogue.name.clone())
                    };

                    let mut block = TranslationBlock::new(&plain_text);
                    if let Some(s) = speaker {
                        block = block.with_speaker(s);
                    }

                    block = block.with_metadata(json!({
                        "format": "ass",
                        "index": dialogue_index,
                        "layer": dialogue.layer,
                        "start": dialogue.start.to_string(),
                        "end": dialogue.end.to_string(),
                        "style": dialogue.style,
                        "name": dialogue.name,
                        "margin_l": dialogue.margin_l,
                        "margin_r": dialogue.margin_r,
                        "margin_v": dialogue.margin_v,
                        "effect": dialogue.effect,
                        "original_text": dialogue.text,
                    }));

                    blocks.push(block);
                    dialogue_index += 1;
                }
            }
        }

        tracing::debug!("Parsed {} dialogue blocks from ASS", blocks.len());
        Ok(blocks)
    }

    fn serialize(&self, blocks: &[TranslationBlock], template: &[u8]) -> Result<Vec<u8>> {
        let template_str = std::str::from_utf8(template)?;
        // If template is provided, use it as base
        let mut output = if template.is_empty() {
            // Generate minimal ASS header
            r#"[Script Info]
ScriptType: v4.00+
PlayResX: 1920
PlayResY: 1080

[V4+ Styles]
Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding
Style: Default,Arial,48,&H00FFFFFF,&H000000FF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,2,2,2,10,10,10,1

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
"#.to_string()
        } else {
            // Use template but replace dialogues
            let mut result = String::new();
            let mut in_events = false;

            for line in template_str.lines() {
                let trimmed = line.trim();

                if trimmed.starts_with('[') && trimmed.ends_with(']') {
                    in_events = trimmed.eq_ignore_ascii_case("[Events]");
                    result.push_str(line);
                    result.push('\n');
                    continue;
                }

                if in_events {
                    if trimmed.starts_with("Format:") {
                        result.push_str(line);
                        result.push('\n');
                    }
                    // Skip existing Dialogue lines - we'll add our own
                    if trimmed.starts_with("Dialogue:") {
                        continue;
                    }
                    // Skip Comment lines
                    if trimmed.starts_with("Comment:") {
                        continue;
                    }
                } else {
                    result.push_str(line);
                    result.push('\n');
                }
            }

            result
        };

        // Add dialogue lines
        for block in blocks {
            let layer = block
                .metadata
                .get("layer")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let start = block
                .metadata
                .get("start")
                .and_then(|v| v.as_str())
                .unwrap_or("0:00:00.00");
            let end = block
                .metadata
                .get("end")
                .and_then(|v| v.as_str())
                .unwrap_or("0:00:10.00");
            let style = block
                .metadata
                .get("style")
                .and_then(|v| v.as_str())
                .unwrap_or("Default");
            let name = block
                .metadata
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let margin_l = block
                .metadata
                .get("margin_l")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let margin_r = block
                .metadata
                .get("margin_r")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let margin_v = block
                .metadata
                .get("margin_v")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let effect = block
                .metadata
                .get("effect")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            // Use translated text, escape newlines as \N
            let text = block.output().replace('\n', "\\N");

            output.push_str(&format!(
                "Dialogue: {},{},{},{},{},{},{},{},{},{}\n",
                layer, start, end, style, name, margin_l, margin_r, margin_v, effect, text
            ));
        }

        Ok(output.into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_ASS: &[u8] = b"[Script Info]
ScriptType: v4.00+
PlayResX: 1920
PlayResY: 1080

[V4+ Styles]
Format: Name, Fontname, Fontsize, PrimaryColour
Style: Default,Arial,48,&H00FFFFFF

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
Dialogue: 0,0:00:01.00,0:00:04.00,Default,,0,0,0,,Hello, world!
Dialogue: 0,0:00:05.00,0:00:08.00,Default,Speaker,0,0,0,,{\\b1}Bold text{\\b0} normal
Dialogue: 0,0:00:09.00,0:00:12.00,Default,,0,0,0,,Line one\\NLine two
";

    const COMPLEX_ASS: &[u8] = b"[Script Info]
ScriptType: v4.00+
Title: Complex Test
Original Script: Test

[V4+ Styles]
Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour
Style: Default,Arial,48,&H00FFFFFF,&H000000FF,&H00000000,&H00000000
Style: Alt,Georgia,52,&H0000FF00,&H000000FF,&H00000000,&H00000000

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
Dialogue: 0,0:00:01.00,0:00:04.00,Default,Alice,0,0,0,,{\\pos(100,200)}Positioned text
Dialogue: 1,0:00:05.00,0:00:08.00,Alt,Bob,10,20,30,ScrollUp,{\\c&HFF0000&}Red text{\\c}
Dialogue: 0,0:00:09.00,0:00:12.00,Default,,0,0,0,,Multiple\\Nlines\\Nof\\Ntext
Dialogue: 0,0:00:13.00,0:00:15.00,Default,,0,0,0,,{\\an8}{\\fscx150}scaled text
Comment: 0,0:00:16.00,0:00:18.00,Default,,0,0,0,,This should be ignored
";

    // ========================================================================
    // UNIT TESTS FOR NOM PARSERS
    // ========================================================================

    #[test]
    fn test_parse_timestamp() {
        let ts = AssTimestamp::parse("1:23:45.67").unwrap();
        assert_eq!(ts.hours, 1);
        assert_eq!(ts.minutes, 23);
        assert_eq!(ts.seconds, 45);
        assert_eq!(ts.centiseconds, 67);
    }

    #[test]
    fn test_parse_timestamp_zero_padded() {
        let ts = AssTimestamp::parse("0:00:01.00").unwrap();
        assert_eq!(ts.hours, 0);
        assert_eq!(ts.minutes, 0);
        assert_eq!(ts.seconds, 1);
        assert_eq!(ts.centiseconds, 0);
    }

    #[test]
    fn test_parse_timestamp_large_values() {
        let ts = AssTimestamp::parse("10:59:59.99").unwrap();
        assert_eq!(ts.hours, 10);
        assert_eq!(ts.minutes, 59);
        assert_eq!(ts.seconds, 59);
        assert_eq!(ts.centiseconds, 99);
    }

    #[test]
    fn test_timestamp_display() {
        let ts = AssTimestamp {
            hours: 1,
            minutes: 23,
            seconds: 45,
            centiseconds: 67,
        };
        assert_eq!(ts.to_string(), "1:23:45.67");
    }

    #[test]
    fn test_parse_simple_dialogue_line() {
        let line = "Dialogue: 0,0:00:01.00,0:00:04.00,Default,,0,0,0,,Hello, world!";
        let (_, dialogue) = parse_dialogue_line(line).unwrap();

        assert_eq!(dialogue.layer, 0);
        assert_eq!(dialogue.start.hours, 0);
        assert_eq!(dialogue.start.minutes, 0);
        assert_eq!(dialogue.start.seconds, 1);
        assert_eq!(dialogue.end.to_string(), "0:00:04.00");
        assert_eq!(dialogue.style, "Default");
        assert_eq!(dialogue.name, "");
        assert_eq!(dialogue.margin_l, 0);
        assert_eq!(dialogue.margin_r, 0);
        assert_eq!(dialogue.margin_v, 0);
        assert_eq!(dialogue.effect, "");
        assert_eq!(dialogue.text, "Hello, world!");
    }

    #[test]
    fn test_parse_dialogue_with_speaker() {
        let line = "Dialogue: 1,0:00:05.00,0:00:08.00,Alt,Alice,10,20,30,ScrollUp,{\\c&HFF0000&}Red{\\c}";
        let (_, dialogue) = parse_dialogue_line(line).unwrap();

        assert_eq!(dialogue.layer, 1);
        assert_eq!(dialogue.name, "Alice");
        assert_eq!(dialogue.style, "Alt");
        assert_eq!(dialogue.margin_l, 10);
        assert_eq!(dialogue.margin_r, 20);
        assert_eq!(dialogue.margin_v, 30);
        assert_eq!(dialogue.effect, "ScrollUp");
        assert_eq!(dialogue.text, "{\\c&HFF0000&}Red{\\c}");
    }

    #[test]
    fn test_parse_dialogue_with_commas_in_text() {
        let line = "Dialogue: 0,0:00:01.00,0:00:04.00,Default,,0,0,0,,Text with, multiple, commas";
        let (_, dialogue) = parse_dialogue_line(line).unwrap();

        assert_eq!(dialogue.text, "Text with, multiple, commas");
    }

    // ========================================================================
    // UNIT TESTS FOR TAG STRIPPING
    // ========================================================================

    #[test]
    fn test_strip_ass_tags_plain() {
        assert_eq!(strip_ass_tags("plain text"), "plain text");
    }

    #[test]
    fn test_strip_ass_tags_bold() {
        assert_eq!(strip_ass_tags("{\\b1}bold{\\b0}"), "bold");
    }

    #[test]
    fn test_strip_ass_tags_newline() {
        assert_eq!(strip_ass_tags("line1\\Nline2"), "line1\nline2");
    }

    #[test]
    fn test_strip_ass_tags_soft_newline() {
        assert_eq!(strip_ass_tags("line1\\nline2"), "line1 line2");
    }

    #[test]
    fn test_strip_ass_tags_hard_space() {
        assert_eq!(strip_ass_tags("word1\\hword2"), "word1 word2");
    }

    #[test]
    fn test_strip_ass_tags_color() {
        assert_eq!(
            strip_ass_tags("{\\c&HFF0000&}Red text{\\c}"),
            "Red text"
        );
    }

    #[test]
    fn test_strip_ass_tags_positioning() {
        assert_eq!(
            strip_ass_tags("{\\pos(100,200)}positioned text"),
            "positioned text"
        );
    }

    #[test]
    fn test_strip_ass_tags_complex() {
        assert_eq!(
            strip_ass_tags("{\\an8}{\\fscx150}scaled text{\\fscx100}"),
            "scaled text"
        );
    }

    #[test]
    fn test_strip_ass_tags_multiple_tags() {
        assert_eq!(
            strip_ass_tags("{\\b1}{\\i1}bold italic{\\b0}{\\i0}"),
            "bold italic"
        );
    }

    // ========================================================================
    // UNIT TESTS FOR TAG PRESERVATION
    // ========================================================================

    #[test]
    fn test_preserve_ass_tags() {
        let (preserved, plain) = preserve_ass_tags("{\\b1}Bold{\\b0} text");
        assert_eq!(preserved, "{\\b1}Bold{\\b0} text");
        assert_eq!(plain, "Bold text");
    }

    #[test]
    fn test_reinsert_ass_tags() {
        let original = "{\\b1}Hello{\\b0}";
        let translated = "你好";
        let result = reinsert_ass_tags(original, translated);

        // Should have some tags preserved
        assert!(result.contains("{"));
    }

    // ========================================================================
    // INTEGRATION TESTS
    // ========================================================================

    #[test]
    fn test_ass_parse() {
        let parser = AssParser;
        let blocks = parser.parse(SAMPLE_ASS).unwrap();

        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].source, "Hello, world!");
        assert_eq!(blocks[1].source, "Bold text normal");
        assert_eq!(blocks[1].speaker, Some("Speaker".to_string()));
        assert!(blocks[2].source.contains("Line one"));
        assert!(blocks[2].source.contains("Line two"));
    }

    #[test]
    fn test_ass_parse_metadata() {
        let parser = AssParser;
        let blocks = parser.parse(SAMPLE_ASS).unwrap();

        // Check first block metadata
        assert_eq!(blocks[0].metadata.get("index").unwrap().as_u64().unwrap(), 0);
        assert_eq!(
            blocks[0].metadata.get("start").unwrap().as_str().unwrap(),
            "0:00:01.00"
        );
        assert_eq!(
            blocks[0].metadata.get("end").unwrap().as_str().unwrap(),
            "0:00:04.00"
        );
        assert_eq!(
            blocks[0].metadata.get("style").unwrap().as_str().unwrap(),
            "Default"
        );
    }

    #[test]
    fn test_ass_parse_complex() {
        let parser = AssParser;
        let blocks = parser.parse(COMPLEX_ASS).unwrap();

        assert_eq!(blocks.len(), 4); // Should skip the Comment line
        assert_eq!(blocks[0].speaker, Some("Alice".to_string()));
        assert_eq!(blocks[1].speaker, Some("Bob".to_string()));
        assert_eq!(blocks[0].source, "Positioned text");
        assert_eq!(blocks[1].source, "Red text");
    }

    #[test]
    fn test_ass_serialize_simple() {
        let blocks = vec![TranslationBlock::new("Hello")
            .with_metadata(json!({
                "layer": 0,
                "start": "0:00:01.00",
                "end": "0:00:04.00",
                "style": "Default",
                "name": "",
                "margin_l": 0,
                "margin_r": 0,
                "margin_v": 0,
                "effect": ""
            }))
            .with_target("你好")];

        let parser = AssParser;
        let output = parser.serialize(&blocks, b"").unwrap();
        let output_str = std::str::from_utf8(&output).unwrap();

        assert!(output_str.contains("[Script Info]"));
        assert!(output_str.contains("[Events]"));
        assert!(output_str.contains("Dialogue:"));
        assert!(output_str.contains("你好"));
    }

    #[test]
    fn test_ass_serialize_with_speaker() {
        let blocks = vec![TranslationBlock::new("Hello")
            .with_speaker("Alice")
            .with_metadata(json!({
                "layer": 0,
                "start": "0:00:01.00",
                "end": "0:00:04.00",
                "style": "Default",
                "name": "Alice",
                "margin_l": 0,
                "margin_r": 0,
                "margin_v": 0,
                "effect": ""
            }))
            .with_target("你好")];

        let parser = AssParser;
        let output = parser.serialize(&blocks, b"").unwrap();
        let output_str = std::str::from_utf8(&output).unwrap();

        assert!(output_str.contains("Dialogue: 0,0:00:01.00,0:00:04.00,Default,Alice"));
    }

    #[test]
    fn test_ass_serialize_with_template() {
        let parser = AssParser;
        let blocks = parser.parse(SAMPLE_ASS).unwrap();

        // Translate blocks
        let translated: Vec<_> = blocks
            .into_iter()
            .map(|b| {
                let source = b.source.clone();
                b.with_target(format!("[TR] {}", source))
            })
            .collect();

        let output = parser.serialize(&translated, SAMPLE_ASS).unwrap();
        let output_str = std::str::from_utf8(&output).unwrap();

        // Should preserve original structure
        assert!(output_str.contains("[Script Info]"));
        assert!(output_str.contains("[V4+ Styles]"));
        assert!(output_str.contains("[Events]"));

        // Should have translated content
        assert!(output_str.contains("[TR]"));
    }

    #[test]
    fn test_ass_roundtrip() {
        let parser = AssParser;
        let blocks = parser.parse(SAMPLE_ASS).unwrap();

        // Add translations
        let translated: Vec<_> = blocks
            .into_iter()
            .map(|b| {
                let source = b.source.clone();
                b.with_target(format!("[TR] {}", source))
            })
            .collect();

        let output = parser.serialize(&translated, SAMPLE_ASS).unwrap();

        // Re-parse
        let reparsed = parser.parse(&output).unwrap();
        assert_eq!(reparsed.len(), 3);
        assert!(reparsed[0].source.contains("[TR]"));
        assert!(reparsed[1].source.contains("[TR]"));
        assert!(reparsed[2].source.contains("[TR]"));
    }

    #[test]
    fn test_ass_newline_handling() {
        let blocks = vec![TranslationBlock::new("Line one\nLine two")
            .with_metadata(json!({
                "layer": 0,
                "start": "0:00:01.00",
                "end": "0:00:04.00",
                "style": "Default",
                "name": "",
                "margin_l": 0,
                "margin_r": 0,
                "margin_v": 0,
                "effect": ""
            }))];

        let parser = AssParser;
        let output = parser.serialize(&blocks, b"").unwrap();
        let output_str = std::str::from_utf8(&output).unwrap();

        // Newlines should be converted to \N
        assert!(output_str.contains("Line one\\NLine two"));
    }

    #[test]
    fn test_ass_empty_dialogue_skipped() {
        let content = b"[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
Dialogue: 0,0:00:01.00,0:00:04.00,Default,,0,0,0,,
Dialogue: 0,0:00:05.00,0:00:08.00,Default,,0,0,0,,Hello
";
        let parser = AssParser;
        let blocks = parser.parse(content).unwrap();

        // Empty dialogue should be skipped
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].source, "Hello");
    }

    #[test]
    fn test_ass_parse_ignores_other_sections() {
        let content = b"[Script Info]
Title: Test

[V4+ Styles]
Format: Name, Fontname, Fontsize
Style: Default,Arial,48

Dialogue: 0,0:00:01.00,0:00:04.00,Default,,0,0,0,,Should not parse

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
Dialogue: 0,0:00:05.00,0:00:08.00,Default,,0,0,0,,Should parse
";
        let parser = AssParser;
        let blocks = parser.parse(content).unwrap();

        // Should only parse the one in [Events] section
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].source, "Should parse");
    }
}

