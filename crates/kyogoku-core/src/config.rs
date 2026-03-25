use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ApiProvider {
    #[default]
    OpenAI,
    DeepSeek,
    Anthropic,
    Google,
    Local,
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TranslationStyle {
    #[default]
    Literary,
    Casual,
    Formal,
    Technical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub provider: ApiProvider,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub api_base: Option<String>,
    pub model: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
}

fn default_max_tokens() -> u32 {
    4096
}

fn default_temperature() -> f32 {
    0.3
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            provider: ApiProvider::OpenAI,
            api_key: None,
            api_base: None,
            model: "gpt-4o".to_string(),
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
        }
    }
}

impl ApiConfig {
    pub fn get_api_base(&self) -> &str {
        if let Some(ref base) = self.api_base {
            return base.as_str();
        }
        match self.provider {
            ApiProvider::OpenAI => "https://api.openai.com/v1",
            ApiProvider::DeepSeek => "https://api.deepseek.com/v1",
            ApiProvider::Anthropic => "https://api.anthropic.com/v1",
            ApiProvider::Google => "https://generativelanguage.googleapis.com/v1beta",
            ApiProvider::Local => "http://localhost:11434/v1",
            ApiProvider::Custom => "http://localhost:8080/v1",
        }
    }

    /// Load API key from environment variable if set to "ENV_VAR"
    pub fn resolve_api_key(&self) -> Option<String> {
        match &self.api_key {
            Some(key) if key == "ENV_VAR" => {
                let env_var = match self.provider {
                    ApiProvider::OpenAI => "OPENAI_API_KEY",
                    ApiProvider::DeepSeek => "DEEPSEEK_API_KEY",
                    ApiProvider::Anthropic => "ANTHROPIC_API_KEY",
                    ApiProvider::Google => "GOOGLE_API_KEY",
                    _ => "API_KEY",
                };
                // Try provider-specific first, then fallback to common alternatives
                std::env::var(env_var)
                    .or_else(|_| std::env::var("OPENROUTER_API_KEY"))
                    .or_else(|_| std::env::var("LLM_API_KEY"))
                    .ok()
            }
            other => other.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationConfig {
    #[serde(default)]
    pub style: TranslationStyle,
    #[serde(default = "default_context_size")]
    pub context_size: usize,
}

fn default_context_size() -> usize {
    5
}

impl Default for TranslationConfig {
    fn default() -> Self {
        Self {
            style: TranslationStyle::Literary,
            context_size: default_context_size(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedConfig {
    #[serde(default = "default_max_concurrency")]
    pub max_concurrency: usize,
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    #[serde(default)]
    pub allocator: Option<String>,
}

fn default_max_concurrency() -> usize {
    8
}

fn default_batch_size() -> usize {
    5
}

impl Default for AdvancedConfig {
    fn default() -> Self {
        Self {
            max_concurrency: default_max_concurrency(),
            batch_size: default_batch_size(),
            allocator: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub source_lang: String,
    pub target_lang: String,
    #[serde(default)]
    pub glossary_path: Option<PathBuf>,
    #[serde(default)]
    pub input_dir: Option<PathBuf>,
    #[serde(default)]
    pub output_dir: Option<PathBuf>,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            source_lang: "ja".to_string(),
            target_lang: "zh".to_string(),
            glossary_path: None,
            input_dir: None,
            output_dir: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RagConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub model_path: Option<PathBuf>,
    #[serde(default)]
    pub tokenizer_path: Option<PathBuf>,
    #[serde(default)]
    pub vector_store_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub api: ApiConfig,
    #[serde(default)]
    pub translation: TranslationConfig,
    #[serde(default)]
    pub advanced: AdvancedConfig,
    #[serde(default)]
    pub project: ProjectConfig,
    #[serde(default)]
    pub rag: RagConfig,
}

impl Config {
    pub fn config_dir() -> Option<PathBuf> {
        // Follow XDG spec
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            return Some(PathBuf::from(xdg).join("kyogoku"));
        }
        ProjectDirs::from("", "", "kyogoku").map(|dirs| dirs.config_dir().to_path_buf())
    }

    pub fn data_dir() -> Option<PathBuf> {
        if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
            return Some(PathBuf::from(xdg).join("kyogoku"));
        }
        ProjectDirs::from("", "", "kyogoku").map(|dirs| dirs.data_dir().to_path_buf())
    }

    pub fn config_path() -> Option<PathBuf> {
        Self::config_dir().map(|dir| dir.join("config.toml"))
    }

    pub fn cache_path() -> Option<PathBuf> {
        Self::data_dir().map(|dir| dir.join("cache"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path().context("Could not determine config directory")?;

        if !path.exists() {
            tracing::info!("Config file not found, using defaults");
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config from {}", path.display()))?;

        toml::from_str(&content)
            .with_context(|| format!("Failed to parse config from {}", path.display()))
    }

    pub fn save(&self) -> Result<()> {
        let dir = Self::config_dir().context("Could not determine config directory")?;
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create config directory {}", dir.display()))?;

        let path = dir.join("config.toml");
        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;

        std::fs::write(&path, content)
            .with_context(|| format!("Failed to write config to {}", path.display()))?;

        tracing::info!("Config saved to {}", path.display());
        Ok(())
    }

    pub fn load_from_file(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config from {}", path.display()))?;

        toml::from_str(&content)
            .with_context(|| format!("Failed to parse config from {}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.api.provider, ApiProvider::OpenAI);
        assert_eq!(config.project.source_lang, "ja");
        assert_eq!(config.project.target_lang, "zh");
        assert_eq!(config.translation.style, TranslationStyle::Literary);
        assert_eq!(config.advanced.max_concurrency, 8);
    }

    #[test]
    fn test_api_base_url() {
        let mut api = ApiConfig::default();
        assert_eq!(api.get_api_base(), "https://api.openai.com/v1");

        api.provider = ApiProvider::DeepSeek;
        assert_eq!(api.get_api_base(), "https://api.deepseek.com/v1");

        api.api_base = Some("https://custom.api.com".to_string());
        assert_eq!(api.get_api_base(), "https://custom.api.com");
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.api.provider, config.api.provider);
    }
}
