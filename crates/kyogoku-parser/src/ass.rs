//! ASS/SSA (Advanced SubStation Alpha) subtitle parser.
//!
//! Supports both ASS (v4.00+) and SSA (v4.00) formats.
//! Preserves styling information and metadata while extracting dialogue text.

use anyhow::Result;
use serde_json::json;

use crate::block::TranslationBlock;
use crate::parser::Parser;

/// ASS/SSA subtitle parser.
pub struct AssParser;

/// Represents an ASS timestamp (H:MM:SS.CC)
#[derive(Debug, Clone, PartialEq)]
struct AssTimestamp {
    hours: u32,
    minutes: u32,
    seconds: u32,
    centiseconds: u32,
}

impl AssTimestamp {
    fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 3 {
            return None;
        }
        
        let hours = parts[0].parse().ok()?;
        let minutes = parts[1].parse().ok()?;
        
        let sec_parts: Vec<&str> = parts[2].split('.').collect();
        if sec_parts.len() != 2 {
            return None;
        }
        
        let seconds = sec_parts[0].parse().ok()?;
        let centiseconds = sec_parts[1].parse().ok()?;
        
        Some(Self { hours, minutes, seconds, centiseconds })
    }
    
    fn to_string(&self) -> String {
        format!(
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

impl DialogueLine {
    fn parse(line: &str) -> Option<Self> {
        // Dialogue: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
        let content = line.strip_prefix("Dialogue:")?.trim();
        
        // Split by comma, but only first 9 commas (Text can contain commas)
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut count = 0;
        
        for ch in content.chars() {
            if ch == ',' && count < 9 {
                parts.push(std::mem::take(&mut current));
                count += 1;
            } else {
                current.push(ch);
            }
        }
        parts.push(current); // Add the remaining text
        
        if parts.len() < 10 {
            return None;
        }
        
        Some(Self {
            layer: parts[0].trim().parse().unwrap_or(0),
            start: AssTimestamp::parse(parts[1].trim())?,
            end: AssTimestamp::parse(parts[2].trim())?,
            style: parts[3].trim().to_string(),
            name: parts[4].trim().to_string(),
            margin_l: parts[5].trim().parse().unwrap_or(0),
            margin_r: parts[6].trim().parse().unwrap_or(0),
            margin_v: parts[7].trim().parse().unwrap_or(0),
            effect: parts[8].trim().to_string(),
            text: parts[9..].join(","),
        })
    }
}

/// Extract plain text from ASS text with override tags removed.
fn strip_ass_tags(text: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    let mut chars = text.chars().peekable();
    
    while let Some(c) = chars.next() {
        if c == '{' {
            in_tag = true;
        } else if c == '}' {
            in_tag = false;
        } else if !in_tag {
            // Handle \N (hard line break) and \n (soft line break)
            if c == '\\' {
                if let Some(&next) = chars.peek() {
                    if next == 'N' || next == 'n' {
                        result.push('\n');
                        chars.next();
                        continue;
                    }
                }
            }
            result.push(c);
        }
    }
    
    result
}

impl Parser for AssParser {
    fn extensions(&self) -> &[&str] {
        &["ass", "ssa"]
    }

    fn parse(&self, content: &str) -> Result<Vec<TranslationBlock>> {
        let mut blocks = Vec::new();
        let mut in_events = false;
        let mut dialogue_index = 0u32;
        
        for line in content.lines() {
            let line = line.trim();
            
            // Check for section headers
            if line.starts_with('[') && line.ends_with(']') {
                in_events = line.eq_ignore_ascii_case("[Events]");
                continue;
            }
            
            // Only process dialogue lines in Events section
            if in_events && line.starts_with("Dialogue:") {
                if let Some(dialogue) = DialogueLine::parse(line) {
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

    fn serialize(&self, blocks: &[TranslationBlock], template: &str) -> Result<String> {
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
            
            for line in template.lines() {
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
            let layer = block.metadata.get("layer").and_then(|v| v.as_u64()).unwrap_or(0);
            let start = block.metadata.get("start").and_then(|v| v.as_str()).unwrap_or("0:00:00.00");
            let end = block.metadata.get("end").and_then(|v| v.as_str()).unwrap_or("0:00:10.00");
            let style = block.metadata.get("style").and_then(|v| v.as_str()).unwrap_or("Default");
            let name = block.metadata.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let margin_l = block.metadata.get("margin_l").and_then(|v| v.as_u64()).unwrap_or(0);
            let margin_r = block.metadata.get("margin_r").and_then(|v| v.as_u64()).unwrap_or(0);
            let margin_v = block.metadata.get("margin_v").and_then(|v| v.as_u64()).unwrap_or(0);
            let effect = block.metadata.get("effect").and_then(|v| v.as_str()).unwrap_or("");
            
            // Use translated text, escape newlines as \N
            let text = block.output().replace('\n', "\\N");
            
            output.push_str(&format!(
                "Dialogue: {},{},{},{},{},{},{},{},{},{}\n",
                layer, start, end, style, name, margin_l, margin_r, margin_v, effect, text
            ));
        }
        
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_ASS: &str = r#"[Script Info]
ScriptType: v4.00+
PlayResX: 1920
PlayResY: 1080

[V4+ Styles]
Format: Name, Fontname, Fontsize, PrimaryColour
Style: Default,Arial,48,&H00FFFFFF

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
Dialogue: 0,0:00:01.00,0:00:04.00,Default,,0,0,0,,Hello, world!
Dialogue: 0,0:00:05.00,0:00:08.00,Default,Speaker,0,0,0,,{\b1}Bold text{\b0} normal
Dialogue: 0,0:00:09.00,0:00:12.00,Default,,0,0,0,,Line one\NLine two
"#;

    #[test]
    fn test_parse_timestamp() {
        let ts = AssTimestamp::parse("1:23:45.67").unwrap();
        assert_eq!(ts.hours, 1);
        assert_eq!(ts.minutes, 23);
        assert_eq!(ts.seconds, 45);
        assert_eq!(ts.centiseconds, 67);
    }

    #[test]
    fn test_strip_ass_tags() {
        assert_eq!(strip_ass_tags("plain text"), "plain text");
        assert_eq!(strip_ass_tags("{\\b1}bold{\\b0}"), "bold");
        assert_eq!(strip_ass_tags("line1\\Nline2"), "line1\nline2");
        assert_eq!(strip_ass_tags("{\\pos(100,200)}text"), "text");
    }

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
    fn test_ass_serialize() {
        let blocks = vec![
            TranslationBlock::new("Hello")
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
                .with_target("你好"),
        ];

        let parser = AssParser;
        let output = parser.serialize(&blocks, "").unwrap();

        assert!(output.contains("[Script Info]"));
        assert!(output.contains("[Events]"));
        assert!(output.contains("Dialogue:"));
        assert!(output.contains("你好"));
    }

    #[test]
    fn test_ass_roundtrip() {
        let parser = AssParser;
        let blocks = parser.parse(SAMPLE_ASS).unwrap();
        
        // Add translations
        let translated: Vec<_> = blocks.into_iter().map(|b| {
            let source = b.source.clone();
            b.with_target(format!("[TR] {}", source))
        }).collect();
        
        let output = parser.serialize(&translated, SAMPLE_ASS).unwrap();
        
        // Re-parse
        let reparsed = parser.parse(&output).unwrap();
        assert_eq!(reparsed.len(), 3);
        assert!(reparsed[0].source.contains("[TR]"));
    }
}
