//! PERColator CLI - Command-line interface for DEX operations
//!
//! This CLI provides complete functionality for:
//! - LP operations (create slabs, manage liquidity)
//! - Trading (reserve, commit, cancel)
//! - Portfolio management
//! - Admin operations
//! - Market making
//! - Monitoring

mod commands;
mod config;
mod error;
mod client;
mod types;
mod utils;

use clap::{Parser, Subcommand};
use commands::*;
use error::CliError;

type Result<T> = std::result::Result<T, CliError>;

#[derive(Parser)]
#[command(name = "perc")]
#[command(about = "PERColator CLI - LP operations, trading, and admin tools", long_about = None)]
#[command(version)]
struct Cli {
    /// RPC URL (overrides config)
    #[arg(long, global = true)]
    rpc_url: Option<String>,

    /// Wallet keypair path (overrides default)
    #[arg(long, global = true)]
    wallet: Option<String>,

    /// Verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Liquidity Provider operations
    #[command(subcommand)]
    Lp(LpCommands),

    /// Trading operations
    #[command(subcommand)]
    Trade(TradeCommands),

    /// Portfolio management
    #[command(subcommand)]
    Portfolio(PortfolioCommands),

    /// Administrative commands
    #[command(subcommand)]
    Admin(AdminCommands),

    /// Market making tools
    #[command(subcommand)]
    Mm(MmCommands),

    /// Position monitoring
    #[command(subcommand)]
    Monitor(MonitorCommands),

    /// Show configuration
    Config,

    /// Check SOL balance
    Balance,

    /// Request devnet SOL airdrop
    Airdrop {
        /// Amount of SOL to request
        #[arg(short, long, default_value = "2")]
        amount: f64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logger
    if cli.verbose {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    } else {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    }

    // Load config
    let mut config = config::Config::load()?;
    
    // Override with CLI args
    if let Some(rpc_url) = cli.rpc_url {
        config.rpc_url = rpc_url;
    }
    if let Some(wallet_path) = cli.wallet {
        config.wallet_path = wallet_path;
    }

    // Execute command
    match cli.command {
        Commands::Lp(cmd) => lp::handle(cmd, &config).await,
        Commands::Trade(cmd) => trade::handle(cmd, &config).await,
        Commands::Portfolio(cmd) => portfolio::handle(cmd, &config).await,
        Commands::Admin(cmd) => admin::handle(cmd, &config).await,
        Commands::Mm(cmd) => mm::handle(cmd, &config).await,
        Commands::Monitor(cmd) => monitor::handle(cmd, &config).await,
        Commands::Config => config::show_config(&config),
        Commands::Balance => utils::check_balance(&config).await,
        Commands::Airdrop { amount } => utils::request_airdrop(&config, amount).await,
    }
}

