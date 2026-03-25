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

    /// Validate and sanitize configuration values
    pub fn validate(&self) -> Result<()> {
        // Validate model name (alphanumeric, dash, underscore, dot)
        if !self
            .model
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            anyhow::bail!(
                "Invalid model name: must contain only alphanumeric characters, dash, underscore, or dot"
            );
        }

        // Validate model name length
        if self.model.is_empty() || self.model.len() > 100 {
            anyhow::bail!("Invalid model name: must be between 1 and 100 characters");
        }

        // Validate max_tokens range
        if self.max_tokens == 0 || self.max_tokens > 1_000_000 {
            anyhow::bail!("Invalid max_tokens: must be between 1 and 1,000,000");
        }

        // Validate temperature range
        if !(0.0..=2.0).contains(&self.temperature) {
            anyhow::bail!("Invalid temperature: must be between 0.0 and 2.0");
        }

        // Validate API base if custom
        if let Some(ref base) = self.api_base {
            if base.is_empty() || base.len() > 500 {
                anyhow::bail!("Invalid API base URL: must be between 1 and 500 characters");
            }
            // Basic URL validation
            if !base.starts_with("http://") && !base.starts_with("https://") {
                anyhow::bail!("Invalid API base URL: must start with http:// or https://");
            }
        }

        // Validate API key if set (not ENV_VAR)
        if let Some(ref key) = self.api_key {
            if key != "ENV_VAR" {
                if key.is_empty() || key.len() > 500 {
                    anyhow::bail!("Invalid API key: must be between 1 and 500 characters");
                }
                // Check for suspicious patterns (shell injection attempts)
                if key.contains(|c: char| c == '\0' || c == '\n' || c == '\r') {
                    anyhow::bail!("Invalid API key: contains invalid characters");
                }
            }
        }

        Ok(())
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

impl AdvancedConfig {
    /// Validate advanced configuration values
    pub fn validate(&self) -> Result<()> {
        // Validate max_concurrency
        if self.max_concurrency == 0 || self.max_concurrency > 100 {
            anyhow::bail!("Invalid max_concurrency: must be between 1 and 100");
        }

        // Validate batch_size
        if self.batch_size == 0 || self.batch_size > 1000 {
            anyhow::bail!("Invalid batch_size: must be between 1 and 1000");
        }

        Ok(())
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

impl ProjectConfig {
    /// Validate project configuration
    pub fn validate(&self) -> Result<()> {
        // Validate language codes (2-3 letter ISO codes)
        if !is_valid_lang_code(&self.source_lang) {
            anyhow::bail!(
                "Invalid source_lang: must be 2-3 letter language code (e.g., 'ja', 'en', 'zh')"
            );
        }
        if !is_valid_lang_code(&self.target_lang) {
            anyhow::bail!(
                "Invalid target_lang: must be 2-3 letter language code (e.g., 'ja', 'en', 'zh')"
            );
        }

        // Prevent same source and target
        if self.source_lang == self.target_lang {
            anyhow::bail!("source_lang and target_lang must be different");
        }

        Ok(())
    }
}

fn is_valid_lang_code(code: &str) -> bool {
    // Allow 2-3 letter codes, optionally with region (e.g., zh-CN, en-US)
    if code.len() < 2 || code.len() > 8 {
        return false;
    }
    // Basic validation: letters, dash, and numbers only
    code.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
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

        let config: Self = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config from {}", path.display()))?;

        // Validate configuration after loading
        config.validate()?;

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        // Validate before saving
        self.validate()?;

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

    /// Validate all configuration sections
    pub fn validate(&self) -> Result<()> {
        self.api
            .validate()
            .context("API configuration validation failed")?;
        self.project
            .validate()
            .context("Project configuration validation failed")?;
        self.advanced
            .validate()
            .context("Advanced configuration validation failed")?;
        Ok(())
    }

    pub fn load_from_file(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config from {}", path.display()))?;

        let config: Self = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config from {}", path.display()))?;

        config.validate()?;
        Ok(config)
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

    #[test]
    fn test_api_config_validation() {
        let mut api = ApiConfig::default();

        // Valid config should pass
        assert!(api.validate().is_ok());

        // Invalid model name (empty)
        api.model = "".to_string();
        assert!(api.validate().is_err());

        // Invalid model name (too long)
        api.model = "a".repeat(101);
        assert!(api.validate().is_err());

        // Invalid model name (special chars)
        api.model = "gpt-4o; rm -rf /".to_string();
        assert!(api.validate().is_err());

        // Valid model name
        api.model = "gpt-4o-mini".to_string();
        assert!(api.validate().is_ok());

        // Invalid max_tokens
        api.max_tokens = 0;
        assert!(api.validate().is_err());
        api.max_tokens = 1_000_001;
        assert!(api.validate().is_err());
        api.max_tokens = 4096;
        assert!(api.validate().is_ok());

        // Invalid temperature
        api.temperature = -0.1;
        assert!(api.validate().is_err());
        api.temperature = 2.1;
        assert!(api.validate().is_err());
        api.temperature = 0.7;
        assert!(api.validate().is_ok());

        // Invalid API base (no protocol)
        api.api_base = Some("example.com".to_string());
        assert!(api.validate().is_err());

        // Valid API base
        api.api_base = Some("https://api.example.com".to_string());
        assert!(api.validate().is_ok());

        // Invalid API key (null bytes)
        api.api_key = Some("key\0with\0nulls".to_string());
        assert!(api.validate().is_err());
    }

    #[test]
    fn test_project_config_validation() {
        let mut project = ProjectConfig::default();

        // Valid config should pass
        assert!(project.validate().is_ok());

        // Invalid language code (too short)
        project.source_lang = "j".to_string();
        assert!(project.validate().is_err());

        // Invalid language code (too long)
        project.source_lang = "toolongcode".to_string();
        assert!(project.validate().is_err());

        // Invalid language code (special chars)
        project.source_lang = "ja; echo".to_string();
        assert!(project.validate().is_err());

        // Valid language code
        project.source_lang = "ja".to_string();
        assert!(project.validate().is_ok());

        // Valid language code with region
        project.source_lang = "zh-CN".to_string();
        assert!(project.validate().is_ok());

        // Same source and target
        project.target_lang = "zh-CN".to_string();
        assert!(project.validate().is_err());
    }

    #[test]
    fn test_advanced_config_validation() {
        let mut advanced = AdvancedConfig::default();

        // Valid config should pass
        assert!(advanced.validate().is_ok());

        // Invalid max_concurrency
        advanced.max_concurrency = 0;
        assert!(advanced.validate().is_err());
        advanced.max_concurrency = 101;
        assert!(advanced.validate().is_err());
        advanced.max_concurrency = 8;
        assert!(advanced.validate().is_ok());

        // Invalid batch_size
        advanced.batch_size = 0;
        assert!(advanced.validate().is_err());
        advanced.batch_size = 1001;
        assert!(advanced.validate().is_err());
        advanced.batch_size = 5;
        assert!(advanced.validate().is_ok());
    }
}
