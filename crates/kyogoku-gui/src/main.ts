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
    allocator?: string | null;
}

interface ProjectConfig {
    source_lang: string;
    target_lang: string;
    glossary_path?: string | null;
    input_dir?: string | null;
    output_dir?: string | null;
}

interface Config {
    api: ApiConfig;
    translation: TranslationConfig;
    advanced: AdvancedConfig;
    project: ProjectConfig;
}

// --- DOM Elements ---

const apiProvider = document.querySelector("#api-provider") as HTMLSelectElement;
const apiKey = document.querySelector("#api-key") as HTMLInputElement;
const apiModel = document.querySelector("#api-model") as HTMLInputElement;
const translationStyle = document.querySelector("#translation-style") as HTMLSelectElement;
const contextSize = document.querySelector("#context-size") as HTMLInputElement;
const maxConcurrency = document.querySelector("#max-concurrency") as HTMLInputElement;

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
        if (apiModel) apiModel.value = config.api.model;
        if (translationStyle) translationStyle.value = config.translation.style;
        if (contextSize) contextSize.value = config.translation.context_size.toString();
        if (maxConcurrency) maxConcurrency.value = config.advanced.max_concurrency.toString();

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
        },
        project: currentConfig.project // Keep existing project config
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
    });

    await listen('translation-progress', (event) => {
        const [done, total] = event.payload as [number, number];
        const percent = Math.round((done / total) * 100);
        setStatus(`Translating: ${done}/${total} (${percent}%)`);
        // We could update a progress bar here if we had one
    });

    await listen('translation-complete', (event) => {
        const path = event.payload as string;
        setStatus(`Done! Saved to: ${path}`, "success");
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
