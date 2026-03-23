use kyogoku_parser::ParserRegistry;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    let registry = ParserRegistry::new();
    
    println!("=== Supported Extensions ===");
    println!("{:?}\n", registry.supported_extensions());
    
    let test_files = [
        "/tmp/kyogoku-test/test.txt",
        "/tmp/kyogoku-test/test.ass",
        "/tmp/kyogoku-test/test.vtt",
    ];
    
    for file_path in test_files {
        let path = Path::new(file_path);
        println!("=== Testing: {} ===", path.display());
        
        if !path.exists() {
            println!("File not found!\n");
            continue;
        }
        
        match registry.parse_file(path) {
            Ok(blocks) => {
                println!("✓ Parsed {} blocks", blocks.len());
                for (i, block) in blocks.iter().enumerate() {
                    let preview = if block.source.chars().count() > 30 {
                        let s: String = block.source.chars().take(30).collect();
                        format!("{}...", s)
                    } else {
                        block.source.clone()
                    };
                    println!("  Block {}: {}", i + 1, preview.replace('\n', "\\n"));
                    if let Some(speaker) = &block.speaker {
                        println!("    Speaker: {}", speaker);
                    }
                }
                
                // Test serialization
                let template = std::fs::read_to_string(path)?;
                if let Some(parser) = registry.get_parser(path) {
                    match parser.serialize(&blocks, &template) {
                        Ok(output) => {
                            println!("  ✓ Serialization: OK ({} bytes)", output.len());
                        }
                        Err(e) => {
                            println!("  ✗ Serialization failed: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                println!("✗ Parse failed: {}", e);
            }
        }
        println!();
    }
    
    Ok(())
}
