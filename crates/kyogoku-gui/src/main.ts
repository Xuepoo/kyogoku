import { invoke } from "@tauri-apps/api/core";
import { open } from '@tauri-apps/plugin-dialog';
import { listen } from '@tauri-apps/api/event';

interface GlossaryEntry {
    source: string;
    target: string;
    context?: string;
}

interface PreviewItem {
    id: number;
    source: string;
    target: string;
    warnings?: string[];
}

let previewItems: PreviewItem[] = [];
let previewFilter: 'all' | 'source' | 'target' | 'warnings' = 'all';
let previewSearch = '';

function checkQuality(source: string, target: string): string[] {
    const warnings: string[] = [];
    
    // Check brackets balance (simple count)
    const count = (str: string, char: string) => str.split(char).length - 1;
    if (count(source, '{') !== count(target, '{')) warnings.push("Mismatched { }");
    if (count(source, '[') !== count(target, '[')) warnings.push("Mismatched [ ]");
    if (count(source, '（') !== count(target, '（')) warnings.push("Mismatched （ ）");
    if (count(source, '「') !== count(target, '「')) warnings.push("Mismatched 「 」");

    // Check variables like {name}
    const sourceVars = source.match(/\{[^}]+\}/g) || [];
    for (const v of sourceVars) {
        if (!target.includes(v)) {
            warnings.push(`Missing variable: ${v}`);
        }
    }
    
    // Check for empty translation
    if (!target.trim() && source.trim()) {
        warnings.push("Empty translation");
    }
    
    return warnings;
}

// --- Toast Notification System ---

type ToastType = 'success' | 'error' | 'warning' | 'info';

interface ToastOptions {
    duration?: number;  // ms, default 4000
    dismissible?: boolean;
}

const toastIcons: Record<ToastType, string> = {
    success: `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path></svg>`,
    error: `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path></svg>`,
    warning: `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"></path></svg>`,
    info: `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path></svg>`
};

const toastStyles: Record<ToastType, string> = {
    success: 'bg-emerald-50 dark:bg-emerald-900/80 border-emerald-200 dark:border-emerald-700 text-emerald-800 dark:text-emerald-200',
    error: 'bg-rose-50 dark:bg-rose-900/80 border-rose-200 dark:border-rose-700 text-rose-800 dark:text-rose-200',
    warning: 'bg-amber-50 dark:bg-amber-900/80 border-amber-200 dark:border-amber-700 text-amber-800 dark:text-amber-200',
    info: 'bg-sky-50 dark:bg-sky-900/80 border-sky-200 dark:border-sky-700 text-sky-800 dark:text-sky-200'
};

const toastIconStyles: Record<ToastType, string> = {
    success: 'text-emerald-500 dark:text-emerald-400',
    error: 'text-rose-500 dark:text-rose-400',
    warning: 'text-amber-500 dark:text-amber-400',
    info: 'text-sky-500 dark:text-sky-400'
};

function showToast(message: string, type: ToastType = 'info', options: ToastOptions = {}) {
    const { duration = 4000, dismissible = true } = options;
    const container = document.getElementById('toast-container');
    if (!container) return;

    const toast = document.createElement('div');
    toast.className = `
        pointer-events-auto flex items-center gap-3 px-4 py-3 rounded-lg border shadow-lg
        backdrop-blur-sm transform transition-all duration-300 ease-out
        translate-x-full opacity-0 max-w-sm
        ${toastStyles[type]}
    `.trim().replace(/\s+/g, ' ');

    toast.innerHTML = `
        <span class="${toastIconStyles[type]}">${toastIcons[type]}</span>
        <span class="flex-1 text-sm font-medium">${message}</span>
        ${dismissible ? `<button class="ml-2 opacity-60 hover:opacity-100 transition-opacity" aria-label="Dismiss">
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path></svg>
        </button>` : ''}
    `;

    container.appendChild(toast);

    // Animate in
    requestAnimationFrame(() => {
        toast.classList.remove('translate-x-full', 'opacity-0');
        toast.classList.add('translate-x-0', 'opacity-100');
    });

    const dismiss = () => {
        toast.classList.add('translate-x-full', 'opacity-0');
        setTimeout(() => toast.remove(), 300);
    };

    if (dismissible) {
        const closeBtn = toast.querySelector('button');
        closeBtn?.addEventListener('click', dismiss);
    }

    if (duration > 0) {
        setTimeout(dismiss, duration);
    }
}

// Convenience functions
const toast = {
    success: (msg: string, opts?: ToastOptions) => showToast(msg, 'success', opts),
    error: (msg: string, opts?: ToastOptions) => showToast(msg, 'error', opts),
    warning: (msg: string, opts?: ToastOptions) => showToast(msg, 'warning', opts),
    info: (msg: string, opts?: ToastOptions) => showToast(msg, 'info', opts)
};

// --- Error Formatting ---

interface ErrorSuggestion {
    pattern: RegExp;
    title: string;
    suggestion: string;
}

const errorSuggestions: ErrorSuggestion[] = [
    {
        pattern: /401|unauthorized|auth/i,
        title: "Authentication Failed",
        suggestion: "Check your API key in Settings. Make sure it's valid and not expired."
    },
    {
        pattern: /429|rate.?limit|too many requests/i,
        title: "Rate Limited",
        suggestion: "Reduce batch size or wait a few minutes before trying again."
    },
    {
        pattern: /timeout|timed out/i,
        title: "Request Timeout",
        suggestion: "The API took too long to respond. Try again or reduce batch size."
    },
    {
        pattern: /network|connection|offline|ECONNREFUSED/i,
        title: "Network Error",
        suggestion: "Check your internet connection. The API server may also be down."
    },
    {
        pattern: /token.?limit|context.?length|max.?tokens/i,
        title: "Token Limit Exceeded",
        suggestion: "Your text is too long. Try splitting into smaller files."
    },
    {
        pattern: /no.?api.?key|api.?key.?not.?set|missing.?key/i,
        title: "API Key Missing",
        suggestion: "Go to Settings and enter your API key for the selected provider."
    },
    {
        pattern: /quota|insufficient.?balance|billing/i,
        title: "Quota Exceeded",
        suggestion: "Check your API provider's billing page. You may need to add credits."
    },
    {
        pattern: /invalid.?model|model.?not.?found/i,
        title: "Invalid Model",
        suggestion: "The selected model doesn't exist. Check model name in Settings."
    },
    {
        pattern: /permission|forbidden|403/i,
        title: "Permission Denied",
        suggestion: "Your API key doesn't have access to this resource."
    },
    {
        pattern: /server.?error|500|502|503|504/i,
        title: "Server Error",
        suggestion: "The API server is having issues. Wait a few minutes and try again."
    },
];

