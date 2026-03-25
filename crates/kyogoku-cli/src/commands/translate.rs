use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::Instant;

use kyogoku_core::{Config, Glossary, TranslationCache, TranslationEngine};
use kyogoku_parser::ParserRegistry;

/// Translation command options
pub struct TranslateOpts {
    pub input: PathBuf,
    pub output: Option<PathBuf>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub glossary_path: Option<PathBuf>,
    pub no_cache: bool,
    pub dry_run: bool,
    pub format: Option<String>,
    pub json_output: bool,
}

#[derive(Serialize)]
struct TranslationResult {
    success: bool,
    files_processed: usize,
    blocks_translated: usize,
    blocks_cached: usize,
    elapsed_seconds: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    files: Vec<FileResult>,
}

#[derive(Serialize)]
struct FileResult {
    path: String,
    blocks_total: usize,
    blocks_translated: usize,
    output_path: Option<String>,
}

pub async fn run(opts: TranslateOpts) -> Result<()> {
    let start_time = Instant::now();
    let mut result = TranslationResult {
        success: true,
        files_processed: 0,
        blocks_translated: 0,
        blocks_cached: 0,
        elapsed_seconds: 0.0,
        error: None,
        files: Vec::new(),
    };

    match run_inner(&opts, &mut result).await {
        Ok(()) => {
            result.elapsed_seconds = start_time.elapsed().as_secs_f64();
            if opts.json_output {
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            Ok(())
        }
        Err(e) => {
            result.success = false;
            result.error = Some(e.to_string());
            result.elapsed_seconds = start_time.elapsed().as_secs_f64();
            if opts.json_output {
                println!("{}", serde_json::to_string_pretty(&result)?);
                Ok(())
            } else {
                Err(e)
            }
        }
    }
}

async fn run_inner(opts: &TranslateOpts, result: &mut TranslationResult) -> Result<()> {
    // Load config
    let mut config = Config::load()?;

    // Override languages if specified
    if let Some(ref lang) = opts.from {
        config.project.source_lang = lang.clone();
    }
    if let Some(ref lang) = opts.to {
        config.project.target_lang = lang.clone();
    }

    // Setup output directory
    let output_dir = opts
        .output
        .clone()
        .unwrap_or_else(|| PathBuf::from("output"));
    if !opts.dry_run {
        std::fs::create_dir_all(&output_dir).context("Failed to create output directory")?;
    }

    // Initialize parser registry
    let registry = ParserRegistry::new();

    // Initialize engine (skip if dry run)
    let engine = if opts.dry_run {
        None
    } else {
        let mut eng = TranslationEngine::new(config.clone())?;

        // Setup cache
        if !opts.no_cache
            && let Ok(cache) = TranslationCache::open_default()
        {
            eng = eng.with_cache(cache);
            if !opts.json_output {
                tracing::info!("Translation cache enabled");
            }
        }

        // Load glossary
        let glossary_path = opts
            .glossary_path
            .clone()
            .or(config.project.glossary_path.clone());
        if let Some(ref path) = glossary_path
            && path.exists()
        {
            let glossary = Glossary::load(path)?;
            eng = eng.with_glossary(glossary);
        }

        Some(eng)
    };

    // Collect files to translate
    let files = collect_files(&opts.input, &registry, opts.format.as_deref())?;

    if files.is_empty() {
        if !opts.json_output {
            println!("No supported files found in {}", opts.input.display());
            println!("Supported formats: {:?}", registry.supported_extensions());
        }
        return Ok(());
    }

    if !opts.json_output {
        if opts.dry_run {
            println!("🔍 DRY RUN - No API calls will be made\n");
        }
        println!("Found {} file(s) to translate", files.len());
        println!(
            "  {} → {}",
            config.project.source_lang, config.project.target_lang
        );
    }

    // Process each file
    for file_path in &files {
        if !opts.json_output {
            println!("\nProcessing: {}", file_path.display());
        }

        // Read and parse file
        let content = std::fs::read(file_path)?;
        let parser = get_parser(&registry, file_path, opts.format.as_deref())?;
        let mut blocks = parser.parse(&content)?;

        let needs_translation = blocks.iter().filter(|b| b.needs_translation()).count();
        let mut file_result = FileResult {
            path: file_path.display().to_string(),
            blocks_total: blocks.len(),
            blocks_translated: 0,
            output_path: None,
        };

        if !opts.json_output {
            println!(
                "  {} blocks ({} need translation)",
                blocks.len(),
                needs_translation
            );
        }

        if needs_translation == 0 {
            if !opts.json_output {
                println!("  Skipping (all translated)");
            }
            result.files.push(file_result);
            continue;
        }

        if opts.dry_run {
            // In dry run mode, just show what would be translated
            if !opts.json_output {
                println!("  Would translate {} blocks:", needs_translation);
                for (i, block) in blocks
                    .iter()
                    .filter(|b| b.needs_translation())
                    .take(5)
                    .enumerate()
                {
                    let preview = if block.source.len() > 60 {
                        format!("{}...", &block.source[..60])
                    } else {
                        block.source.clone()
                    };
                    println!("    {}. {}", i + 1, preview);
                }
                if needs_translation > 5 {
                    println!("    ... and {} more", needs_translation - 5);
                }
            }
            file_result.blocks_translated = needs_translation;
            result.files.push(file_result);
            result.files_processed += 1;
            continue;
        }

        // Setup progress bar
        let pb = if opts.json_output {
            ProgressBar::hidden()
        } else {
            let pb = ProgressBar::new(needs_translation as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("  [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
                    .unwrap()
                    .progress_chars("█▓░"),
            );
            pb
        };

        // Track cached vs translated
        let mut cached_count = 0;

        // Translate
        if let Some(ref engine) = engine {
            engine
                .translate_blocks(&mut blocks, |completed, _total, block| {
                    pb.set_position(completed as u64);
                    // Check if this was a cache hit (target already set before translation)
                    if block.target.is_some() {
                        cached_count += 1;
                    }
                })
                .await?;
        }

        pb.finish_and_clear();

        // Get parser and serialize
        let parser = get_parser(&registry, file_path, opts.format.as_deref())?;
        let output_content = parser.serialize(&blocks, &content)?;

        // Write output
        let output_path = output_dir.join(file_path.file_name().unwrap());
        std::fs::write(&output_path, output_content)?;

        file_result.blocks_translated = needs_translation;
        file_result.output_path = Some(output_path.display().to_string());
        result.blocks_translated += needs_translation;
        result.blocks_cached += cached_count;
        result.files_processed += 1;
        result.files.push(file_result);

        if !opts.json_output {
            println!("  ✓ Output: {}", output_path.display());
        }
    }

    if !opts.json_output {
        if opts.dry_run {
            println!("\n🔍 Dry run complete - no changes made");
        } else {
            println!("\n✓ Translation complete!");
        }
    }

    Ok(())
}

fn get_parser<'a>(
    registry: &'a ParserRegistry,
    file_path: &Path,
    format: Option<&str>,
) -> Result<&'a dyn kyogoku_parser::Parser> {
    if let Some(fmt) = format {
        registry
            .get_parser_by_extension(fmt)
            .ok_or_else(|| anyhow::anyhow!("Unknown format: {}", fmt))
    } else {
        registry
            .get_parser(file_path)
            .ok_or_else(|| anyhow::anyhow!("No parser found for file: {}", file_path.display()))
    }
}

fn collect_files(
    input: &Path,
    registry: &ParserRegistry,
    format: Option<&str>,
) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    if input.is_file() {
        // If format is specified, use that; otherwise check by extension
        if format.is_some() || registry.get_parser(input).is_some() {
            files.push(input.to_path_buf());
        }
    } else if input.is_dir() {
        for entry in walkdir(input)? {
            if format.is_some() || registry.get_parser(&entry).is_some() {
                files.push(entry);
            }
        }
    }

    Ok(files)
}

fn walkdir(dir: &Path) -> Result<Vec<PathBuf>> {
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
