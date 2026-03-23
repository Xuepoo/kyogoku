use anyhow::{Context, Result};
use serde_json::{Map, Value, json};

use crate::block::TranslationBlock;
use crate::parser::Parser;

/// JSON file parser - supports MTool format and simple key-value JSON.
pub struct JsonParser;

impl Parser for JsonParser {
    fn extensions(&self) -> &[&str] {
        &["json"]
    }

    fn parse(&self, content: &str) -> Result<Vec<TranslationBlock>> {
        let json: Value = serde_json::from_str(content).context("Failed to parse JSON")?;

        let blocks = match json {
            Value::Object(map) => parse_object(&map),
            Value::Array(arr) => parse_array(&arr),
            _ => Vec::new(),
        };

        tracing::debug!("Parsed {} blocks from JSON", blocks.len());
        Ok(blocks)
    }

    fn serialize(&self, blocks: &[TranslationBlock], template: &str) -> Result<String> {
        let json: Value =
            serde_json::from_str(template).context("Failed to parse template JSON")?;

        let output = match json {
            Value::Object(map) => write_object(map, blocks),
            Value::Array(arr) => write_array(arr, blocks),
            other => other,
        };

        serde_json::to_string_pretty(&output).context("Failed to serialize JSON")
    }
}

fn parse_object(map: &Map<String, Value>) -> Vec<TranslationBlock> {
    let mut blocks = Vec::new();

    for (key, value) in map {
        match value {
            Value::String(s) if !s.is_empty() => {
                blocks.push(
                    TranslationBlock::new(s.clone())
                        .with_metadata(json!({ "key": key, "format": "simple" })),
                );
            }
            Value::Object(nested) => {
                // MTool format: { "original": "source", "translation": "target" }
                if let Some(Value::String(source)) = nested.get("original").or(nested.get("source"))
                {
                    let mut block = TranslationBlock::new(source.clone())
                        .with_metadata(json!({ "key": key, "format": "mtool" }));

                    if let Some(Value::String(target)) =
                        nested.get("translation").or(nested.get("target"))
                        && !target.is_empty()
                    {
                        block = block.with_target(target.clone());
                    }
                    blocks.push(block);
                } else {
                    // Recursively parse nested objects
                    let nested_blocks = parse_object(nested);
                    blocks.extend(nested_blocks);
                }
            }
            Value::Array(arr) => {
                // Recursively parse nested arrays
                let arr_blocks = parse_array(arr);
                blocks.extend(arr_blocks);
            }
            _ => {}
        }
    }

    blocks
}

fn parse_array(arr: &[Value]) -> Vec<TranslationBlock> {
    let mut blocks = Vec::new();

    for (idx, value) in arr.iter().enumerate() {
        match value {
            Value::String(s) if !s.is_empty() => {
                blocks.push(
                    TranslationBlock::new(s.clone())
                        .with_metadata(json!({ "index": idx, "format": "array" })),
                );
            }
            Value::Object(obj) => {
                if let Some(Value::String(text)) = obj.get("text").or(obj.get("message")) {
                    let mut block = TranslationBlock::new(text.clone())
                        .with_metadata(json!({ "index": idx, "format": "dialogue" }));

                    if let Some(Value::String(name)) = obj.get("name").or(obj.get("speaker")) {
                        block = block.with_speaker(name.clone());
                    }
                    blocks.push(block);
                }
            }
            _ => {}
        }
    }

    blocks
}

fn write_object(mut map: Map<String, Value>, blocks: &[TranslationBlock]) -> Value {
    for block in blocks {
        let key = block.metadata.get("key").and_then(|v| v.as_str());
        let format = block.metadata.get("format").and_then(|v| v.as_str());

        if let Some(key) = key
            && let Some(value) = map.get_mut(key)
        {
            match format {
                Some("simple") => {
                    if matches!(value, Value::String(_)) {
                        *value = Value::String(block.output().to_string());
                    }
                }
                Some("mtool") => {
                    if let Value::Object(nested) = value {
                        if nested.contains_key("translation") {
                            nested.insert(
                                "translation".to_string(),
                                Value::String(block.output().to_string()),
                            );
                        } else if nested.contains_key("target") {
                            nested.insert(
                                "target".to_string(),
                                Value::String(block.output().to_string()),
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }
    Value::Object(map)
}

fn write_array(mut arr: Vec<Value>, blocks: &[TranslationBlock]) -> Value {
    for block in blocks {
        let idx = block
            .metadata
            .get("index")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);
        let format = block.metadata.get("format").and_then(|v| v.as_str());

        if let Some(idx) = idx
            && let Some(value) = arr.get_mut(idx)
        {
            match format {
                Some("array") => {
                    if matches!(value, Value::String(_)) {
                        *value = Value::String(block.output().to_string());
                    }
                }
                Some("dialogue") => {
                    if let Value::Object(obj) = value {
                        if obj.contains_key("text") {
                            obj.insert(
                                "text".to_string(),
                                Value::String(block.output().to_string()),
                            );
                        } else if obj.contains_key("message") {
                            obj.insert(
                                "message".to_string(),
                                Value::String(block.output().to_string()),
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }
    Value::Array(arr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_simple_object() {
        let content = r#"{ "hello": "Hello", "world": "World" }"#;
        let parser = JsonParser;
        let blocks = parser.parse(content).unwrap();

        assert_eq!(blocks.len(), 2);
    }

    #[test]
    fn test_json_mtool_format() {
        let content = r#"{
            "line1": { "original": "Hello", "translation": "" },
            "line2": { "original": "World", "translation": "世界" }
        }"#;

        let parser = JsonParser;
        let blocks = parser.parse(content).unwrap();

        assert_eq!(blocks.len(), 2);
        assert!(blocks.iter().any(|b| b.source == "Hello"));
        assert!(blocks.iter().any(|b| b.target == Some("世界".to_string())));
    }

    #[test]
    fn test_json_serialize() {
        let template = r#"{ "hello": "Hello" }"#;
        let mut blocks = JsonParser.parse(template).unwrap();
        blocks[0] = blocks[0].clone().with_target("你好");

        let output = JsonParser.serialize(&blocks, template).unwrap();
        assert!(output.contains("你好"));
    }
}