function formatError(error: string | Error | unknown): string {
    const errorStr = error instanceof Error ? error.message : String(error);
    
    for (const { pattern, title, suggestion } of errorSuggestions) {
        if (pattern.test(errorStr)) {
            return `${title}: ${suggestion}`;
        }
    }
    
    // Default: just return the error with a generic prefix
    return `Error: ${errorStr}`;
}

function notifyError(error: string | Error | unknown) {
    const formatted = formatError(error);
    toast.error(formatted, { duration: 6000, dismissible: true });
}

// --- Keyboard Shortcuts ---

interface Shortcut {
    key: string;
    ctrl?: boolean;
    alt?: boolean;
    shift?: boolean;
    description: string;
    action: () => void;
}

const shortcuts: Shortcut[] = [];

function initKeyboardShortcuts() {
    document.addEventListener('keydown', (e) => {
        // Don't trigger shortcuts when typing in inputs
        const target = e.target as HTMLElement;
        if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.tagName === 'SELECT') {
            // Allow Escape to blur inputs
            if (e.key === 'Escape') {
                target.blur();
            }
            return;
        }

        for (const shortcut of shortcuts) {
            const ctrlMatch = shortcut.ctrl ? (e.ctrlKey || e.metaKey) : !(e.ctrlKey || e.metaKey);
            const altMatch = shortcut.alt ? e.altKey : !e.altKey;
            const shiftMatch = shortcut.shift ? e.shiftKey : !e.shiftKey;
            
            if (e.key.toLowerCase() === shortcut.key.toLowerCase() && ctrlMatch && altMatch && shiftMatch) {
                e.preventDefault();
                shortcut.action();
                return;
            }
        }
    });
}

function registerShortcut(shortcut: Shortcut) {
    shortcuts.push(shortcut);
}

function getShortcutHint(key: string, ctrl = false, alt = false, shift = false): string {
    const parts: string[] = [];
    const isMac = navigator.platform.toUpperCase().indexOf('MAC') >= 0;
    
    if (ctrl) parts.push(isMac ? '⌘' : 'Ctrl');
    if (alt) parts.push(isMac ? '⌥' : 'Alt');
    if (shift) parts.push('⇧');
    parts.push(key.toUpperCase());
    
    return parts.join(isMac ? '' : '+');
}

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
const apiTemperature = document.querySelector("#api-temperature") as HTMLInputElement;
const apiTemperatureValue = document.querySelector("#api-temperature-value") as HTMLElement;
const apiMaxTokens = document.querySelector("#api-max-tokens") as HTMLInputElement;
const translationStyle = document.querySelector("#translation-style") as HTMLSelectElement;
const contextSize = document.querySelector("#context-size") as HTMLInputElement;
const maxConcurrency = document.querySelector("#max-concurrency") as HTMLInputElement;
const batchSize = document.querySelector("#batch-size") as HTMLInputElement;
const budgetPresetBtn = document.querySelector("#budget-preset-btn") as HTMLButtonElement;

// RAG Elements
const ragEnabled = document.querySelector("#rag-enabled") as HTMLInputElement;
const ragModelPath = document.querySelector("#rag-model-path") as HTMLInputElement;
const ragTokenizerPath = document.querySelector("#rag-tokenizer-path") as HTMLInputElement;
const ragStorePath = document.querySelector("#rag-store-path") as HTMLInputElement;
const ragSettings = document.querySelector("#rag-settings") as HTMLElement;

const configForm = document.querySelector("#config-form") as HTMLFormElement;
const languageSelect = document.querySelector("#language-select") as HTMLSelectElement;
const reloadBtn = document.querySelector("#reload-btn") as HTMLButtonElement;
const statusMsg = document.querySelector("#status-msg") as HTMLElement;
const statusBadge = document.querySelector("#status-badge") as HTMLElement;

let currentConfig: Config | null = null;
let glossaryTerms: GlossaryEntry[] = [];

// --- Logic ---

async function loadConfig() {
    try {
        setStatus("Loading configuration...");
        const config = await invoke<Config>("get_config");
        currentConfig = config;
        
        console.log("Config loaded:", config);

        // Fetch glossary
        try {
            glossaryTerms = await invoke<GlossaryEntry[]>("get_glossary");
            console.log("Glossary loaded:", glossaryTerms.length, "entries");
        } catch (e) {
            console.warn("Failed to load glossary:", e);
        }

        // Populate form
        if (apiProvider) apiProvider.value = config.api.provider;
        if (apiKey) apiKey.value = config.api.api_key || "";
        if (apiBase) apiBase.value = config.api.api_base || "";
        if (apiModel) apiModel.value = config.api.model;
        if (apiTemperature) {
            apiTemperature.value = config.api.temperature?.toString() || "0.3";
            if (apiTemperatureValue) apiTemperatureValue.textContent = apiTemperature.value;
        }
        if (apiMaxTokens) apiMaxTokens.value = (config.api.max_tokens || 4096).toString();
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

        // Load output directory
        if (outputDirectory && config.project.output_dir) {
            outputDirectory.value = config.project.output_dir;
        }

        setStatus("Configuration loaded", "success");
        clearStatus();
    } catch (e) {
        notifyError(e);
        console.error(e);
    }
}

