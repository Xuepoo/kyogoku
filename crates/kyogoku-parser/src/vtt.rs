//! WebVTT (Web Video Text Tracks) subtitle parser.
//!
//! Supports the WebVTT format used for HTML5 video subtitles.
//! Preserves cue timing, settings, and styling.

use anyhow::Result;
use serde_json::json;

use crate::block::TranslationBlock;
use crate::parser::Parser;

/// WebVTT subtitle parser.
pub struct VttParser;

/// Represents a WebVTT cue
#[derive(Debug, Clone)]
struct VttCue {
    identifier: Option<String>,
    start: String,
    end: String,
    settings: String,
    text: String,
}

impl VttCue {
    fn parse_from_lines(lines: &[&str]) -> Option<Self> {
        if lines.is_empty() {
            return None;
        }

        let mut iter = lines.iter();
        let first_line = *iter.next()?;

        // Check if first line is an identifier (doesn't contain "-->")
        let (identifier, timing_line) = if first_line.contains("-->") {
            (None, first_line)
        } else {
            let timing = *iter.next()?;
            if !timing.contains("-->") {
                return None;
            }
            (Some(first_line.to_string()), timing)
        };

        // Parse timing line: "00:00:01.000 --> 00:00:04.000 settings..."
        let arrow_pos = timing_line.find("-->")?;
        let start = timing_line[..arrow_pos].trim().to_string();
        let after_arrow = timing_line[arrow_pos + 3..].trim();

        // Settings come after end time
        let (end, settings) = if let Some(space_pos) = after_arrow.find(|c: char| c.is_whitespace())
        {
            (
                after_arrow[..space_pos].to_string(),
                after_arrow[space_pos..].trim().to_string(),
            )
        } else {
            (after_arrow.to_string(), String::new())
        };

        // Remaining lines are the cue text
        let text: Vec<&str> = iter.copied().collect();
        let text = text.join("\n");

        Some(Self {
            identifier,
            start,
            end,
            settings,
            text,
        })
    }
}

/// Strip VTT formatting tags from text.
fn strip_vtt_tags(text: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;

    for c in text.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(c);
        }
    }

    result
}

impl Parser for VttParser {
    fn extensions(&self) -> &[&str] {
        &["vtt", "webvtt"]
    }

    fn parse(&self, content: &str) -> Result<Vec<TranslationBlock>> {
        let mut blocks = Vec::new();
        let mut cue_lines: Vec<&str> = Vec::new();
        let mut cue_index = 0u32;
        let mut in_header = true;

        for line in content.lines() {
            // Skip WEBVTT header line
            if in_header && line.starts_with("WEBVTT") {
                in_header = false;
                continue;
            }

            // Empty line marks end of cue or continues header
            if line.trim().is_empty() {
                if !cue_lines.is_empty() {
                    if let Some(cue) = VttCue::parse_from_lines(&cue_lines) {
                        let plain_text = strip_vtt_tags(&cue.text);

                        if !plain_text.trim().is_empty() {
                            let block = TranslationBlock::new(&plain_text).with_metadata(json!({
                                "format": "vtt",
                                "index": cue_index,
                                "identifier": cue.identifier,
                                "start": cue.start,
                                "end": cue.end,
                                "settings": cue.settings,
                                "original_text": cue.text,
                            }));

                            blocks.push(block);
                            cue_index += 1;
                        }
                    }
                    cue_lines.clear();
                }
            } else {
                // Skip NOTE blocks and style definitions
                if line.starts_with("NOTE") || line.starts_with("STYLE") || line.contains("::") {
                    continue;
                }
                cue_lines.push(line);
            }
        }

        // Handle last cue if no trailing newline
        if !cue_lines.is_empty()
            && let Some(cue) = VttCue::parse_from_lines(&cue_lines)
        {
            let plain_text = strip_vtt_tags(&cue.text);

            if !plain_text.trim().is_empty() {
                let block = TranslationBlock::new(&plain_text).with_metadata(json!({
                    "format": "vtt",
                    "index": cue_index,
                    "identifier": cue.identifier,
                    "start": cue.start,
                    "end": cue.end,
                    "settings": cue.settings,
                    "original_text": cue.text,
                }));

                blocks.push(block);
            }
        }

        tracing::debug!("Parsed {} cues from VTT", blocks.len());
        Ok(blocks)
    }

