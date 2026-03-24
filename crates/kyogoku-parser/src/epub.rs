use anyhow::{Context, Result};
use quick_xml::events::{BytesText, Event};
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;
use serde_json::json;
use std::collections::HashMap;
use std::io::{Cursor, Read, Write};
use zip::{ZipArchive, ZipWriter, write::FileOptions};

use crate::block::TranslationBlock;
use crate::parser::Parser;

/// EPUB file parser (Novel format).
/// Treats EPUB as a ZIP archive of XHTML files.
pub struct EpubParser;

impl Parser for EpubParser {
    fn extensions(&self) -> &[&str] {
        &["epub"]
    }

    fn parse(&self, content: &[u8]) -> Result<Vec<TranslationBlock>> {
        let cursor = Cursor::new(content);
        let mut archive = ZipArchive::new(cursor).context("Failed to open EPUB as ZIP")?;

        // 1. Locate OPF file
        let opf_path = find_opf_path(&mut archive)?;

        // 2. Parse OPF to get spine (reading order) and manifest
        let (manifest, spine) = parse_opf(&mut archive, &opf_path)?;

        let mut blocks = Vec::new();
        let mut global_index = 0;

        // 3. Iterate through spine items
        for item_id in spine {
            if let Some(href) = manifest.get(&item_id) {
                // Resolve href relative to OPF file location
                let file_path = resolve_path(&opf_path, href);
                println!("Processing file: {}", file_path);

                // Read file content
                let mut file = match archive.by_name(&file_path) {
                    Ok(f) => f,
                    Err(e) => {
                        println!("Failed to find file {}: {}", file_path, e);
                        continue;
                    }
                };
                let mut xhtml_content = String::new();
                file.read_to_string(&mut xhtml_content)?;

                // Parse XHTML
                let file_blocks = parse_xhtml(&xhtml_content, &file_path, &mut global_index)?;
                blocks.extend(file_blocks);
            }
        }

        tracing::debug!("Parsed {} blocks from EPUB", blocks.len());
        Ok(blocks)
    }

    fn serialize(&self, blocks: &[TranslationBlock], template: &[u8]) -> Result<Vec<u8>> {
        let cursor = Cursor::new(template);
        let mut archive = ZipArchive::new(cursor).context("Failed to open template EPUB")?;

        let output = Cursor::new(Vec::new());
        let mut zip = ZipWriter::new(output);

        // Map blocks by file path for faster lookup
        let mut blocks_by_file: HashMap<String, Vec<&TranslationBlock>> = HashMap::new();
        for block in blocks {
            if let Some(path) = block.metadata.get("file_path").and_then(|v| v.as_str()) {
                blocks_by_file
                    .entry(path.to_string())
                    .or_default()
                    .push(block);
            }
        }

        // Iterate through all files in the archive
        let file_names: Vec<String> = archive.file_names().map(|s| s.to_string()).collect();

        for file_name in file_names {
            let mut file = archive.by_name(&file_name)?;
            let options = FileOptions::default()
                .compression_method(file.compression())
                .unix_permissions(file.unix_mode().unwrap_or(0o644));

            let mut content = Vec::new();
            file.read_to_end(&mut content)?;

            // Check if this file needs modification
            if let Some(file_blocks) = blocks_by_file.get(&file_name) {
                // It's an XHTML file we parsed earlier
                let content_str = String::from_utf8(content)?;
                let modified_content = serialize_xhtml(&content_str, file_blocks)?;

                zip.start_file(&file_name, options)?;
                zip.write_all(modified_content.as_bytes())?;
            } else {
                // Just copy the file
                zip.start_file(&file_name, options)?;
                zip.write_all(&content)?;
            }
        }

        let output = zip.finish()?;
        Ok(output.into_inner())
    }
}

// Helper functions

