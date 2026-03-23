use anyhow::Result;
use kyogoku_core::Config;

pub async fn run() -> Result<()> {
    let config = Config::default();
    config.save()?;

    println!("✓ Configuration initialized");

    if let Some(path) = Config::config_path() {
        println!("  Config file: {}", path.display());
    }

    println!("\nNext steps:");
    println!("  1. Set your API provider: kyogoku config set api.provider deepseek");
    println!("  2. Set your API key:      kyogoku config set api.key YOUR_KEY");
    println!("  3. Test connection:       kyogoku config test");
    println!("  4. Start translating:     kyogoku translate ./input -o ./output");

    Ok(())
}
