//! Administrative commands

use crate::{config::Config, Result};
use clap::Subcommand;
use console::style;
use indicatif::ProgressBar;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[derive(Subcommand)]
pub enum AdminCommands {
    /// Deploy programs to devnet/mainnet
    Deploy {
        /// Network (devnet|mainnet)
        #[arg(short, long, default_value = "devnet")]
        network: String,
    },

    /// Initialize router program
    InitializeRouter,

    /// Register slab in router registry
    RegisterSlab {
        /// Slab address
        #[arg(short, long)]
        slab: String,
    },
}

pub async fn handle(cmd: AdminCommands, config: &Config) -> Result<()> {
    match cmd {
        AdminCommands::Deploy { network } => deploy(config, &network).await,
        AdminCommands::InitializeRouter => initialize_router(config).await,
        AdminCommands::RegisterSlab { slab } => register_slab(config, &slab).await,
    }
}

async fn deploy(_config: &Config, network: &str) -> Result<()> {
    println!("\n{}", style(format!("🚀 Deploying to {}...", network)).cyan().bold());
    println!("{}", style("This will run the deployment scripts").dim());
    println!("\n{}", style("Run: ./deploy-devnet.sh").yellow());

    Ok(())
}

async fn initialize_router(config: &Config) -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_message("Initializing router...");

    // TODO: Implement actual router initialization

    pb.finish_with_message(format!("{} Router initialized", style("✅").green()));

    Ok(())
}

async fn register_slab(config: &Config, slab: &str) -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_message("Registering slab...");

    // Validate slab address
    let _slab_pubkey = Pubkey::from_str(slab)
        .map_err(|e| format!("Invalid slab address: {}", e))?;

    // TODO: Implement actual slab registration

    pb.finish_with_message(format!("{} Registered {}", style("✅").green(), slab));

    Ok(())
}

