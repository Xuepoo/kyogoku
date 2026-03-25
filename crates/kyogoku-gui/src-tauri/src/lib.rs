use kyogoku_core::{Glossary, GlossaryEntry, TranslationCache, TranslationEngine, config::Config};

#[cfg(feature = "rag")]
use kyogoku_core::rag::{
    embeddings::EmbeddingModel,
    vectordb::{SimpleVectorStore, VectorStore},
};

use kyogoku_parser::ParserRegistry;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tauri::{Emitter, State, Window};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
enum FileStatus {
    Pending,
    Processing,
    Complete,
    Failed,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct FileQueueItem {
    id: String,
    file_path: String,
    file_name: String,
    status: FileStatus,
    word_count: Option<usize>,
    progress: f32,
    error_message: Option<String>,
}

#[derive(Default)]
struct FileQueue {
    items: Vec<FileQueueItem>,
    current_index: Option<usize>,
    batch_start_time: Option<std::time::Instant>,
    total_blocks_in_batch: usize,
    completed_blocks_in_batch: usize,
}

struct AppState {
    config: Mutex<Config>,
    file_queue: Arc<Mutex<FileQueue>>,
}

#[tauri::command]
fn get_config(state: State<AppState>) -> Result<Config, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(config.clone())
}

#[tauri::command]
fn save_config(state: State<AppState>, new_config: Config) -> Result<(), String> {
    let mut config = state.config.lock().map_err(|e| e.to_string())?;

    // Update memory state
    *config = new_config.clone();

    // Persist to disk
    config.save().map_err(|e| e.to_string())?;

    Ok(())
}

#[derive(serde::Serialize, Clone)]
struct TranslationProgressEvent {
    completed: usize,
    total: usize,
    source: String,
    target: String,
}

#[derive(serde::Serialize, Clone)]
struct BatchStatsEvent {
    total_files: usize,
    completed_files: usize,
    failed_files: usize,
    total_blocks: usize,
    completed_blocks: usize,
    elapsed_seconds: f64,
    estimated_remaining_seconds: Option<f64>,
}

#[tauri::command]
async fn translate_file(
    window: Window,
    state: State<'_, AppState>,
    file_path: String,
) -> Result<String, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();
    let path = PathBuf::from(&file_path);

    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    // Initialize Engine
    let mut engine = TranslationEngine::new(config.clone()).map_err(|e| e.to_string())?;

    // Enable Cache
    if let Ok(cache) = TranslationCache::open_default() {
        engine = engine.with_cache(cache);
    }

    // Load Glossary
    if let Some(ref path) = config.project.glossary_path
        && path.exists()
    {
        let glossary = Glossary::load(path).map_err(|e| e.to_string())?;
        engine = engine.with_glossary(glossary);
    }

    // Initialize RAG
    #[cfg(feature = "rag")]
    if config.rag.enabled {
        if let Some(ref model_path) = config.rag.model_path {
            if let Some(ref tokenizer_path) = config.rag.tokenizer_path {
                if let Some(ref vector_store_path) = config.rag.vector_store_path {
                    // Check if paths exist
                    if model_path.exists() && tokenizer_path.exists() {
                        // Load model
                        match EmbeddingModel::new(model_path, tokenizer_path) {
                            Ok(model) => {
                                let model = Arc::new(model);
                                // Load vector store
                                let mut store = SimpleVectorStore::new(vector_store_path);
                                if let Err(e) = store.load() {
                                    eprintln!(
                                        "Failed to load vector store (starting fresh): {}",
                                        e
                                    );
                                }
                                let store = Arc::new(Mutex::new(store));
                                engine = engine.with_rag(model, store);
                            }
                            Err(e) => {
                                eprintln!("Failed to load RAG model: {}", e);
                            }
                        }
                    }
                }
            }
        }
    }

    // Parse
    let content = std::fs::read(&path).map_err(|e| e.to_string())?;
    let registry = ParserRegistry::new();
    let parser = registry
        .get_parser(&path)
        .ok_or_else(|| format!("No parser found for file: {}", file_path))?;

    let mut blocks = parser.parse(&content).map_err(|e| e.to_string())?;
    let total_blocks = blocks.iter().filter(|b| b.needs_translation()).count();

    if total_blocks == 0 {
        return Ok("No translation needed".to_string());
    }

    window
        .emit("translation-start", total_blocks)
        .map_err(|e| e.to_string())?;

    // Translate
    let window_clone = window.clone();
    engine
        .translate_blocks(&mut blocks, move |completed, total, block| {
            let _ = window_clone.emit(
                "translation-progress",
                TranslationProgressEvent {
                    completed,
                    total,
                    source: block.source.clone(),
                    target: block.target.clone().unwrap_or_default(),
                },
            );
        })
        .await
        .map_err(|e| e.to_string())?;

    // Serialize
    let output_content = parser
        .serialize(&blocks, &content)
        .map_err(|e| e.to_string())?;

    // Write Output
    let file_stem = path.file_stem().unwrap().to_string_lossy();
    let extension = path.extension().unwrap_or_default().to_string_lossy();
    let output_filename = format!("{}_translated.{}", file_stem, extension);
    let output_path = path.with_file_name(output_filename);

    std::fs::write(&output_path, output_content).map_err(|e| e.to_string())?;

    let output_str = output_path.to_string_lossy().to_string();
    window
        .emit("translation-complete", &output_str)
        .map_err(|e| e.to_string())?;

    Ok(output_str)
}

