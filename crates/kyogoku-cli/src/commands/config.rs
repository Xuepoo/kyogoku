use anyhow::{Result, bail};
use kyogoku_core::{ApiClient, Config};

pub async fn show() -> Result<()> {
    let config = Config::load()?;
    let toml = toml::to_string_pretty(&config)?;
    println!("{}", toml);
    Ok(())
}

pub async fn set(key: &str, value: &str) -> Result<()> {
    let mut config = Config::load()?;

    match key {
        "api.provider" => {
            config.api.provider = match value.to_lowercase().as_str() {
                "openai" => kyogoku_core::ApiProvider::OpenAI,
                "deepseek" => kyogoku_core::ApiProvider::DeepSeek,
                "anthropic" => kyogoku_core::ApiProvider::Anthropic,
                "google" => kyogoku_core::ApiProvider::Google,
                "local" => kyogoku_core::ApiProvider::Local,
                "custom" => kyogoku_core::ApiProvider::Custom,
                _ => bail!(
                    "Unknown provider: {}. Use: openai, deepseek, anthropic, google, local, custom",
                    value
                ),
            };
        }
        "api.key" => config.api.api_key = Some(value.to_string()),
        "api.base" => config.api.api_base = Some(value.to_string()),
        "api.model" => config.api.model = value.to_string(),
        "project.source_lang" | "source" | "from" => config.project.source_lang = value.to_string(),
        "project.target_lang" | "target" | "to" => config.project.target_lang = value.to_string(),
        "translation.context_size" => {
            config.translation.context_size = value
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid number: {}", value))?;
        }
        "advanced.max_concurrency" => {
            let n = value
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid number"))?;
            config.advanced.max_concurrency = n;
        }
        "advanced.batch_size" => {
            let n = value
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid number"))?;
            config.advanced.batch_size = n;
        }
        "rag.enabled" => {
            let b = value
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid boolean"))?;
            config.rag.enabled = b;
        }
        _ => bail!("Unknown config key: {}", key),
    }

    config.save()?;
    println!("✓ Set {} = {}", key, value);

    Ok(())
}

pub async fn test() -> Result<()> {
    let config = Config::load()?;

    println!(
        "Testing connection to {} ({})...",
        config.api.get_api_base(),
        config.api.model
    );

    let client = ApiClient::new(config.api)?;
    client.test_connection().await?;

    println!("✓ Connection successful!");

    Ok(())
}
