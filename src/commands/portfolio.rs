//! Portfolio management

use crate::{config::Config, Result};
use clap::Subcommand;
use console::style;
use indicatif::ProgressBar;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use std::str::FromStr;

#[derive(Subcommand)]
pub enum PortfolioCommands {
    /// Show portfolio summary
    Show {
        /// User address (default: wallet)
        #[arg(short, long)]
        user: Option<String>,
    },

    /// List all positions
    Positions,
}

pub async fn handle(cmd: PortfolioCommands, config: &Config) -> Result<()> {
    match cmd {
        PortfolioCommands::Show { user } => show_portfolio(config, user).await,
        PortfolioCommands::Positions => show_positions(config).await,
    }
}

async fn show_portfolio(config: &Config, user: Option<String>) -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_message("Loading portfolio...");

    // Load wallet to get default user
    let wallet = crate::client::load_wallet(&config.wallet_path)?;
    let user_pubkey = if let Some(user_str) = user {
        Pubkey::from_str(&user_str)
            .map_err(|e| format!("Invalid user address: {}", e))?
    } else {
        wallet.pubkey()
    };

    // TODO: Fetch actual portfolio data

    pb.finish_and_clear();

    println!("\n{}", style("📊 Portfolio Summary").cyan().bold());
    println!("{}", style("─".repeat(60)).dim());
    println!("User: {}", user_pubkey);
    println!("Equity: {}", style("$0.00").green());
    println!("Free Collateral: {}", style("$0.00").yellow());
    println!("Positions: {}", style("0").blue());

    Ok(())
}

async fn show_positions(config: &Config) -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_message("Loading positions...");

    // TODO: Fetch actual positions

    pb.finish_and_clear();

    println!("\n{}", style("📈 Open Positions").cyan().bold());
    println!("{}", style("─".repeat(60)).dim());
    println!("No positions");

    Ok(())
}