// --- Batch File Processing Commands ---

#[tauri::command]
fn add_files_to_queue(
    state: State<AppState>,
    file_paths: Vec<String>,
) -> Result<Vec<FileQueueItem>, String> {
    let mut queue = state.file_queue.lock().map_err(|e| e.to_string())?;

    let new_items: Vec<FileQueueItem> = file_paths
        .into_iter()
        .map(|path| {
            let path_buf = PathBuf::from(&path);
            let file_name = path_buf
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            // Generate unique ID
            let id = format!(
                "{}-{}",
                file_name.replace('.', "_"),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis()
            );

            // Try to count words/blocks
            let word_count = count_words_in_file(&path);

            FileQueueItem {
                id,
                file_path: path,
                file_name,
                status: FileStatus::Pending,
                word_count,
                progress: 0.0,
                error_message: None,
            }
        })
        .collect();

    queue.items.extend(new_items.clone());
    Ok(new_items)
}

#[tauri::command]
fn get_file_queue(state: State<AppState>) -> Result<Vec<FileQueueItem>, String> {
    let queue = state.file_queue.lock().map_err(|e| e.to_string())?;
    Ok(queue.items.clone())
}

#[tauri::command]
fn remove_from_queue(state: State<AppState>, file_id: String) -> Result<(), String> {
    let mut queue = state.file_queue.lock().map_err(|e| e.to_string())?;
    queue.items.retain(|item| item.id != file_id);
    Ok(())
}

#[tauri::command]
fn clear_queue(state: State<AppState>) -> Result<(), String> {
    let mut queue = state.file_queue.lock().map_err(|e| e.to_string())?;
    queue.items.clear();
    queue.current_index = None;
    Ok(())
}

#[tauri::command]
fn reorder_queue(
    state: State<AppState>,
    file_id: String,
    new_index: usize,
) -> Result<Vec<FileQueueItem>, String> {
    let mut queue = state.file_queue.lock().map_err(|e| e.to_string())?;

    if let Some(old_index) = queue.items.iter().position(|item| item.id == file_id) {
        let item = queue.items.remove(old_index);
        let new_index = new_index.min(queue.items.len());
        queue.items.insert(new_index, item);
    }

    Ok(queue.items.clone())
}

