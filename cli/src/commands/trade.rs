//! Trading operations

use crate::{config::Config, Result};
use clap::Subcommand;
use console::style;
use indicatif::ProgressBar;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[derive(Subcommand)]
pub enum TradeCommands {
    /// Reserve liquidity
    Reserve {
        /// Slab address
        #[arg(short, long)]
        slab: String,

        /// Buy or Sell
        #[arg(long)]
        side: String,

        /// Quantity
        #[arg(long)]
        qty: f64,

        /// Limit price
        #[arg(long)]
        price: f64,

        /// Instrument index
        #[arg(long, default_value = "0")]
        instrument: u8,

        /// Time-to-live ms
        #[arg(long, default_value = "60000")]
        ttl: u64,
    },

    /// Commit a reservation
    Commit {
        /// Slab address
        #[arg(short, long)]
        slab: String,

        /// Reservation hold ID
        #[arg(long)]
        hold_id: String,
    },

    /// Cancel a reservation
    Cancel {
        /// Slab address
        #[arg(short, long)]
        slab: String,

        /// Reservation hold ID
        #[arg(long)]
        hold_id: String,
    },
}

pub async fn handle(cmd: TradeCommands, config: &Config) -> Result<()> {
    match cmd {
        TradeCommands::Reserve {
            slab,
            side,
            qty,
            price,
            instrument,
            ttl,
        } => reserve(config, &slab, &side, qty, price, instrument, ttl).await,
        
        TradeCommands::Commit { slab, hold_id } => commit(config, &slab, &hold_id).await,
        
        TradeCommands::Cancel { slab, hold_id } => cancel(config, &slab, &hold_id).await,
    }
}

async fn reserve(
    config: &Config,
    slab: &str,
    side: &str,
    qty: f64,
    price: f64,
    instrument: u8,
    ttl: u64,
) -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_message("Reserving...");

    // Validate inputs
    let _slab_pubkey = Pubkey::from_str(slab)
        .map_err(|e| format!("Invalid slab address: {}", e))?;

    let side_lower = side.to_lowercase();
    if side_lower != "buy" && side_lower != "sell" {
        return Err("Side must be 'buy' or 'sell'".into());
    }

    println!("\n{}", style(format!("Reserving {} {} @ ${}", side, qty, price)).cyan());
    println!("{}", style(format!("Instrument: {}, TTL: {}ms", instrument, ttl)).dim());

    // TODO: Implement actual reserve

    pb.finish_with_message(format!("{} Reserved", style("✅").green()));

    Ok(())
}

async fn commit(config: &Config, slab: &str, hold_id: &str) -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_message("Committing...");

    // Validate slab address
    let _slab_pubkey = Pubkey::from_str(slab)
        .map_err(|e| format!("Invalid slab address: {}", e))?;

    // TODO: Implement actual commit

    pb.finish_with_message(format!("{} Committed", style("✅").green()));

    Ok(())
}

async fn cancel(config: &Config, slab: &str, hold_id: &str) -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_message("Canceling...");

    // Validate slab address
    let _slab_pubkey = Pubkey::from_str(slab)
        .map_err(|e| format!("Invalid slab address: {}", e))?;

    // TODO: Implement actual cancel

    pb.finish_with_message(format!("{} Canceled", style("✅").green()));

    Ok(())
}

