use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;

use kyogoku_core::{Config, Glossary, TranslationCache, TranslationEngine};
use kyogoku_parser::ParserRegistry;

pub async fn run(
    input: PathBuf,
    output: Option<PathBuf>,
    from: Option<String>,
    to: Option<String>,
    glossary_path: Option<PathBuf>,
    no_cache: bool,
) -> Result<()> {
    // Load config
    let mut config = Config::load()?;

    // Override languages if specified
    if let Some(lang) = from {
        config.project.source_lang = lang;
    }
    if let Some(lang) = to {
        config.project.target_lang = lang;
    }

    // Setup output directory
    let output_dir = output.unwrap_or_else(|| PathBuf::from("output"));
    std::fs::create_dir_all(&output_dir).context("Failed to create output directory")?;

    // Initialize parser registry
    let registry = ParserRegistry::new();

    // Initialize engine
    let mut engine = TranslationEngine::new(config.clone())?;

    // Setup cache
    if !no_cache && let Ok(cache) = TranslationCache::open_default() {
        engine = engine.with_cache(cache);
        tracing::info!("Translation cache enabled");
    }

    // Load glossary
    let glossary_path = glossary_path.or(config.project.glossary_path.clone());
    if let Some(ref path) = glossary_path
        && path.exists()
    {
        let glossary = Glossary::load(path)?;
        engine = engine.with_glossary(glossary);
    }

    // Collect files to translate
    let files = collect_files(&input, &registry)?;

    if files.is_empty() {
        println!("No supported files found in {}", input.display());
        println!("Supported formats: {:?}", registry.supported_extensions());
        return Ok(());
    }

    println!("Found {} file(s) to translate", files.len());
    println!(
        "  {} → {}",
        config.project.source_lang, config.project.target_lang
    );

    // Process each file
    for file_path in &files {
        println!("\nProcessing: {}", file_path.display());

        // Read and parse file
        let content = std::fs::read(file_path)?;
        let parser = registry
            .get_parser(file_path)
            .ok_or_else(|| anyhow::anyhow!("No parser found for file: {}", file_path.display()))?;
        let mut blocks = parser.parse(&content)?;

        let needs_translation = blocks.iter().filter(|b| b.needs_translation()).count();
        println!(
            "  {} blocks ({} need translation)",
            blocks.len(),
            needs_translation
        );

        if needs_translation == 0 {
            println!("  Skipping (all translated)");
            continue;
        }

        // Setup progress bar
        let pb = ProgressBar::new(needs_translation as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("  [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
                .unwrap()
                .progress_chars("█▓░"),
        );

        // Translate
        engine
            .translate_blocks(&mut blocks, |completed, _total| {
                pb.set_position(completed as u64);
            })
            .await?;

        pb.finish_and_clear();

        // Get parser and serialize
        let parser = registry.get_parser(file_path).unwrap();
        let output_content = parser.serialize(&blocks, &content)?;

        // Write output
        let output_path = output_dir.join(file_path.file_name().unwrap());
        std::fs::write(&output_path, output_content)?;

        println!("  ✓ Output: {}", output_path.display());
    }

    println!("\n✓ Translation complete!");

    Ok(())
}

fn collect_files(input: &PathBuf, registry: &ParserRegistry) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    if input.is_file() {
        if registry.get_parser(input).is_some() {
            files.push(input.clone());
        }
    } else if input.is_dir() {
        for entry in walkdir(input)? {
            if registry.get_parser(&entry).is_some() {
                files.push(entry);
            }
        }
    }

    Ok(files)
}

fn walkdir(dir: &PathBuf) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            files.extend(walkdir(&path)?);
        } else {
            files.push(path);
        }
    }

    Ok(files)
}