fn find_opf_path(archive: &mut ZipArchive<Cursor<&[u8]>>) -> Result<String> {
    // Check META-INF/container.xml
    let mut container = archive
        .by_name("META-INF/container.xml")
        .context("Missing META-INF/container.xml")?;
    let mut content = String::new();
    container.read_to_string(&mut content)?;

    // Simple regex to find full-path attribute
    // <rootfile full-path="OEBPS/content.opf" ... />
    let re = regex::Regex::new(r#"full-path="([^"]+)""#).unwrap();
    if let Some(caps) = re.captures(&content) {
        return Ok(caps[1].to_string());
    }

    anyhow::bail!("Could not find OPF path in container.xml")
}

fn parse_opf(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    opf_path: &str,
) -> Result<(HashMap<String, String>, Vec<String>)> {
    let mut file = archive.by_name(opf_path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let mut manifest = HashMap::new();
    let mut spine = Vec::new();

    let mut reader = Reader::from_str(&content);
    reader.trim_text(true);

    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(e)) | Ok(Event::Start(e)) => {
                match e.name().as_ref() {
                    b"item" => {
                        // <item id="item1" href="chapter1.xhtml" ... />
                        let mut id = None;
                        let mut href = None;
                        for attr in e.attributes() {
                            let attr = attr?;
                            match attr.key.as_ref() {
                                b"id" => id = Some(attr.unescape_value()?.to_string()),
                                b"href" => href = Some(attr.unescape_value()?.to_string()),
                                _ => {}
                            }
                        }
                        if let (Some(id), Some(href)) = (id, href) {
                            manifest.insert(id, href);
                        }
                    }
                    b"itemref" => {
                        // <itemref idref="item1" />
                        for attr in e.attributes() {
                            let attr = attr?;
                            if attr.key.as_ref() == b"idref" {
                                spine.push(attr.unescape_value()?.to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => anyhow::bail!("Error parsing OPF: {}", e),
            _ => {}
        }
        buf.clear();
    }

    Ok((manifest, spine))
}

fn resolve_path(base: &str, relative: &str) -> String {
    // If base is "OEBPS/content.opf", and relative is "chapter1.xhtml", result is "OEBPS/chapter1.xhtml"
    let path = std::path::Path::new(base);
    if let Some(parent) = path.parent() {
        if parent.as_os_str().is_empty() {
            relative.to_string()
        } else {
            parent
                .join(relative)
                .to_str()
                .unwrap_or(relative)
                .to_string()
        }
    } else {
        relative.to_string()
    }
}

fn parse_xhtml(
    content: &str,
    file_path: &str,
    global_index: &mut u32,
) -> Result<Vec<TranslationBlock>> {
    let mut blocks = Vec::new();
    let mut reader = Reader::from_str(content);
    reader.trim_text(false);

    let mut buf = Vec::new();
    let mut in_p_tag = false;
    let mut current_text = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                // println!("Start tag: {:?}", std::str::from_utf8(e.name().as_ref()).unwrap());
                if e.name().as_ref() == b"p" {
                    in_p_tag = true;
                    current_text.clear();
                }
            }
            Ok(Event::End(ref e)) => {
                if e.name().as_ref() == b"p" {
                    in_p_tag = false;
                    let trimmed = current_text.trim();
                    if !trimmed.is_empty() {
                        let block = TranslationBlock::new(trimmed).with_metadata(json!({
                            "format": "epub",
                            "file_path": file_path,
                            "index": *global_index,
                        }));
                        blocks.push(block);
                        *global_index += 1;
                    }
                }
            }
            Ok(Event::Text(e)) => {
                if in_p_tag {
                    current_text.push_str(&e.unescape()?);
                }
            }
            Ok(Event::Eof) => break,
            Err(_e) => {
                // println!("Error at position {}: {:?}", reader.buffer_position(), e);
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(blocks)
}

fn serialize_xhtml(content: &str, blocks: &[&TranslationBlock]) -> Result<String> {
    // This is a simplified reconstruction
    // A proper implementation would need to match blocks to their original positions
    // robustly. Here we just replace <p> content in order.

    let mut block_iter = blocks.iter();

    let mut reader = Reader::from_str(content);
    reader.trim_text(false);

    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();

    loop {
        let event = reader.read_event_into(&mut buf);

        match event {
            Ok(Event::Start(e)) => {
                if e.name().as_ref() == b"p" {
                    writer.write_event(Event::Start(e.clone()))?;

                    // Consume text until End(p)
                    let mut p_text = String::new();
                    let mut inner_buf = Vec::new();
                    loop {
                        match reader.read_event_into(&mut inner_buf) {
                            Ok(Event::Text(t)) => p_text.push_str(&t.unescape()?),
                            Ok(Event::End(end)) if end.name().as_ref() == b"p" => {
                                // Found end of P
                                if !p_text.trim().is_empty() {
                                    if let Some(block) = block_iter.next() {
                                        writer.write_event(Event::Text(BytesText::new(
                                            &block.output(),
                                        )))?;
                                    } else {
                                        writer.write_event(Event::Text(BytesText::new(&p_text)))?;
                                    }
                                } else {
                                    writer.write_event(Event::Text(BytesText::new(&p_text)))?;
                                }
                                writer.write_event(Event::End(end))?;
                                break;
                            }
                            Ok(Event::Eof) => break,
                            Ok(_) => {
                                // Nested tags? Ignore for now or handle
                                // For simplicity in this mock, we skip nested tags inside p
                            }
                            Err(_) => break,
                        }
                        inner_buf.clear();
                    }
                } else {
                    writer.write_event(Event::Start(e))?;
                }
            }
            Ok(Event::End(e)) => {
                writer.write_event(Event::End(e))?;
            }
            Ok(Event::Text(e)) => {
                writer.write_event(Event::Text(e))?;
            }
            Ok(Event::Eof) => break,
            Ok(e) => {
                writer.write_event(e)?;
            }
            Err(e) => return Err(anyhow::anyhow!("XML error: {}", e)),
        }
        buf.clear();
    }

    let result = writer.into_inner().into_inner();
    Ok(String::from_utf8(result)?)
}
