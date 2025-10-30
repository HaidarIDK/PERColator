//! Configuration management

use crate::error::CliError;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub rpc_url: String,
    pub wallet_path: String,
    pub router_program_id: String,
    pub slab_program_id: String,
    pub amm_program_id: String,
    pub oracle_program_id: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            rpc_url: "https://api.devnet.solana.com".to_string(),
            wallet_path: default_wallet_path(),
            router_program_id: "RoutR1VdCpHqj89WEMJhb6TkGT9cPfr1rVjhM3e2YQr".to_string(),
            slab_program_id: "SLabZ6PsDLh2X6HzEoqxFDMqCVcJXDKCNEYuPzUvGPk".to_string(),
            amm_program_id: "AMMxxx1111111111111111111111111111111111111".to_string(),
            oracle_program_id: "ORACxxx111111111111111111111111111111111111".to_string(),
        }
    }
}

impl Config {
    /// Load config from default location, create if doesn't exist
    pub fn load() -> Result<Self> {
        let config_path = config_path();

        if !config_path.exists() {
            let config = Self::default();
            config.save()?;
            return Ok(config);
        }

        let contents = fs::read_to_string(&config_path)
            .map_err(|e| CliError::Config(format!("Failed to read config: {}", e)))?;

        serde_json::from_str(&contents)
            .map_err(|e| CliError::Config(format!("Failed to parse config: {}", e)))
    }

    /// Save config to default location
    pub fn save(&self) -> Result<()> {
        let config_path = config_path();

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = serde_json::to_string_pretty(self)?;
        fs::write(&config_path, contents)?;

        Ok(())
    }
}

fn config_path() -> PathBuf {
    let mut path = dirs::home_dir().expect("Failed to get home directory");
    path.push(".percolator");
    path.push("config.json");
    path
}

fn default_wallet_path() -> String {
    let mut path = dirs::home_dir().expect("Failed to get home directory");
    path.push(".config");
    path.push("solana");
    path.push("id.json");
    path.to_string_lossy().to_string()
}

pub fn show_config(config: &Config) -> Result<()> {
    use console::style;

    println!("\n{}", style("⚙️  Configuration").cyan().bold());
    println!("{}", style("─".repeat(60)).dim());
    println!("RPC URL: {}", style(&config.rpc_url).green());
    println!("Wallet: {}", style(&config.wallet_path).green());
    println!("Router Program: {}", style(&config.router_program_id).yellow());
    println!("Slab Program: {}", style(&config.slab_program_id).yellow());
    println!("AMM Program: {}", style(&config.amm_program_id).yellow());
    println!("Oracle Program: {}", style(&config.oracle_program_id).yellow());
    println!("\n{}", style(format!("Edit: {}", config_path().display())).dim());

    Ok(())
}