    fn serialize(&self, blocks: &[TranslationBlock], _template: &str) -> Result<String> {
        let mut output = String::from("WEBVTT\n\n");

        for block in blocks {
            // Optional identifier
            if let Some(id) = block.metadata.get("identifier").and_then(|v| v.as_str())
                && !id.is_empty()
            {
                output.push_str(id);
                output.push('\n');
            }

            // Timing line
            let start = block
                .metadata
                .get("start")
                .and_then(|v| v.as_str())
                .unwrap_or("00:00:00.000");
            let end = block
                .metadata
                .get("end")
                .and_then(|v| v.as_str())
                .unwrap_or("00:00:10.000");
            let settings = block
                .metadata
                .get("settings")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            output.push_str(&format!("{} --> {}", start, end));
            if !settings.is_empty() {
                output.push(' ');
                output.push_str(settings);
            }
            output.push('\n');

            // Cue text
            output.push_str(block.output());
            output.push_str("\n\n");
        }

        Ok(output.trim_end().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_VTT: &str = r#"WEBVTT

1
00:00:01.000 --> 00:00:04.000
Hello, world!

2
00:00:05.000 --> 00:00:08.000 align:start
This is <b>styled</b> text.

00:00:09.000 --> 00:00:12.000
Line one
Line two
"#;

    #[test]
    fn test_strip_vtt_tags() {
        assert_eq!(strip_vtt_tags("plain text"), "plain text");
        assert_eq!(strip_vtt_tags("<b>bold</b>"), "bold");
        assert_eq!(strip_vtt_tags("<v Speaker>Hello</v>"), "Hello");
        assert_eq!(strip_vtt_tags("<c.yellow>colored</c>"), "colored");
    }

    #[test]
    fn test_vtt_parse() {
        let parser = VttParser;
        let blocks = parser.parse(SAMPLE_VTT).unwrap();

        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].source, "Hello, world!");
        assert_eq!(
            blocks[0]
                .metadata
                .get("identifier")
                .and_then(|v| v.as_str()),
            Some("1")
        );

        assert_eq!(blocks[1].source, "This is styled text.");
        assert_eq!(
            blocks[1].metadata.get("settings").and_then(|v| v.as_str()),
            Some("align:start")
        );

        assert!(blocks[2].source.contains("Line one"));
        assert!(blocks[2].source.contains("Line two"));
        assert!(
            blocks[2]
                .metadata
                .get("identifier")
                .and_then(|v| v.as_str())
                .is_none()
        );
    }

    #[test]
    fn test_vtt_serialize() {
        let blocks = vec![
            TranslationBlock::new("Hello")
                .with_metadata(json!({
                    "identifier": "1",
                    "start": "00:00:01.000",
                    "end": "00:00:04.000",
                    "settings": ""
                }))
                .with_target("你好"),
        ];

        let parser = VttParser;
        let output = parser.serialize(&blocks, "").unwrap();

        assert!(output.starts_with("WEBVTT"));
        assert!(output.contains("00:00:01.000 --> 00:00:04.000"));
        assert!(output.contains("你好"));
    }

    #[test]
    fn test_vtt_roundtrip() {
        let parser = VttParser;
        let blocks = parser.parse(SAMPLE_VTT).unwrap();

        let translated: Vec<_> = blocks
            .into_iter()
            .map(|b| {
                let source = b.source.clone();
                b.with_target(format!("[TR] {}", source))
            })
            .collect();

        let output = parser.serialize(&translated, SAMPLE_VTT).unwrap();

        let reparsed = parser.parse(&output).unwrap();
        assert_eq!(reparsed.len(), 3);
        assert!(reparsed[0].source.contains("[TR]"));
    }
}
