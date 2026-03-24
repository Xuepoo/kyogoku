import { invoke } from "@tauri-apps/api/core";
import { open } from '@tauri-apps/plugin-dialog';
import { listen } from '@tauri-apps/api/event';

// --- Types ---

interface ApiConfig {
    provider: string;
    api_key: string | null;
    api_base?: string;
    model: string;
    max_tokens: number;
    temperature: number;
}

interface TranslationConfig {
    style: string;
    context_size: number;
}

interface AdvancedConfig {
    max_concurrency: number;
    batch_size: number;
    allocator?: string | null;
}

interface ProjectConfig {
    source_lang: string;
    target_lang: string;
    glossary_path?: string | null;
    input_dir?: string | null;
    output_dir?: string | null;
}

interface RagConfig {
    enabled: boolean;
    model_path?: string | null;
    tokenizer_path?: string | null;
    vector_store_path?: string | null;
}

interface Config {
    api: ApiConfig;
    translation: TranslationConfig;
    advanced: AdvancedConfig;
    project: ProjectConfig;
    rag: RagConfig;
}

// --- DOM Elements ---

const apiProvider = document.querySelector("#api-provider") as HTMLSelectElement;
const apiBase = document.querySelector("#api-base") as HTMLInputElement;
const apiKey = document.querySelector("#api-key") as HTMLInputElement;
const apiModel = document.querySelector("#api-model") as HTMLInputElement;
const translationStyle = document.querySelector("#translation-style") as HTMLSelectElement;
const contextSize = document.querySelector("#context-size") as HTMLInputElement;
const maxConcurrency = document.querySelector("#max-concurrency") as HTMLInputElement;
const batchSize = document.querySelector("#batch-size") as HTMLInputElement;

// RAG Elements
const ragEnabled = document.querySelector("#rag-enabled") as HTMLInputElement;
const ragModelPath = document.querySelector("#rag-model-path") as HTMLInputElement;
const ragTokenizerPath = document.querySelector("#rag-tokenizer-path") as HTMLInputElement;
const ragStorePath = document.querySelector("#rag-store-path") as HTMLInputElement;
const ragSettings = document.querySelector("#rag-settings") as HTMLElement;

const configForm = document.querySelector("#config-form") as HTMLFormElement;
const reloadBtn = document.querySelector("#reload-btn") as HTMLButtonElement;
const statusMsg = document.querySelector("#status-msg") as HTMLElement;
const statusBadge = document.querySelector("#status-badge") as HTMLElement;

let currentConfig: Config | null = null;

// --- Logic ---

async function loadConfig() {
    try {
        setStatus("Loading configuration...");
        const config = await invoke<Config>("get_config");
        currentConfig = config;
        
        console.log("Config loaded:", config);

        // Populate form
        if (apiProvider) apiProvider.value = config.api.provider;
        if (apiKey) apiKey.value = config.api.api_key || "";
        if (apiBase) apiBase.value = config.api.api_base || "";
        if (apiModel) apiModel.value = config.api.model;
        if (translationStyle) translationStyle.value = config.translation.style;
        if (contextSize) contextSize.value = config.translation.context_size.toString();
        if (maxConcurrency) maxConcurrency.value = config.advanced.max_concurrency.toString();
        if (batchSize) batchSize.value = (config.advanced.batch_size || 5).toString();

        // RAG Config
        if (ragEnabled) {
            ragEnabled.checked = config.rag.enabled;
            // Toggle visibility on load
            if (config.rag.enabled && ragSettings) {
                ragSettings.classList.remove("hidden");
            } else if (ragSettings) {
                ragSettings.classList.add("hidden");
            }
        }
        if (ragModelPath && config.rag.model_path) {
            ragModelPath.value = config.rag.model_path;
        }
        if (ragTokenizerPath && config.rag.tokenizer_path) {
            ragTokenizerPath.value = config.rag.tokenizer_path;
        }
        if (ragStorePath && config.rag.vector_store_path) {
            ragStorePath.value = config.rag.vector_store_path;
        }

        setStatus("Configuration loaded", "success");
        setTimeout(clearStatus, 2000);
    } catch (e) {
        setStatus(`Failed to load config: ${e}`, "error");
        console.error(e);
    }
}