#[tauri::command]
async fn start_batch_translation(
    window: Window,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let _config = state.config.lock().map_err(|e| e.to_string())?.clone();

    // Get all pending files and calculate total blocks
    let pending_files: Vec<FileQueueItem> = {
        let mut queue = state.file_queue.lock().map_err(|e| e.to_string())?;
        queue.batch_start_time = Some(std::time::Instant::now());
        queue.total_blocks_in_batch = queue
            .items
            .iter()
            .filter(|item| matches!(item.status, FileStatus::Pending))
            .filter_map(|item| item.word_count)
            .sum();
        queue.completed_blocks_in_batch = 0;

        queue
            .items
            .iter()
            .filter(|item| matches!(item.status, FileStatus::Pending))
            .cloned()
            .collect()
    };

    if pending_files.is_empty() {
        return Err("No pending files in queue".to_string());
    }

    let total_files = pending_files.len();

    window
        .emit("batch-started", total_files)
        .map_err(|e| e.to_string())?;

    let mut completed_count = 0;
    let mut failed_count = 0;

    for file_item in pending_files {
        // Update status to processing
        {
            let mut queue = state.file_queue.lock().map_err(|e| e.to_string())?;
            if let Some(item) = queue.items.iter_mut().find(|i| i.id == file_item.id) {
                item.status = FileStatus::Processing;
                item.progress = 0.0;
            }
        }

        window
            .emit("file-processing", &file_item)
            .map_err(|e| e.to_string())?;

        // Emit batch stats
        emit_batch_stats(&window, &state, total_files, completed_count, failed_count)?;

        // Translate the file
        match translate_single_file(
            window.clone(),
            state.clone(),
            &file_item.file_path,
            &file_item.id,
        )
        .await
        {
            Ok(output_path) => {
                completed_count += 1;

                // Update status to complete
                {
                    let mut queue = state.file_queue.lock().map_err(|e| e.to_string())?;
                    if let Some(item) = queue.items.iter_mut().find(|i| i.id == file_item.id) {
                        item.status = FileStatus::Complete;
                        item.progress = 100.0;
                    }
                }

                window
                    .emit("file-complete", (&file_item.id, &output_path))
                    .map_err(|e| e.to_string())?;
            }
            Err(e) => {
                failed_count += 1;

                // Update status to failed
                {
                    let mut queue = state.file_queue.lock().map_err(|e| e.to_string())?;
                    if let Some(item) = queue.items.iter_mut().find(|i| i.id == file_item.id) {
                        item.status = FileStatus::Failed;
                        item.error_message = Some(e.clone());
                    }
                }

                window
                    .emit("file-failed", (&file_item.id, &e))
                    .map_err(|e| e.to_string())?;
            }
        }

        // Emit updated batch stats
        emit_batch_stats(&window, &state, total_files, completed_count, failed_count)?;
    }

    window
        .emit(
            "batch-complete",
            format!("{} completed, {} failed", completed_count, failed_count),
        )
        .map_err(|e| e.to_string())?;

    Ok(format!(
        "Batch translation complete: {} succeeded, {} failed",
        completed_count, failed_count
    ))
}

fn emit_batch_stats(
    window: &Window,
    state: &State<AppState>,
    total_files: usize,
    completed_files: usize,
    failed_files: usize,
) -> Result<(), String> {
    let queue = state.file_queue.lock().map_err(|e| e.to_string())?;

    let elapsed = queue
        .batch_start_time
        .map(|t| t.elapsed().as_secs_f64())
        .unwrap_or(0.0);

    let estimated_remaining = if completed_files > 0 && queue.completed_blocks_in_batch > 0 {
        let blocks_per_second = queue.completed_blocks_in_batch as f64 / elapsed;
        let remaining_blocks = queue
            .total_blocks_in_batch
            .saturating_sub(queue.completed_blocks_in_batch);
        Some(remaining_blocks as f64 / blocks_per_second.max(0.001))
    } else {
        None
    };

    let stats = BatchStatsEvent {
        total_files,
        completed_files,
        failed_files,
        total_blocks: queue.total_blocks_in_batch,
        completed_blocks: queue.completed_blocks_in_batch,
        elapsed_seconds: elapsed,
        estimated_remaining_seconds: estimated_remaining,
    };

    window
        .emit("batch-stats", &stats)
        .map_err(|e| e.to_string())
}

// Helper function to count words (simplified)
fn count_words_in_file(path: &str) -> Option<usize> {
    let path_buf = PathBuf::from(path);
    if !path_buf.exists() {
        return None;
    }

    let content = std::fs::read(&path_buf).ok()?;
    let registry = ParserRegistry::new();
    let parser = registry.get_parser(&path_buf)?;
    let blocks = parser.parse(&content).ok()?;

    Some(blocks.iter().filter(|b| b.needs_translation()).count())
}

