//! CSV Parser Plugin for Kyogoku
//!
//! This is an example WASM plugin that parses CSV files for translation.

use serde::{Deserialize, Serialize};
use std::alloc::{alloc as std_alloc, dealloc as std_dealloc, Layout};
use std::slice;

/// Translation block (matches kyogoku-parser::TranslationBlock)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationBlock {
    pub id: String,
    pub speaker: Option<String>,
    pub source: String,
    pub target: Option<String>,
    pub metadata: Option<String>,
}

// Global state for result buffer
static mut RESULT_BUF: Vec<u8> = Vec::new();

/// Allocate memory for the host to write into
#[no_mangle]
pub extern "C" fn alloc(size: i32) -> i32 {
    let layout = Layout::from_size_align(size as usize, 1).unwrap();
    unsafe { std_alloc(layout) as i32 }
}

/// Free previously allocated memory
#[no_mangle]
pub extern "C" fn dealloc(ptr: i32, size: i32) {
    let layout = Layout::from_size_align(size as usize, 1).unwrap();
    unsafe { std_dealloc(ptr as *mut u8, layout) }
}

/// Get the length of the last result
#[no_mangle]
pub extern "C" fn get_result_len() -> i32 {
    unsafe { RESULT_BUF.len() as i32 }
}

/// Parse CSV content into translation blocks
/// Returns pointer to JSON result
#[no_mangle]
pub extern "C" fn parse(ptr: i32, len: i32) -> i32 {
    let content = unsafe { slice::from_raw_parts(ptr as *const u8, len as usize) };
    
    let result = parse_csv(content);
    
    unsafe {
        RESULT_BUF = serde_json::to_vec(&result).unwrap_or_default();
        RESULT_BUF.as_ptr() as i32
    }
}

/// Serialize translation blocks back to CSV
#[no_mangle]
pub extern "C" fn serialize(
    blocks_ptr: i32,
    blocks_len: i32,
    template_ptr: i32,
    template_len: i32,
) -> i32 {
    let blocks_json = unsafe { slice::from_raw_parts(blocks_ptr as *const u8, blocks_len as usize) };
    let template = unsafe { slice::from_raw_parts(template_ptr as *const u8, template_len as usize) };
    
    let blocks: Vec<TranslationBlock> = serde_json::from_slice(blocks_json).unwrap_or_default();
    let result = serialize_csv(&blocks, template);
    
    unsafe {
        RESULT_BUF = result;
        RESULT_BUF.as_ptr() as i32
    }
}

/// Parse CSV content
fn parse_csv(content: &[u8]) -> Vec<TranslationBlock> {
    let text = String::from_utf8_lossy(content);
    let mut blocks = Vec::new();
    
    for (line_num, line) in text.lines().enumerate() {
        // Skip header row
        if line_num == 0 && line.to_lowercase().contains("key") {
            continue;
        }
        
        // Skip empty lines
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        
        // Parse CSV columns (simple split, doesn't handle quoted fields)
        let cols: Vec<&str> = line.split(',').collect();
        
        if cols.len() >= 2 {
            let key = cols[0].trim();
            let source = cols[1].trim();
            let context = cols.get(2).map(|s| s.trim().to_string());
            
            // Create block ID using blake3-like hash (simplified for WASM)
            let id = format!("{:016x}", simple_hash(source));
            
            let metadata = serde_json::json!({
                "key": key,
                "context": context,
                "line": line_num + 1,
            });
            
            blocks.push(TranslationBlock {
                id,
                speaker: None,
                source: source.to_string(),
                target: None,
                metadata: Some(metadata.to_string()),
            });
        }
    }
    
    blocks
}

/// Serialize blocks back to CSV
fn serialize_csv(blocks: &[TranslationBlock], template: &[u8]) -> Vec<u8> {
    let original = String::from_utf8_lossy(template);
    let mut output = String::new();
    
    // Preserve header
    let mut lines = original.lines();
    if let Some(header) = lines.next() {
        if header.to_lowercase().contains("key") {
            // Add "target" column if not present
            if !header.to_lowercase().contains("target") {
                output.push_str(header);
                output.push_str(",target\n");
            } else {
                output.push_str(header);
                output.push('\n');
            }
        }
    }
    
    // Build lookup map from blocks
    let block_map: std::collections::HashMap<String, &TranslationBlock> = blocks
        .iter()
        .filter_map(|b| {
            b.metadata.as_ref().and_then(|m| {
                serde_json::from_str::<serde_json::Value>(m)
                    .ok()
                    .and_then(|v| v["key"].as_str().map(|k| (k.to_string(), b)))
            })
        })
        .collect();
    
    // Reconstruct CSV with translations
    for line in lines {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            output.push_str(line);
            output.push('\n');
            continue;
        }
        
        let cols: Vec<&str> = line.split(',').collect();
        if cols.len() >= 2 {
            let key = cols[0].trim();
            
            // Find translation for this key
            let translation = block_map
                .get(key)
                .and_then(|b| b.target.as_deref())
                .unwrap_or("");
            
            // Output original columns plus translation
            output.push_str(line);
            output.push(',');
            output.push_str(translation);
            output.push('\n');
        } else {
            output.push_str(line);
            output.push('\n');
        }
    }
    
    output.into_bytes()
}

/// Simple hash function for ID generation
fn simple_hash(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325; // FNV offset basis
    for byte in s.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3); // FNV prime
    }
    hash
}
