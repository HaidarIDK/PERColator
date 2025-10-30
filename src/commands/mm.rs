//! Market making tools

use crate::{config::Config, Result};
use clap::Subcommand;
use console::style;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[derive(Subcommand)]
pub enum MmCommands {
    /// Post two-sided quote
    Quote {
        /// Slab address
        #[arg(short, long)]
        slab: String,

        /// Mid price
        #[arg(long)]
        mid: f64,

        /// Spread in bps
        #[arg(long)]
        spread: u32,

        /// Size per side
        #[arg(long)]
        size: f64,
    },

    /// Watch market and auto-quote
    Watch {
        /// Slab address
        #[arg(short, long)]
        slab: String,

        /// Target spread bps
        #[arg(long, default_value = "10")]
        spread: u32,

        /// Size per side
        #[arg(long, default_value = "1")]
        size: f64,
    },
}

pub async fn handle(cmd: MmCommands, config: &Config) -> Result<()> {
    match cmd {
        MmCommands::Quote {
            slab,
            mid,
            spread,
            size,
        } => quote(config, &slab, mid, spread, size).await,
        
        MmCommands::Watch {
            slab,
            spread,
            size,
        } => watch(config, &slab, spread, size).await,
    }
}

async fn quote(config: &Config, slab: &str, mid: f64, spread: u32, size: f64) -> Result<()> {
    // Validate slab address
    let _slab_pubkey = Pubkey::from_str(slab)
        .map_err(|e| format!("Invalid slab address: {}", e))?;

    println!("\n{}", style("📊 Posting quote...").cyan().bold());
    println!("Mid: ${}, Spread: {}bps, Size: {}", mid, spread, size);

    let bid_price = mid * (1.0 - spread as f64 / 10000.0);
    let ask_price = mid * (1.0 + spread as f64 / 10000.0);

    println!("{} {} x {}", style("Bid:").green(), style(format!("${:.2}", bid_price)).bold(), size);
    println!("{} {} x {}", style("Ask:").red(), style(format!("${:.2}", ask_price)).bold(), size);

    // TODO: Post orders

    Ok(())
}

async fn watch(config: &Config, slab: &str, spread: u32, size: f64) -> Result<()> {
    // Validate slab address
    let _slab_pubkey = Pubkey::from_str(slab)
        .map_err(|e| format!("Invalid slab address: {}", e))?;

    println!("\n{}", style("🤖 Market Making Bot Starting...").cyan().bold());
    println!("Slab: {}", slab);
    println!("Spread: {}bps, Size: {}", spread, size);
    println!("{}", style("\nPress Ctrl+C to stop\n").dim());

    // TODO: Implement market making loop

    Ok(())
}

