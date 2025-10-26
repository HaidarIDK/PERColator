//! Position monitoring

use crate::{config::Config, Result};
use clap::Subcommand;
use console::style;

#[derive(Subcommand)]
pub enum MonitorCommands {
    /// Monitor account equity
    Equity {
        /// User to monitor
        #[arg(short, long)]
        user: Option<String>,

        /// Update interval in seconds
        #[arg(short, long, default_value = "5")]
        interval: u64,
    },

    /// Monitor liquidation opportunities
    Liquidations {
        /// Minimum profit threshold
        #[arg(short, long, default_value = "100")]
        min_profit: f64,
    },
}

pub async fn handle(cmd: MonitorCommands, config: &Config) -> Result<()> {
    match cmd {
        MonitorCommands::Equity { user, interval } => monitor_equity(config, user, interval).await,
        MonitorCommands::Liquidations { min_profit } => monitor_liquidations(config, min_profit).await,
    }
}

async fn monitor_equity(_config: &Config, user: Option<String>, interval: u64) -> Result<()> {
    println!("{}", style("📊 Monitoring Equity").cyan().bold());
    if let Some(user_addr) = user {
        println!("User: {}", user_addr);
    }
    println!("Interval: {}s", interval);
    println!("{}", style("Press Ctrl+C to stop\n").dim());

    // TODO: Implement equity monitoring loop

    Ok(())
}

async fn monitor_liquidations(_config: &Config, min_profit: f64) -> Result<()> {
    println!("{}", style("🔍 Monitoring Liquidations").cyan().bold());
    println!("Min Profit: ${}", min_profit);
    println!("{}", style("Press Ctrl+C to stop\n").dim());

    // TODO: Implement liquidation monitoring

    Ok(())
}

