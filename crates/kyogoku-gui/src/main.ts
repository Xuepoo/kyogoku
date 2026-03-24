import { invoke } from "@tauri-apps/api/core";
import { open } from '@tauri-apps/plugin-dialog';
import { listen } from '@tauri-apps/api/event';

// --- Theme ---

function initTheme() {
    const themeToggle = document.querySelector("#theme-toggle") as HTMLButtonElement;
    
    if (themeToggle) {
        themeToggle.addEventListener("click", () => {
            const isDark = document.documentElement.classList.contains('dark');
            
            if (isDark) {
                document.documentElement.classList.remove('dark');
                localStorage.setItem('theme', 'light');
            } else {
                document.documentElement.classList.add('dark');
                localStorage.setItem('theme', 'dark');
            }
        });
    }
    
    // Listen for system theme changes
    window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (e) => {
        if (!localStorage.getItem('theme')) {
            if (e.matches) {
                document.documentElement.classList.add('dark');
            } else {
                document.documentElement.classList.remove('dark');
            }
        }
    });
}

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

// --- Batch File Queue ---

const fileQueueSection = document.querySelector("#file-queue-section") as HTMLElement;
const fileQueueContainer = document.querySelector("#file-queue-container") as HTMLElement;
const startBatchBtn = document.querySelector("#start-batch-btn") as HTMLButtonElement;
const clearQueueBtn = document.querySelector("#clear-queue-btn") as HTMLButtonElement;

// --- Statistics Panel ---

const statsPanel = document.querySelector("#stats-panel") as HTMLElement;
const statsFilesCompleted = document.querySelector("#stats-files-completed") as HTMLElement;
const statsFilesTotal = document.querySelector("#stats-files-total") as HTMLElement;
const statsBlocksCompleted = document.querySelector("#stats-blocks-completed") as HTMLElement;
const statsBlocksTotal = document.querySelector("#stats-blocks-total") as HTMLElement;
const statsElapsed = document.querySelector("#stats-elapsed") as HTMLElement;
const statsEta = document.querySelector("#stats-eta") as HTMLElement;
const statsProgressBar = document.querySelector("#stats-progress-bar") as HTMLElement;

interface FileQueueItem {
    id: string;
    file_path: string;
    file_name: string;
    status: 'pending' | 'processing' | 'complete' | 'failed';
    word_count?: number;
    progress: number;
    error_message?: string;
}

interface BatchStats {
    total_files: number;
    completed_files: number;
    failed_files: number;
    total_blocks: number;
    completed_blocks: number;
    elapsed_seconds: number;
    estimated_remaining_seconds?: number;
}

let fileQueue: FileQueueItem[] = [];

function formatTime(seconds: number): string {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
}

function updateStatsPanel(stats: BatchStats) {
    if (statsFilesCompleted) statsFilesCompleted.textContent = stats.completed_files.toString();
    if (statsFilesTotal) statsFilesTotal.textContent = stats.total_files.toString();
    if (statsBlocksCompleted) statsBlocksCompleted.textContent = stats.completed_blocks.toString();
    if (statsBlocksTotal) statsBlocksTotal.textContent = stats.total_blocks.toString();
    if (statsElapsed) statsElapsed.textContent = formatTime(stats.elapsed_seconds);
    if (statsEta) {
        statsEta.textContent = stats.estimated_remaining_seconds 
            ? formatTime(stats.estimated_remaining_seconds)
            : '--:--';
    }
    
    // Update progress bar
    if (statsProgressBar && stats.total_blocks > 0) {
        const percent = (stats.completed_blocks / stats.total_blocks) * 100;
        statsProgressBar.style.width = `${percent}%`;
    }
}

async function addFilesToQueue(filePaths: string[]) {
    try {
        await invoke<FileQueueItem[]>("add_files_to_queue", { filePaths });
        fileQueue = await invoke<FileQueueItem[]>("get_file_queue");
        renderFileQueue();
        
        if (fileQueueSection) {
            fileQueueSection.classList.remove("hidden");
        }
        if (recentActivity) {
            recentActivity.classList.add("hidden");
        }
    } catch (e) {
        console.error("Failed to add files to queue:", e);
        setStatus(`Error: ${e}`, "error");
    }
}

async function removeFileFromQueue(fileId: string) {
    try {
        await invoke("remove_from_queue", { fileId });
        fileQueue = await invoke<FileQueueItem[]>("get_file_queue");
        renderFileQueue();
        
        if (fileQueue.length === 0 && fileQueueSection) {
            fileQueueSection.classList.add("hidden");
            if (recentActivity) {
                recentActivity.classList.remove("hidden");
            }
        }
    } catch (e) {
        console.error("Failed to remove file:", e);
    }
}

async function clearQueue() {
    try {
        await invoke("clear_queue");
        fileQueue = [];
        renderFileQueue();
        
        if (fileQueueSection) {
            fileQueueSection.classList.add("hidden");
        }
        if (recentActivity) {
            recentActivity.classList.remove("hidden");
        }
    } catch (e) {
        console.error("Failed to clear queue:", e);
    }
}