async function saveConfig() {
    if (!currentConfig) return;

    // Create updated config object
    // Note: We need to carefully reconstruct the nested structure to match Rust struct
    const parsedTemp = apiTemperature ? parseFloat(apiTemperature.value) : currentConfig.api.temperature;
    const parsedMaxTokens = apiMaxTokens ? parseInt(apiMaxTokens.value, 10) : currentConfig.api.max_tokens;

    const newConfig: Config = {
        api: {
            ...currentConfig.api,
            provider: apiProvider.value,
            api_key: apiKey.value || null,
            api_base: apiBase.value || undefined,
            model: apiModel.value,
            temperature: Number.isFinite(parsedTemp) ? parsedTemp : currentConfig.api.temperature,
            max_tokens: Number.isFinite(parsedMaxTokens) ? parsedMaxTokens : currentConfig.api.max_tokens,
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
        project: {
            ...currentConfig.project,
            output_dir: outputDirectory ? outputDirectory.value || null : null,
        },
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
        
        notify("Configuration saved!", "success");
        showSavedBadge();
        updateCostEstimation(); // Update cost when config changes
        clearStatus();
    } catch (e) {
        notifyError(e);
        console.error(e);
    }
}

function applyBudgetPreset() {
    if (apiProvider) apiProvider.value = "google";
    if (apiBase) apiBase.value = "https://openrouter.ai/api/v1";
    if (apiModel) apiModel.value = "google/gemini-2.5-flash";
    if (apiTemperature) {
        apiTemperature.value = "0.3";
        if (apiTemperatureValue) apiTemperatureValue.textContent = apiTemperature.value;
    }
    if (apiMaxTokens) apiMaxTokens.value = "4096";
}

// --- I18n Functions ---

let currentLocale = 'en-US';
const translations = new Map<string, string>();

async function initI18n() {
    try {
        currentLocale = await invoke("get_current_locale") as string;
        console.log(`Initialized locale: ${currentLocale}`);
        
        // Populate translations map if needed, or update text directly
        await updateTexts();
    } catch (e) {
        console.error("Failed to initialize i18n:", e);
    }
}

async function setLocale(locale: string) {
    try {
        await invoke("set_locale", { locale });
        currentLocale = locale;
        await updateTexts();
        notify("Language updated", "success");
    } catch (e) {
        console.error("Failed to set locale:", e);
        notifyError(e);
    }
}

async function t(key: string): Promise<string> {
    try {
        const text = await invoke("translate_text", { key }) as string;
        translations.set(key, text); // Cache it
        return text;
    } catch (e) {
        console.error(`Failed to translate key: ${key}`, e);
        return key;
    }
}

async function updateTexts() {
    // Update all UI texts with translations
    const elements = document.querySelectorAll('[data-i18n]');
    for (const element of elements) {
        const key = element.getAttribute('data-i18n');
        if (key) {
            const text = await t(key);
            element.textContent = text;
        }
    }
}

// --- Helpers ---

// For real-time progress updates (doesn't trigger toast)
function setStatus(msg: string, type: "info" | "success" | "error" = "info") {
    if (!statusMsg) return;
    statusMsg.textContent = msg;
    statusMsg.classList.remove("opacity-0");
    statusMsg.classList.add("opacity-100");
    
    statusMsg.className = "text-xs text-center h-4 transition-all duration-300 font-mono opacity-100";
    if (type === "error") statusMsg.classList.add("text-rose-500", "dark:text-rose-400");
    else if (type === "success") statusMsg.classList.add("text-emerald-500", "dark:text-emerald-400");
    else statusMsg.classList.add("text-stone-500", "dark:text-stone-400");
}

// For important notifications (triggers toast)
function notify(msg: string, type: "info" | "success" | "error" | "warning" = "info") {
    toast[type](msg);
}

function clearStatus() {
    if (statusMsg) {
        statusMsg.classList.add("opacity-0");
        setTimeout(() => {
            if (statusMsg) statusMsg.textContent = "";
        }, 300);
    }
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

// --- Cost Estimation ---

const TOKEN_PRICING: {[key: string]: {input: number, output: number}} = {
    // OpenAI pricing per 1M tokens
    'gpt-4o': { input: 2.50, output: 10.00 },
    'gpt-4o-mini': { input: 0.15, output: 0.60 },
    'gpt-4-turbo': { input: 10.00, output: 30.00 },
    'gpt-3.5-turbo': { input: 0.50, output: 1.50 },
    
    // Anthropic pricing per 1M tokens
    'claude-3-5-sonnet-20241022': { input: 3.00, output: 15.00 },
    'claude-3-5-haiku-20241022': { input: 0.25, output: 1.25 },
    'claude-3-opus-20240229': { input: 15.00, output: 75.00 },
    
    // DeepSeek pricing per 1M tokens
    'deepseek-chat': { input: 0.14, output: 0.28 },
    'deepseek-coder': { input: 0.14, output: 0.28 },
    
    // Google pricing per 1M tokens
    'gemini-1.5-pro': { input: 1.25, output: 5.00 },
    'gemini-1.5-flash': { input: 0.075, output: 0.30 },
    'google/gemini-2.5-flash': { input: 0.075, output: 0.30 },
    
    // Default for unknown models
    'default': { input: 1.00, output: 3.00 }
};

function estimateTokens(text: string): number {
    // Rough estimation: ~4 chars per token for English/Chinese mixed text
    // This is a simplification, real tokenization varies by model
    return Math.ceil(text.length / 4);
}

function updateCostEstimation() {
    if (!costTokens || !costAmount || !currentConfig) {
        return;
    }

    let totalTokens = 0;
    let totalCost = 0;

    // Get current model and pricing
    const modelId = currentConfig.api.model.toLowerCase();
    const pricing = TOKEN_PRICING[modelId] || TOKEN_PRICING['default'];

    // Calculate total tokens from file queue
    for (const item of fileQueue) {
        if (item.word_count) {
            // Estimate tokens (word_count is actually block count)
            // Each block might average 50 characters
            const textLength = item.word_count * 50; 
            const blockTokens = estimateTokens("a".repeat(textLength)); // Use helper
            
            // For translation, we need input tokens (source) + output tokens (translation)
            // Assume translation is same length as source
            const inputTokens = blockTokens;
            const outputTokens = blockTokens;
            
            totalTokens += inputTokens + outputTokens;
            totalCost += (inputTokens * pricing.input / 1000000) + (outputTokens * pricing.output / 1000000);
        }
    }

    // Update display
    costTokens.textContent = totalTokens > 0 ? 
        `${Math.round(totalTokens).toLocaleString()}` : '—';
    
    costAmount.textContent = totalCost > 0 ? 
        `$${totalCost.toFixed(3)}` : '$—';
        
    // Show cost panel if we have files
    if (costPanel && totalTokens > 0) {
        costPanel.classList.remove('hidden');
    } else if (costPanel && totalTokens === 0) {
        costPanel.classList.add('hidden');
    }
}

// --- Statistics Export ---

interface StatsSummary {
    session: {
        timestamp: string;
        model: string;
        provider: string;
        total_files: number;
        completed_files: number;
        failed_files: number;
        total_blocks: number;
        completed_blocks: number;
        elapsed_seconds: number;
        estimated_cost: number;
        estimated_tokens: number;
    };
    files: Array<{
        file_name: string;
        file_path: string;
        status: string;
        word_count?: number;
        progress: number;
        error_message?: string;
    }>;
}

let currentStats: BatchStats | null = null;

function generateStatsSummary(): StatsSummary {
    const now = new Date();
    const summary: StatsSummary = {
        session: {
            timestamp: now.toISOString(),
            model: currentConfig?.api.model || 'unknown',
            provider: currentConfig?.api.provider || 'unknown',
            total_files: fileQueue.length,
            completed_files: fileQueue.filter(f => f.status === 'complete').length,
            failed_files: fileQueue.filter(f => f.status === 'failed').length,
            total_blocks: currentStats?.total_blocks || 0,
            completed_blocks: currentStats?.completed_blocks || 0,
            elapsed_seconds: currentStats?.elapsed_seconds || 0,
            estimated_cost: 0,
            estimated_tokens: 0
        },
        files: fileQueue.map(item => ({
            file_name: item.file_name,
            file_path: item.file_path,
            status: item.status,
            word_count: item.word_count,
            progress: item.progress,
            error_message: item.error_message
        }))
    };

    // Calculate estimated cost and tokens
    if (currentConfig) {
        let totalTokens = 0;
        let totalCost = 0;
        const modelId = currentConfig.api.model.toLowerCase();
        const pricing = TOKEN_PRICING[modelId] || TOKEN_PRICING['default'];

        for (const item of fileQueue) {
            if (item.word_count) {
                const blockTokens = item.word_count * 50 / 4;
                const inputTokens = blockTokens;
                const outputTokens = blockTokens;
                
                totalTokens += inputTokens + outputTokens;
                totalCost += (inputTokens * pricing.input / 1000000) + (outputTokens * pricing.output / 1000000);
            }
        }
        
        summary.session.estimated_tokens = Math.round(totalTokens);
        summary.session.estimated_cost = Number(totalCost.toFixed(6));
    }

    return summary;
}

function exportAsCSV() {
    const summary = generateStatsSummary();
    
    // Create CSV content
    let csv = 'File Name,File Path,Status,Word Count,Progress (%),Error Message\n';
    
    for (const file of summary.files) {
        const line = [
            `"${file.file_name}"`,
            `"${file.file_path}"`,
            file.status,
            file.word_count || '',
            file.progress.toFixed(1),
            `"${file.error_message || ''}"`
        ].join(',');
        csv += line + '\n';
    }
    
    // Add summary at the end
    csv += '\n--- Session Summary ---\n';
    csv += `Timestamp,"${summary.session.timestamp}"\n`;
    csv += `Model,"${summary.session.model}"\n`;
    csv += `Provider,"${summary.session.provider}"\n`;
    csv += `Total Files,${summary.session.total_files}\n`;
    csv += `Completed Files,${summary.session.completed_files}\n`;
    csv += `Failed Files,${summary.session.failed_files}\n`;
    csv += `Total Blocks,${summary.session.total_blocks}\n`;
    csv += `Completed Blocks,${summary.session.completed_blocks}\n`;
    csv += `Elapsed (seconds),${summary.session.elapsed_seconds}\n`;
    csv += `Estimated Tokens,${summary.session.estimated_tokens}\n`;
    csv += `Estimated Cost ($),${summary.session.estimated_cost}\n`;

    // Download CSV
    downloadFile(csv, 'kyogoku-stats.csv', 'text/csv');
    toast.success('Statistics exported as CSV');
}

function exportAsJSON() {
    const summary = generateStatsSummary();
    
    // Convert to JSON
    const jsonStr = JSON.stringify(summary, null, 2);
    
    // Download JSON
    downloadFile(jsonStr, 'kyogoku-stats.json', 'application/json');
    toast.success('Statistics exported as JSON');
}

function downloadFile(content: string, fileName: string, mimeType: string) {
    const blob = new Blob([content], { type: mimeType });
    const url = URL.createObjectURL(blob);
    
    const link = document.createElement('a');
    link.href = url;
    link.download = fileName;
    link.click();
    
    URL.revokeObjectURL(url);
}

function updateExportButtons() {
    const hasData = fileQueue.length > 0;
    if (exportCsvBtn) {
        exportCsvBtn.disabled = !hasData;
    }
    if (exportJsonBtn) {
        exportJsonBtn.disabled = !hasData;
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

// --- Output Directory ---
const outputDirectory = document.querySelector("#output-directory") as HTMLInputElement;
const browseOutputBtn = document.querySelector("#browse-output-btn") as HTMLButtonElement;
const clearOutputBtn = document.querySelector("#clear-output-btn") as HTMLButtonElement;

// --- Statistics Panel ---

const statsPanel = document.querySelector("#stats-panel") as HTMLElement;
const statsFilesCompleted = document.querySelector("#stats-files-completed") as HTMLElement;
const statsFilesTotal = document.querySelector("#stats-files-total") as HTMLElement;
const statsBlocksCompleted = document.querySelector("#stats-blocks-completed") as HTMLElement;
const statsBlocksTotal = document.querySelector("#stats-blocks-total") as HTMLElement;
const statsElapsed = document.querySelector("#stats-elapsed") as HTMLElement;
const statsEta = document.querySelector("#stats-eta") as HTMLElement;
const statsProgressBar = document.querySelector("#stats-progress-bar") as HTMLElement;
const exportCsvBtn = document.querySelector("#export-csv-btn") as HTMLButtonElement;
const exportJsonBtn = document.querySelector("#export-json-btn") as HTMLButtonElement;

// --- Cost Estimation Panel ---

const costPanel = document.querySelector("#cost-panel") as HTMLElement;
const costTokens = document.querySelector("#cost-tokens") as HTMLElement;
const costAmount = document.querySelector("#cost-amount") as HTMLElement;

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
    currentStats = stats; // Save current stats for export
    
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
    
    // Update export buttons
    updateExportButtons();
}

async function addFilesToQueue(filePaths: string[]) {
    try {
        await invoke<FileQueueItem[]>("add_files_to_queue", { filePaths });
        fileQueue = await invoke<FileQueueItem[]>("get_file_queue");
        renderFileQueue();
        updateCostEstimation();
        updateExportButtons();
        
        if (fileQueueSection) {
            fileQueueSection.classList.remove("hidden");
        }
        if (recentActivity) {
            recentActivity.classList.add("hidden");
        }
        toast.info(`Added ${filePaths.length} file(s) to queue`);
    } catch (e) {
        console.error("Failed to add files to queue:", e);
        notifyError(e);
    }
}

async function removeFileFromQueue(fileId: string) {
    try {
        await invoke("remove_from_queue", { fileId });
        fileQueue = await invoke<FileQueueItem[]>("get_file_queue");
        renderFileQueue();
        updateCostEstimation();
        updateExportButtons();
        
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
        updateCostEstimation();
        updateExportButtons();
        
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
        fileQueueContainer.innerHTML = `
            <div class="text-center py-8 px-4 border-2 border-dashed border-stone-200 dark:border-stone-800 rounded-xl bg-stone-50/50 dark:bg-stone-900/50">
                <div class="text-3xl mb-3 opacity-40 grayscale">📂</div>
                <p class="text-sm text-stone-500 dark:text-stone-400 mb-2 font-medium">No files in queue</p>
                <p class="text-xs text-stone-400 dark:text-stone-500">
                    Files you add will appear here ready for batch processing
                </p>
            </div>
        `;
        return;
    }
    
    const statusColors: Record<string, string> = {
        pending: 'bg-stone-200 dark:bg-stone-700 text-stone-600 dark:text-stone-300',
        processing: 'bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300 animate-pulse',
        complete: 'bg-emerald-100 dark:bg-emerald-900/30 text-emerald-700 dark:text-emerald-300',
        failed: 'bg-rose-100 dark:bg-rose-900/30 text-rose-700 dark:text-rose-300'
    };
    
    const statusIcons: Record<string, string> = {
        pending: 'Wait',
        processing: 'Processing',
        complete: 'Done',
        failed: 'Failed'
    };
    
    fileQueueContainer.innerHTML = fileQueue.map(item => `
        <div class="bg-white dark:bg-stone-900 p-3 rounded-lg border border-stone-200 dark:border-stone-800 flex items-center justify-between shadow-sm hover:border-amber-300 dark:hover:border-amber-700/50 transition-colors group">
            <div class="flex-1 min-w-0 pr-4">
                <div class="flex items-center gap-2 mb-1.5">
                    <span class="${statusColors[item.status]} px-2 py-0.5 rounded text-[10px] font-bold uppercase tracking-wide flex items-center gap-1">
                        ${item.status === 'processing' ? '<span class="animate-spin">⟳</span>' : ''}
                        ${statusIcons[item.status]}
                    </span>
                    <span class="text-stone-700 dark:text-stone-200 text-sm font-semibold truncate" title="${item.file_name}">${item.file_name}</span>
                </div>
                
                <div class="flex items-center gap-4">
                    ${item.word_count ? `<p class="text-[10px] text-stone-400 font-mono">~${item.word_count} blocks</p>` : ''}
                    ${item.error_message ? `<p class="text-[10px] text-rose-500 font-medium truncate max-w-[200px]" title="${item.error_message}">${item.error_message}</p>` : ''}
                </div>
                
                ${item.status === 'processing' ? `
                    <div class="mt-2 bg-stone-200 dark:bg-stone-800 rounded-full h-1 overflow-hidden">
                        <div class="bg-blue-500 h-full transition-all duration-300 rounded-full" style="width: ${Math.max(5, item.progress)}%"></div>
                    </div>
                ` : ''}
            </div>
            
            <button 
                class="p-1.5 text-stone-400 hover:text-rose-500 hover:bg-rose-50 dark:hover:bg-rose-900/20 rounded-md transition-colors ${item.status === 'processing' ? 'opacity-30 cursor-not-allowed' : ''}" 
                onclick="window.removeFileFromQueue('${item.id}')"
                ${item.status === 'processing' ? 'disabled' : ''}
                title="Remove from queue"
                aria-label="Remove from queue"
            >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
                </svg>
            </button>
        </div>
    `).join('');
}

async function startBatchTranslation() {
    try {
        setStatus("Starting batch translation...");
        toast.info("Starting batch translation...");
        
        if (startBatchBtn) {
            startBatchBtn.disabled = true;
            startBatchBtn.innerHTML = `
                <svg class="animate-spin -ml-1 mr-2 h-4 w-4 text-white" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                    <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                    <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                </svg>
                Processing...
            `;
        }

        // Show loading state immediately
        if (previewSection) previewSection.classList.remove("hidden");
        if (recentActivity) recentActivity.classList.add("hidden");
        renderSkeletonPreview();
        
        // Show skeleton stats
        if (statsPanel) statsPanel.classList.remove('hidden');
        if (statsFilesCompleted) statsFilesCompleted.innerHTML = '<span class="skeleton rounded px-2 w-8 inline-block">&nbsp;</span>';
        if (statsBlocksCompleted) statsBlocksCompleted.innerHTML = '<span class="skeleton rounded px-2 w-12 inline-block">&nbsp;</span>';
        
        await invoke("start_batch_translation");
    } catch (e) {
        notifyError(e);
        console.error(e);
        
        if (startBatchBtn) {
            startBatchBtn.disabled = false;
            startBatchBtn.innerHTML = `
                <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z"></path><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path></svg>
                Start Batch
            `;
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
            <div class="flex items-center justify-between mb-3">
                <h3 class="font-bold text-stone-500 dark:text-stone-400 text-xs uppercase tracking-wider">Recent Activity</h3>
            </div>
            <div class="bg-stone-50 dark:bg-stone-900/50 rounded-lg p-8 text-center border border-stone-200 dark:border-stone-800 border-dashed">
                <div class="w-12 h-12 bg-stone-100 dark:bg-stone-800 rounded-full flex items-center justify-center mx-auto mb-3">
                    <svg class="w-6 h-6 text-stone-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"></path></svg>
                </div>
                <p class="text-sm font-medium text-stone-600 dark:text-stone-300 mb-1">No recent activity</p>
                <p class="text-xs text-stone-500 dark:text-stone-500">Your translation history will appear here</p>
            </div>
        `;
        return;
    }
    
    const items = history.map(item => {
        const date = new Date(item.timestamp);
        const timeStr = date.toLocaleString(undefined, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
        return `
            <div class="flex justify-between items-center p-3 bg-stone-50 dark:bg-stone-800/50 hover:bg-white dark:hover:bg-stone-800 rounded-lg border border-transparent hover:border-stone-200 dark:hover:border-stone-700 transition-all group cursor-default">
                <div class="flex items-center gap-3 overflow-hidden">
                    <div class="bg-amber-100 dark:bg-amber-900/30 p-1.5 rounded-md text-amber-600 dark:text-amber-500">
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path></svg>
                    </div>
                    <span class="text-stone-700 dark:text-stone-300 text-sm font-medium truncate">${item.filename}</span>
                </div>
                <span class="text-stone-400 dark:text-stone-500 text-xs whitespace-nowrap">${timeStr}</span>
            </div>
        `;
    }).join("");
    
    recentActivity.innerHTML = `
        <div class="flex items-center justify-between mb-3">
            <h3 class="font-bold text-stone-500 dark:text-stone-400 text-xs uppercase tracking-wider">Recent Activity</h3>
            <button onclick="localStorage.removeItem('kyogoku_history'); location.reload();" class="text-[10px] text-stone-400 hover:text-rose-500 transition-colors">Clear History</button>
        </div>
        <div class="space-y-2">${items}</div>
    `;
}


// --- Event Listeners ---

// --- Preview Helpers ---

const VISIBLE_ROWS = 50;
const ROW_HEIGHT = 80;
let virtualScrollTop = 0;
let filteredItems: PreviewItem[] = [];

function escapeHtml(text: string): string {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

function highlightTerms(text: string, mode: 'source' | 'target' = 'source'): string {
    if (!text) return "";
    let result = escapeHtml(text);
    
    if (!glossaryTerms.length) return result;

    const sortedTerms = [...glossaryTerms].sort((a, b) => {
        const lenA = mode === 'source' ? a.source.length : a.target.length;
        const lenB = mode === 'source' ? b.source.length : b.target.length;
        return lenB - lenA;
    });
    
    for (const term of sortedTerms) {
        const key = mode === 'source' ? term.source : term.target;
        const escapedKey = escapeHtml(key);
        
        if (result.includes(escapedKey)) {
            const tooltip = mode === 'source' 
                ? `${term.target}${term.context ? ' (' + term.context + ')' : ''}`
                : `${term.source}${term.context ? ' (' + term.context + ')' : ''}`;
            const escapedTooltip = escapeHtml(tooltip);
                
            const replacement = `<span class="text-amber-600 dark:text-amber-400 font-medium border-b border-dashed border-amber-500/50 cursor-help" title="Glossary: ${escapedTooltip}">${escapedKey}</span>`;
            result = result.split(escapedKey).join(replacement);
        }
    }
    return result;
}

function getFilteredItems(): PreviewItem[] {
    return previewItems.filter(item => {
        if (previewSearch) {
            const searchLower = previewSearch.toLowerCase();
            if (!item.source.toLowerCase().includes(searchLower) && !item.target.toLowerCase().includes(searchLower)) {
                return false;
            }
        }
        if (previewFilter === 'warnings') {
            return item.warnings && item.warnings.length > 0;
        }
        return true;
    });
}

function renderPreviewRow(item: PreviewItem): string {
    const showSource = previewFilter === 'all' || previewFilter === 'source' || previewFilter === 'warnings';
    const showTarget = previewFilter === 'all' || previewFilter === 'target' || previewFilter === 'warnings';
    const isTwoCol = previewFilter === 'all' || previewFilter === 'warnings';
    
    let html = `<div class="grid ${isTwoCol ? 'grid-cols-2' : 'grid-cols-1'} gap-4 p-4 border-b border-stone-200 dark:border-stone-800 text-sm hover:bg-stone-100 dark:hover:bg-stone-900/80 transition-colors" style="min-height: ${ROW_HEIGHT}px;">`;
    
    if (showSource) {
        html += `<div class="text-stone-600 dark:text-stone-400 font-serif leading-relaxed ${isTwoCol ? 'text-right border-r border-stone-200 dark:border-stone-800 pr-4' : ''}">${highlightTerms(item.source, 'source')}</div>`;
    }
    if (showTarget) {
        html += `<div class="text-stone-900 dark:text-emerald-100 font-serif leading-relaxed ${isTwoCol ? 'pl-2' : ''}">${highlightTerms(item.target, 'target')}</div>`;
    }
    if (item.warnings && item.warnings.length > 0) {
        html += `<div class="col-span-2 text-xs text-amber-600 dark:text-amber-500 bg-amber-50 dark:bg-amber-900/20 px-3 py-2 rounded-lg flex items-center gap-2 mt-2 border border-amber-200 dark:border-amber-800/30">
            <svg class="w-4 h-4 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"></path></svg>
            <span>${item.warnings.join(', ')}</span>
        </div>`;
    }
    html += `</div>`;
    return html;
}

function renderSkeletonPreview() {
    if (!previewContainer) return;
    
    const skeletonRow = `
        <div class="grid grid-cols-2 gap-4 p-4 border-b border-stone-200 dark:border-stone-800" style="height: ${ROW_HEIGHT}px;">
            <div class="space-y-2 border-r border-stone-200 dark:border-stone-800 pr-4">
                <div class="h-4 w-3/4 ml-auto skeleton rounded"></div>
                <div class="h-4 w-1/2 ml-auto skeleton rounded"></div>
            </div>
            <div class="space-y-2 pl-2">
                <div class="h-4 w-5/6 skeleton rounded"></div>
                <div class="h-4 w-2/3 skeleton rounded"></div>
            </div>
        </div>
    `;
    
    previewContainer.innerHTML = Array(6).fill(skeletonRow).join('');
}

function renderPreview() {
    if (!previewContainer) return;

    // Show empty state if no items
    if (previewItems.length === 0) {
        previewContainer.innerHTML = `
            <div class="flex flex-col items-center justify-center h-full text-center p-8 opacity-60">
                <div class="w-16 h-16 bg-stone-100 dark:bg-stone-900 rounded-full flex items-center justify-center mb-4 text-stone-300 dark:text-stone-600">
                    <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path></svg>
                </div>
                <h3 class="text-stone-900 dark:text-stone-200 font-medium mb-1">Preview Area</h3>
                <p class="text-sm text-stone-500 dark:text-stone-400 max-w-xs">
                    Translation progress and results will appear here in real-time.
                </p>
            </div>
        `;
        return;
    }

    filteredItems = getFilteredItems();
    const totalItems = filteredItems.length;
    
    // For small lists, render all
    if (totalItems <= VISIBLE_ROWS) {
        previewContainer.innerHTML = filteredItems.map(renderPreviewRow).join('');
        previewContainer.scrollTop = previewContainer.scrollHeight;
        return;
    }

    // Virtual scrolling for large lists
    const startIdx = Math.max(0, Math.floor(virtualScrollTop / ROW_HEIGHT));
    const endIdx = Math.min(totalItems, startIdx + VISIBLE_ROWS);
    const visibleItems = filteredItems.slice(startIdx, endIdx);
    
    const paddingTop = startIdx * ROW_HEIGHT;
    const paddingBottom = (totalItems - endIdx) * ROW_HEIGHT;
    
    previewContainer.innerHTML = `
        <div style="height: ${paddingTop}px;"></div>
        ${visibleItems.map(renderPreviewRow).join('')}
        <div style="height: ${paddingBottom}px;"></div>
    `;
}

// --- Onboarding Wizard ---

function shouldShowOnboarding(): boolean {
    // Check if user has completed onboarding before
    const completed = localStorage.getItem('onboarding_completed');
    if (completed === 'true') return false;
    
    // Check if API key is configured
    if (currentConfig && currentConfig.api.api_key && currentConfig.api.api_key !== 'ENV_VAR' && currentConfig.api.api_key.length > 10) {
        return false;
    }
    
    return true;
}

function initOnboarding() {
    const wizard = document.getElementById('onboarding-wizard');
    const step1 = document.getElementById('wizard-step-1');
    const step2 = document.getElementById('wizard-step-2');
    const step3 = document.getElementById('wizard-step-3');
    
    const wizardProvider = document.getElementById('wizard-provider') as HTMLSelectElement;
    const wizardApiBase = document.getElementById('wizard-api-base') as HTMLInputElement;
    const wizardModel = document.getElementById('wizard-model') as HTMLInputElement;
    const wizardApiKey = document.getElementById('wizard-api-key') as HTMLInputElement;
    
    if (!wizard || !step1 || !step2 || !step3) return;
    
    function showStep(step: number) {
        [step1, step2, step3].forEach((el, idx) => {
            el?.classList.toggle('hidden', idx + 1 !== step);
        });
    }
    
    function closeWizard() {
        if (wizard) wizard.classList.add('hidden');
        localStorage.setItem('onboarding_completed', 'true');
    }
    
    // Step 1: Welcome
    document.getElementById('wizard-skip')?.addEventListener('click', closeWizard);
    document.getElementById('wizard-next-1')?.addEventListener('click', () => showStep(2));
    
    // Step 2: API Config
    document.getElementById('wizard-back-2')?.addEventListener('click', () => showStep(1));
    document.getElementById('wizard-skip-2')?.addEventListener('click', closeWizard);
    document.getElementById('wizard-next-2')?.addEventListener('click', async () => {
        const provider = wizardProvider?.value || 'custom';
        const apiBase = wizardApiBase?.value || '';
        const model = wizardModel?.value || 'google/gemini-2.5-flash';
        const apiKey = wizardApiKey?.value || '';
        
        // Validate API key
        if (!apiKey || apiKey.length < 10) {
            showToast('Please enter a valid API key', 'error');
            wizardApiKey?.focus();
            return;
        }
        
        // Save config
        if (!currentConfig) {
            showToast('Configuration not loaded', 'error');
            return;
        }
        
        try {
            await invoke('update_config', {
                config: {
                    ...currentConfig,
                    api: {
                        ...currentConfig.api,
                        provider: provider,
                        api_base: apiBase || undefined,
                        model: model,
                        api_key: apiKey
                    }
                }
            });
            
            showToast('Configuration saved successfully', 'success');
            await loadConfig();
            showStep(3);
        } catch (e) {
            showToast(`Failed to save configuration: ${e}`, 'error');
        }
    });
    
    // Step 3: Done
    document.getElementById('wizard-finish')?.addEventListener('click', closeWizard);
    
    // Show wizard if needed
    if (shouldShowOnboarding()) {
        wizard.classList.remove('hidden');
        showStep(1);
    }
}

window.addEventListener("DOMContentLoaded", async () => {
    initTheme();
    initKeyboardShortcuts();
    await loadConfig();
    initOnboarding();

    // Register keyboard shortcuts
    registerShortcut({
        key: 'o',
        ctrl: true,
        description: 'Open files',
        action: async () => {
            const selected = await open({
                multiple: true,
                filters: [{
                    name: 'Supported Files',
                    extensions: ['txt', 'srt', 'vtt', 'ass', 'ssa', 'json', 'epub', 'rpy']
                }]
            });
            if (selected && Array.isArray(selected) && selected.length > 0) {
                addFilesToQueue(selected);
            }
        }
    });

    registerShortcut({
        key: 's',
        ctrl: true,
        description: 'Save configuration',
        action: async () => await saveConfig()
    });

    registerShortcut({
        key: 'Enter',
        ctrl: true,
        description: 'Start batch translation',
        action: () => {
            if (fileQueue.length > 0 && startBatchBtn && !startBatchBtn.disabled) {
                startBatchTranslation();
            }
        }
    });

    registerShortcut({
        key: 't',
        ctrl: true,
        description: 'Toggle theme',
        action: () => {
            const isDark = document.documentElement.classList.contains('dark');
            if (isDark) {
                document.documentElement.classList.remove('dark');
                localStorage.setItem('theme', 'light');
            } else {
                document.documentElement.classList.add('dark');
                localStorage.setItem('theme', 'dark');
            }
        }
    });

    registerShortcut({
        key: '?',
        shift: true,
        description: 'Show keyboard shortcuts',
        action: () => {
            const shortcutList = shortcuts.map(s => {
                const hint = getShortcutHint(s.key, s.ctrl, s.alt, s.shift);
                return `${hint}: ${s.description}`;
            }).join('\n');
            toast.info(`Keyboard Shortcuts:\n${shortcutList}`, { duration: 6000 });
        }
    });

    if (configForm) {
        configForm.addEventListener("submit", async (e) => {
            e.preventDefault();
            await saveConfig();
        });
    }

    if (reloadBtn) {
        reloadBtn.addEventListener("click", () => {
            loadConfig();
        });
    }

    if (apiTemperature && apiTemperatureValue) {
        apiTemperature.addEventListener("input", () => {
            apiTemperatureValue.textContent = apiTemperature.value;
        });
    }

    if (budgetPresetBtn) {
        budgetPresetBtn.addEventListener("click", async () => {
            applyBudgetPreset();
            await saveConfig();
            updateCostEstimation();
            toast.success("Budget preset applied: OpenRouter + google/gemini-2.5-flash");
        });
    }

    if (languageSelect) {
        languageSelect.addEventListener("change", (e) => {
            const select = e.target as HTMLSelectElement;
            setLocale(select.value);
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

    // Export button handlers
    if (exportCsvBtn) {
        exportCsvBtn.addEventListener("click", () => {
            exportAsCSV();
        });
    }

    if (exportJsonBtn) {
        exportJsonBtn.addEventListener("click", () => {
            exportAsJSON();
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

    // --- Output Directory Event Handlers ---

    if (browseOutputBtn) {
        browseOutputBtn.addEventListener("click", async () => {
            try {
                const selected = await open({
                    directory: true,
                    title: 'Select Output Directory'
                });

                if (selected !== null && outputDirectory) {
                    outputDirectory.value = selected;
                    // Auto-save configuration
                    await saveConfig();
                    toast.success("Output directory updated");
                }
            } catch (e) {
                console.error("Failed to open directory dialog", e);
                notifyError(e);
            }
        });
    }

    if (clearOutputBtn) {
        clearOutputBtn.addEventListener("click", async () => {
            if (outputDirectory) {
                outputDirectory.value = '';
                // Auto-save configuration
                await saveConfig();
                toast.info("Output directory cleared");
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
        notifyError(error);
        
        // Update queue display
        fileQueue = await invoke<FileQueueItem[]>("get_file_queue");
        renderFileQueue();
    });
    
    await listen('batch-complete', async (event) => {
        const summary = event.payload as string;
        notify(summary, "success");
        clearStatus();
        
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
    
    // --- Preview Logic ---
    const previewSearchInput = document.getElementById('preview-search') as HTMLInputElement;
    const previewFilterSelect = document.getElementById('preview-filter') as HTMLSelectElement;

    if (previewSearchInput) {
        previewSearchInput.addEventListener('input', (e) => {
            previewSearch = (e.target as HTMLInputElement).value.toLowerCase();
            renderPreview();
        });
    }

    if (previewFilterSelect) {
        previewFilterSelect.addEventListener('change', (e) => {
            previewFilter = (e.target as HTMLSelectElement).value as any;
            renderPreview();
        });
    }

    // Handle scroll for virtual scrolling
    if (previewContainer) {
        previewContainer.addEventListener('scroll', () => {
            const newScrollTop = previewContainer!.scrollTop;
            // Only re-render if scrolled significantly (avoid thrashing)
            if (Math.abs(newScrollTop - virtualScrollTop) > ROW_HEIGHT * 5) {
                virtualScrollTop = newScrollTop;
                if (filteredItems.length > VISIBLE_ROWS) {
                    renderPreview();
                }
            }
        });
    }

    await listen('translation-start', (event) => {
        const total = event.payload as number;
        setStatus(`Translation started: 0/${total} blocks...`);
        
        if (previewSection) previewSection.classList.remove("hidden");
        if (recentActivity) recentActivity.classList.add("hidden");
        
        // Reset state
        previewItems = [];
        renderSkeletonPreview();
        if (previewStatus) previewStatus.textContent = `0 / ${total}`;
        
        // Reset stats with skeleton
        if (statsPanel) statsPanel.classList.remove('hidden');
        if (statsFilesCompleted) statsFilesCompleted.innerHTML = '<span class="skeleton rounded px-2">0</span>';
        if (statsBlocksCompleted) statsBlocksCompleted.innerHTML = '<span class="skeleton rounded px-2">0</span>';
    });

    await listen('translation-progress', (event) => {
        const payload = event.payload as { completed: number, total: number, source: string, target: string };
        const { completed, total, source, target } = payload;
        const percent = Math.round((completed / total) * 100);
        
        setStatus(`Translating: ${completed}/${total} (${percent}%)`);
        if (previewStatus) previewStatus.textContent = `${completed} / ${total}`;
        
        // Add to state
        const warnings = checkQuality(source, target);
        previewItems.push({
            id: completed,
            source,
            target,
            warnings
        });
        
        // Virtual scrolling: use efficient append for streaming updates
        if (!previewSearch && previewFilter === 'all' && previewItems.length <= VISIBLE_ROWS) {
            // Small list: direct append
            if (previewContainer) {
                // Remove skeleton if this is the first item
                if (previewItems.length === 1) {
                    previewContainer.innerHTML = '';
                }
                
                const item = previewItems[previewItems.length - 1];
                previewContainer.insertAdjacentHTML('beforeend', renderPreviewRow(item));
                previewContainer.scrollTop = previewContainer.scrollHeight;
            }
        } else {
            // Large list or filter active: use virtual scrolling
            // Scroll to bottom and re-render
            virtualScrollTop = Math.max(0, (previewItems.length - VISIBLE_ROWS) * ROW_HEIGHT);
            renderPreview();
        }
    });

    await listen('translation-complete', (event) => {
        const path = event.payload as string;
        const filename = path.split('/').pop() || path;
        notify(`Translation complete: ${filename}`, "success");
        
        // Add to history
        addToHistory(filename, 0); // TODO: track actual block count
        renderRecentActivity();
        clearStatus();
    });

    // Initialize i18n
    await initI18n();
});
