use kyogoku_core::{config::Config, TranslationEngine, TranslationCache, Glossary};
use kyogoku_parser::ParserRegistry;
use std::sync::Mutex;
use std::path::PathBuf;
use tauri::{State, Window, Emitter}; // Added Emitter, Removed Manager which was unused


struct AppState {
    config: Mutex<Config>,
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
    let mut engine = TranslationEngine::new(config.clone())
        .map_err(|e| e.to_string())?;

    // Enable Cache
    if let Ok(cache) = TranslationCache::open_default() {
        engine = engine.with_cache(cache);
    }

    // Load Glossary
    if let Some(ref path) = config.project.glossary_path {
        if path.exists() {
             let glossary = Glossary::load(path).map_err(|e| e.to_string())?;
             engine = engine.with_glossary(glossary);
        }
    }

    // Parse
    let content = std::fs::read(&path).map_err(|e| e.to_string())?;
    let registry = ParserRegistry::new();
    let parser = registry.get_parser(&path)
        .ok_or_else(|| format!("No parser found for file: {}", file_path))?;
    
    let mut blocks = parser.parse(&content).map_err(|e| e.to_string())?;
    let total_blocks = blocks.iter().filter(|b| b.needs_translation()).count();

    if total_blocks == 0 {
        return Ok("No translation needed".to_string());
    }

    window.emit("translation-start", total_blocks).map_err(|e| e.to_string())?;

    // Translate
    let window_clone = window.clone();
    engine.translate_blocks(&mut blocks, move |completed, total| {
        let _ = window_clone.emit("translation-progress", (completed, total));
    }).await.map_err(|e| e.to_string())?;

    // Serialize
    let output_content = parser.serialize(&blocks, &content).map_err(|e| e.to_string())?;

    // Write Output
    let file_stem = path.file_stem().unwrap().to_string_lossy();
    let extension = path.extension().unwrap_or_default().to_string_lossy();
    let output_filename = format!("{}_translated.{}", file_stem, extension);
    let output_path = path.with_file_name(output_filename);

    std::fs::write(&output_path, output_content).map_err(|e| e.to_string())?;

    let output_str = output_path.to_string_lossy().to_string();
    window.emit("translation-complete", &output_str).map_err(|e| e.to_string())?;

    Ok(output_str)
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
        })
        .invoke_handler(tauri::generate_handler![get_config, save_config, translate_file])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