function renderFileQueue() {
    if (!fileQueueContainer) return;
    
    if (fileQueue.length === 0) {
        fileQueueContainer.innerHTML = '<p class="text-sm text-gray-500 text-center py-4">No files in queue</p>';
        return;
    }
    
    const statusColors = {
        pending: 'bg-gray-600 text-gray-300',
        processing: 'bg-blue-600 text-white animate-pulse',
        complete: 'bg-green-600 text-white',
        failed: 'bg-red-600 text-white'
    };
    
    const statusIcons = {
        pending: '⏳',
        processing: '🔄',
        complete: '✅',
        failed: '❌'
    };
    
    fileQueueContainer.innerHTML = fileQueue.map(item => `
        <div class="bg-gray-900/50 rounded p-3 flex items-center justify-between border border-gray-700 hover:border-gray-600 transition">
            <div class="flex-1 min-w-0">
                <div class="flex items-center gap-2 mb-1">
                    <span class="${statusColors[item.status]} px-2 py-0.5 rounded text-xs font-mono">
                        ${statusIcons[item.status]} ${item.status.toUpperCase()}
                    </span>
                    <span class="text-gray-300 text-sm font-medium truncate">${item.file_name}</span>
                </div>
                ${item.word_count ? `<p class="text-xs text-gray-500">~${item.word_count} blocks</p>` : ''}
                ${item.status === 'processing' ? `
                    <div class="mt-2 bg-gray-800 rounded-full h-1.5 overflow-hidden">
                        <div class="bg-blue-500 h-full transition-all duration-300" style="width: ${item.progress}%"></div>
                    </div>
                ` : ''}
                ${item.error_message ? `<p class="text-xs text-red-400 mt-1">${item.error_message}</p>` : ''}
            </div>
            <button 
                class="ml-3 text-gray-500 hover:text-red-400 transition" 
                onclick="window.removeFileFromQueue('${item.id}')"
                ${item.status === 'processing' ? 'disabled' : ''}
            >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                </svg>
            </button>
        </div>
    `).join('');
}

async function startBatchTranslation() {
    try {
        setStatus("Starting batch translation...");
        
        if (startBatchBtn) {
            startBatchBtn.disabled = true;
            startBatchBtn.textContent = "⏸ Processing...";
        }
        
        await invoke("start_batch_translation");
    } catch (e) {
        setStatus(`Batch translation error: ${e}`, "error");
        console.error(e);
        
        if (startBatchBtn) {
            startBatchBtn.disabled = false;
            startBatchBtn.textContent = "▶ Start Batch";
        }
    }
}

// Expose functions to global scope for inline onclick handlers
(window as any).removeFileFromQueue = removeFileFromQueue;

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
    initTheme();
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
    
    // Batch queue button handlers
    if (startBatchBtn) {
        startBatchBtn.addEventListener("click", () => {
            startBatchTranslation();
        });
    }
    
    if (clearQueueBtn) {
        clearQueueBtn.addEventListener("click", () => {
            if (confirm("Clear all files from queue?")) {
                clearQueue();
            }
        });
    }

    if (dropZone) {
        dropZone.addEventListener("click", async () => {
            try {
                const selected = await open({
                    multiple: true, // Enable multiple file selection
                    filters: [{
                        name: 'Supported Files',
                        extensions: ['rpy', 'ass', 'srt', 'vtt', 'epub', 'txt', 'json', 'md']
                    }]
                });

                if (selected === null) return;
                
                const filePaths = Array.isArray(selected) ? selected : [selected];
                await addFilesToQueue(filePaths);
            } catch (e) {
                console.error("Failed to open file dialog", e);
            }
        });
    }

    // Listen for file drops (support multiple files)
    await listen('tauri://file-drop', (event) => {
        const files = event.payload as string[];
        if (files && files.length > 0) {
            addFilesToQueue(files);
        }
    });
    
    // Listen for batch events
    await listen('batch-started', (event) => {
        const totalFiles = event.payload as number;
        setStatus(`Batch started: ${totalFiles} files queued`);
        
        if (previewSection) previewSection.classList.remove("hidden");
        if (previewContainer) previewContainer.innerHTML = "";
        if (statsPanel) statsPanel.classList.remove("hidden");
    });
    
    await listen('batch-stats', (event) => {
        const stats = event.payload as BatchStats;
        updateStatsPanel(stats);
    });
    
    await listen('file-processing', async (event) => {
        const item = event.payload as FileQueueItem;
        setStatus(`Processing: ${item.file_name}`);
        
        // Update queue display
        fileQueue = await invoke<FileQueueItem[]>("get_file_queue");
        renderFileQueue();
    });
    
    await listen('file-complete', async (event) => {
        const [_fileId, outputPath] = event.payload as [string, string];
        const filename = outputPath.split('/').pop() || outputPath;
        
        setStatus(`✓ Completed: ${filename}`, "success");
        
        // Update queue display
        fileQueue = await invoke<FileQueueItem[]>("get_file_queue");
        renderFileQueue();
        
        addToHistory(filename, 0);
    });
    
    await listen('file-failed', async (event) => {
        const [_fileId, error] = event.payload as [string, string];
        setStatus(`✗ Failed: ${error}`, "error");
        
        // Update queue display
        fileQueue = await invoke<FileQueueItem[]>("get_file_queue");
        renderFileQueue();
    });
    
    await listen('batch-complete', async (event) => {
        const summary = event.payload as string;
        setStatus(summary, "success");
        
        if (startBatchBtn) {
            startBatchBtn.disabled = false;
            startBatchBtn.textContent = "▶ Start Batch";
        }
        
        // Refresh queue
        fileQueue = await invoke<FileQueueItem[]>("get_file_queue");
        renderFileQueue();
        renderRecentActivity();
        
        setTimeout(clearStatus, 5000);
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