// Helper function for translating a single file (extracted from translate_file)
async fn translate_single_file(
    window: Window,
    state: State<'_, AppState>,
    file_path: &str,
    file_id: &str,
) -> Result<String, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();
    let path = PathBuf::from(file_path);

    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    // Initialize Engine
    let mut engine = TranslationEngine::new(config.clone()).map_err(|e| e.to_string())?;

    // Enable Cache
    if let Ok(cache) = TranslationCache::open_default() {
        engine = engine.with_cache(cache);
    }

    // Load Glossary
    if let Some(ref path) = config.project.glossary_path
        && path.exists()
    {
        let glossary = Glossary::load(path).map_err(|e| e.to_string())?;
        engine = engine.with_glossary(glossary);
    }

    // Initialize RAG
    #[cfg(feature = "rag")]
    if config.rag.enabled {
        if let Some(ref model_path) = config.rag.model_path {
            if let Some(ref tokenizer_path) = config.rag.tokenizer_path {
                if let Some(ref vector_store_path) = config.rag.vector_store_path {
                    if model_path.exists() && tokenizer_path.exists() {
                        match EmbeddingModel::new(model_path, tokenizer_path) {
                            Ok(model) => {
                                let model = Arc::new(model);
                                let mut store = SimpleVectorStore::new(vector_store_path);
                                if let Err(e) = store.load() {
                                    eprintln!(
                                        "Failed to load vector store (starting fresh): {}",
                                        e
                                    );
                                }
                                let store = Arc::new(Mutex::new(store));
                                engine = engine.with_rag(model, store);
                            }
                            Err(e) => {
                                eprintln!("Failed to load RAG model: {}", e);
                            }
                        }
                    }
                }
            }
        }
    }

    // Parse
    let content = std::fs::read(&path).map_err(|e| e.to_string())?;
    let registry = ParserRegistry::new();
    let parser = registry
        .get_parser(&path)
        .ok_or_else(|| format!("No parser found for file: {}", file_path))?;

    let mut blocks = parser.parse(&content).map_err(|e| e.to_string())?;
    let total_blocks = blocks.iter().filter(|b| b.needs_translation()).count();

    if total_blocks == 0 {
        return Err("No translation needed".to_string());
    }

    // Translate with progress updates
    let window_clone = window.clone();
    let file_id_clone = file_id.to_string();
    let state_clone = state.clone();

    engine
        .translate_blocks(&mut blocks, move |completed, total, block| {
            let progress = (completed as f32 / total as f32) * 100.0;

            // Update queue item progress and batch stats
            if let Ok(mut queue) = state_clone.file_queue.lock() {
                if let Some(item) = queue.items.iter_mut().find(|i| i.id == file_id_clone) {
                    item.progress = progress;
                }
                // Update completed blocks in batch
                queue.completed_blocks_in_batch += 1;
            }

            let _ = window_clone.emit(
                "translation-progress",
                TranslationProgressEvent {
                    completed,
                    total,
                    source: block.source.clone(),
                    target: block.target.clone().unwrap_or_default(),
                },
            );
        })
        .await
        .map_err(|e| e.to_string())?;

    // Serialize
    let output_content = parser
        .serialize(&blocks, &content)
        .map_err(|e| e.to_string())?;

    // Write Output
    let file_stem = path.file_stem().unwrap().to_string_lossy();
    let extension = path.extension().unwrap_or_default().to_string_lossy();
    let output_filename = format!("{}_translated.{}", file_stem, extension);
    let output_path = path.with_file_name(output_filename);

    std::fs::write(&output_path, output_content).map_err(|e| e.to_string())?;

    Ok(output_path.to_string_lossy().to_string())
}

// --- I18n Commands ---

#[tauri::command]
fn get_available_locales() -> Result<Vec<String>, String> {
    Ok(vec![
        "en-US".to_string(),
        "zh-CN".to_string(),
        "ja-JP".to_string(),
    ])
}

#[tauri::command]
fn get_current_locale() -> Result<String, String> {
    Ok(kyogoku_i18n::get_locale())
}

#[tauri::command]
fn set_locale(locale: String) -> Result<(), String> {
    kyogoku_i18n::set_locale(&locale);
    Ok(())
}

#[tauri::command]
fn translate_text(key: String) -> Result<String, String> {
    Ok(kyogoku_i18n::translate(&key))
}

#[tauri::command]
fn get_glossary(state: State<AppState>) -> Result<Vec<GlossaryEntry>, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;

    if let Some(ref path) = config.project.glossary_path
        && path.exists()
    {
        let glossary = Glossary::load(path).map_err(|e| e.to_string())?;
        return Ok(glossary.entries().into_iter().cloned().collect());
    }
    Ok(vec![])
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load config on startup or use default
    let config = Config::load().unwrap_or_else(|e| {
        eprintln!("Failed to load config: {}", e);
        Config::default()
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            config: Mutex::new(config),
            file_queue: Arc::new(Mutex::new(FileQueue::default())),
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            translate_file,
            add_files_to_queue,
            get_file_queue,
            remove_from_queue,
            clear_queue,
            reorder_queue,
            start_batch_translation,
            get_available_locales,
            get_current_locale,
            set_locale,
            translate_text,
            get_glossary,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
