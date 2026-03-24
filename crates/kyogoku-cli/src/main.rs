mod commands;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[derive(Parser)]
#[command(name = "kyogoku")]
#[command(
    author,
    version,
    about = "AI-powered translation engine for literature and games"
)]
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

        /// Preview translation without making API calls
        #[arg(long)]
        dry_run: bool,

        /// Force specific parser format (txt, srt, json, ass, vtt, epub)
        #[arg(long)]
        format: Option<String>,

        /// Output results as JSON (for scripting)
        #[arg(long)]
        json: bool,
    },

    /// Show cache statistics
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },

    /// Manage plugins
    Plugin {
        #[command(subcommand)]
        action: PluginAction,
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

#[derive(Subcommand)]
enum PluginAction {
    /// List installed plugins
    List,

    /// Show plugin details
    Info {
        /// Plugin name
        name: String,
    },

    /// Show plugin directories
    Dirs,
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
            dry_run,
            format,
            json,
        } => {
            commands::translate::run(
                input, output, from, to, glossary, no_cache, dry_run, format, json,
            )
            .await
        }
        Commands::Cache { action } => match action {
            CacheAction::Stats => commands::cache::stats().await,
            CacheAction::Clear => commands::cache::clear().await,
        },
        Commands::Plugin { action } => match action {
            PluginAction::List => commands::plugin::list().await,
            PluginAction::Info { name } => commands::plugin::info(&name).await,
            PluginAction::Dirs => commands::plugin::dirs().await,
        },
    }
}
