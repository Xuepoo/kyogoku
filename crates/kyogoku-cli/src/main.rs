mod commands;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Parser)]
#[command(name = "kyogoku")]
#[command(author, version, about = "AI-powered translation engine for literature and games")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize configuration
    Init,

    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Translate files
    Translate {
        /// Input file or directory
        input: std::path::PathBuf,

        /// Output directory (default: ./output)
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,

        /// Source language (default: from config)
        #[arg(long)]
        from: Option<String>,

        /// Target language (default: from config)
        #[arg(long)]
        to: Option<String>,

        /// Glossary file path
        #[arg(long)]
        glossary: Option<std::path::PathBuf>,

        /// Skip cache lookup
        #[arg(long)]
        no_cache: bool,
    },

    /// Show cache statistics
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Show current configuration
    Show,

    /// Set a configuration value
    Set {
        /// Configuration key (e.g., api.provider, api.model)
        key: String,
        /// Value to set
        value: String,
    },

    /// Test API connection
    Test,
}

#[derive(Subcommand)]
enum CacheAction {
    /// Show cache statistics
    Stats,

    /// Clear the cache
    Clear,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup logging
    let filter = if cli.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .init();

    match cli.command {
        Commands::Init => commands::init::run().await,
        Commands::Config { action } => match action {
            ConfigAction::Show => commands::config::show().await,
            ConfigAction::Set { key, value } => commands::config::set(&key, &value).await,
            ConfigAction::Test => commands::config::test().await,
        },
        Commands::Translate {
            input,
            output,
            from,
            to,
            glossary,
            no_cache,
        } => {
            commands::translate::run(input, output, from, to, glossary, no_cache).await
        }
        Commands::Cache { action } => match action {
            CacheAction::Stats => commands::cache::stats().await,
            CacheAction::Clear => commands::cache::clear().await,
        },
    }
}