async function saveConfig() {
    if (!currentConfig) return;

    // Create updated config object
    // Note: We need to carefully reconstruct the nested structure to match Rust struct
    const newConfig: Config = {
        api: {
            ...currentConfig.api,
            provider: apiProvider.value,
            api_key: apiKey.value || null,
            api_base: apiBase.value || undefined,
            model: apiModel.value,
        },
        translation: {
            ...currentConfig.translation,
            style: translationStyle.value,
            context_size: parseInt(contextSize.value, 10),
        },
        advanced: {
            ...currentConfig.advanced,
            max_concurrency: parseInt(maxConcurrency.value, 10),
            batch_size: parseInt(batchSize.value, 10),
        },
        project: currentConfig.project, // Keep existing project config
        rag: {
            ...currentConfig.rag,
            enabled: ragEnabled ? ragEnabled.checked : false,
            model_path: ragModelPath ? ragModelPath.value || null : null,
            tokenizer_path: ragTokenizerPath ? ragTokenizerPath.value || null : null,
            vector_store_path: ragStorePath ? ragStorePath.value || null : null,
        }
    };

    try {
        setStatus("Saving...");
        await invoke("save_config", { newConfig });
        currentConfig = newConfig;
        
        setStatus("Configuration saved successfully", "success");
        showSavedBadge();
        setTimeout(clearStatus, 3000);
    } catch (e) {
        setStatus(`Failed to save config: ${e}`, "error");
        console.error(e);
    }
}

// --- Helpers ---

function setStatus(msg: string, type: "info" | "success" | "error" = "info") {
    if (!statusMsg) return;
    statusMsg.textContent = msg;
    
    statusMsg.className = "text-xs text-center h-4 transition-colors duration-300";
    if (type === "error") statusMsg.classList.add("text-red-400");
    else if (type === "success") statusMsg.classList.add("text-green-400");
    else statusMsg.classList.add("text-gray-400");
}

function clearStatus() {
    if (statusMsg) statusMsg.textContent = "";
}

function showSavedBadge() {
    if (statusBadge) {
        statusBadge.classList.remove("hidden");
        statusBadge.classList.add("bg-green-500/20", "text-green-400", "border", "border-green-500/30");
        setTimeout(() => {
            statusBadge.classList.add("hidden");
        }, 2000);
    }
}

const dropZone = document.querySelector("#drop-zone") as HTMLElement;
const previewSection = document.querySelector("#preview-section") as HTMLElement;
const previewContainer = document.querySelector("#preview-container") as HTMLElement;
const previewStatus = document.querySelector("#preview-status") as HTMLElement;
const recentActivity = document.querySelector("#recent-activity") as HTMLElement;

// --- Recent Activity (localStorage) ---

interface TranslationHistoryItem {
    filename: string;
    timestamp: number;
    blocksCount: number;
}

function getHistory(): TranslationHistoryItem[] {
    try {
        const data = localStorage.getItem("kyogoku_history");
        return data ? JSON.parse(data) : [];
    } catch {
        return [];
    }
}

function addToHistory(filename: string, blocksCount: number) {
    const history = getHistory();
    history.unshift({ filename, timestamp: Date.now(), blocksCount });
    // Keep only last 10
    if (history.length > 10) history.pop();
    localStorage.setItem("kyogoku_history", JSON.stringify(history));
}

function renderRecentActivity() {
    if (!recentActivity) return;
    const history = getHistory();
    
    if (history.length === 0) {
        recentActivity.innerHTML = `
            <h3 class="font-bold text-gray-400 text-sm uppercase tracking-wider mb-3">Recent Activity</h3>
            <div class="bg-gray-900/50 rounded p-4 text-center">
                <p class="text-sm text-gray-500 italic">No recent tasks found.</p>
            </div>
        `;
        return;
    }
    
    const items = history.map(item => {
        const date = new Date(item.timestamp);
        const timeStr = date.toLocaleString();
        return `
            <div class="flex justify-between items-center p-2 bg-gray-900/30 rounded text-sm">
                <span class="text-gray-300 truncate max-w-xs">${item.filename}</span>
                <span class="text-gray-500 text-xs">${timeStr}</span>
            </div>
        `;
    }).join("");
    
    recentActivity.innerHTML = `
        <h3 class="font-bold text-gray-400 text-sm uppercase tracking-wider mb-3">Recent Activity</h3>
        <div class="space-y-2">${items}</div>
    `;
}


// --- Event Listeners ---

window.addEventListener("DOMContentLoaded", async () => {
    loadConfig();

    if (configForm) {
        configForm.addEventListener("submit", (e) => {
            e.preventDefault();
            saveConfig();
        });
    }

    if (reloadBtn) {
        reloadBtn.addEventListener("click", () => {
            loadConfig();
        });
    }

    if (ragEnabled && ragSettings) {
        ragEnabled.addEventListener("change", () => {
            if (ragEnabled.checked) {
                ragSettings.classList.remove("hidden");
            } else {
                ragSettings.classList.add("hidden");
            }
        });
    }

    renderRecentActivity();

    if (dropZone) {
        dropZone.addEventListener("click", async () => {
            try {
                const selected = await open({
                    multiple: false,
                    filters: [{
                        name: 'Supported Files',
                        extensions: ['rpy', 'ass', 'srt', 'vtt', 'epub', 'txt', 'json']
                    }]
                });

                if (selected === null) return;
                
                const filePath = Array.isArray(selected) ? selected[0] : selected;
                startTranslation(filePath);
            } catch (e) {
                console.error("Failed to open file dialog", e);
            }
        });
    }

    // Listen for file drops
    await listen('tauri://file-drop', (event) => {
        const files = event.payload as string[];
        if (files && files.length > 0) {
            startTranslation(files[0]);
        }
    });

    // Listen for translation progress
    await listen('translation-start', (event) => {
        const total = event.payload as number;
        setStatus(`Translation started: 0/${total} blocks...`);
        
        if (previewSection) previewSection.classList.remove("hidden");
        if (recentActivity) recentActivity.classList.add("hidden");
        if (previewContainer) previewContainer.innerHTML = "";
        if (previewStatus) previewStatus.textContent = `0 / ${total}`;
    });

    await listen('translation-progress', (event) => {
        const payload = event.payload as { completed: number, total: number, source: string, target: string };
        const { completed, total, source, target } = payload;
        const percent = Math.round((completed / total) * 100);
        
        setStatus(`Translating: ${completed}/${total} (${percent}%)`);
        if (previewStatus) previewStatus.textContent = `${completed} / ${total}`;
        
        if (previewContainer) {
            const row = document.createElement("div");
            row.className = "grid grid-cols-2 gap-4 p-3 border-b border-gray-700/50 text-sm hover:bg-gray-800/50 transition-colors";
            row.innerHTML = `
                <div class="text-gray-400 font-serif leading-relaxed text-right border-r border-gray-700 pr-4">${source}</div>
                <div class="text-emerald-300 font-serif leading-relaxed pl-2">${target}</div>
            `;
            // append to end since flex-col-reverse handles the scrolling to bottom/top
            // Wait, flex-col-reverse puts the FIRST child at the bottom.
            // If I append, it goes to the TOP (visually).
            // I want new items at the TOP? Or bottom?
            // Usually bottom like a terminal.
            // If I use flex-col-reverse, the first child (index 0) is at the bottom.
            // So prepending adds to the bottom visually? No.
            // Let's just use prepend for now, so newest is at the top.
            previewContainer.prepend(row);
        }
    });

    await listen('translation-complete', (event) => {
        const path = event.payload as string;
        setStatus(`Done! Saved to: ${path}`, "success");
        
        // Add to history
        const filename = path.split('/').pop() || path;
        addToHistory(filename, 0); // TODO: track actual block count
        renderRecentActivity();
        
        setTimeout(clearStatus, 5000);
    });
});

async function startTranslation(filePath: string) {
    if (!filePath) return;
    
    // Check if configured
    if (!currentConfig?.api.api_key && currentConfig?.api.provider !== "local") {
        setStatus("Please set an API Key first!", "error");
        return;
    }

    setStatus(`Initializing translation for ${filePath}...`);
    try {
        await invoke("translate_file", { filePath });
    } catch (e) {
        setStatus(`Error: ${e}`, "error");
    }
}
