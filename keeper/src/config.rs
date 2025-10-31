//! Keeper configuration

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// RPC URL for Solana cluster
    pub rpc_url: String,

    /// WebSocket URL for event subscription
    pub ws_url: String,

    /// Router program ID
    pub router_program: Pubkey,

    /// Collateral mint (e.g., USDC)
    pub collateral_mint: Pubkey,

    /// Keeper wallet keypair path
    pub keypair_path: String,

    /// Polling interval in seconds
    pub poll_interval_secs: u64,

    /// Pre-liquidation buffer (in 1e6 scale)
    pub preliq_buffer: i128,

    /// Maximum liquidations per batch
    pub max_liquidations_per_batch: usize,

    /// Minimum health to trigger liquidation (negative = below MM)
    pub liquidation_threshold: i128,
}

impl Config {
    /// Load configuration from TOML file
    pub fn load() -> Result<Self> {
        let config_path = std::env::var("KEEPER_CONFIG")
            .unwrap_or_else(|_| "keeper-config.toml".to_string());

        let config_str = std::fs::read_to_string(&config_path)
            .context(format!("Failed to read config file: {}", config_path))?;

        let config: Config = toml::from_str(&config_str)
            .context("Failed to parse config TOML")?;

        Ok(config)
    }

    /// Create default configuration
    pub fn default_devnet() -> Self {
        Self {
            rpc_url: "https://api.devnet.solana.com".to_string(),
            ws_url: "wss://api.devnet.solana.com".to_string(),
            router_program: Pubkey::from_str("RoutR1VdCpHqj89WEMJhb6TkGT9cPfr1rVjhM3e2YQr")
                .unwrap(),
            // Devnet USDC mint
            collateral_mint: Pubkey::from_str("4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU")
                .unwrap(),
            keypair_path: "~/.config/solana/id.json".to_string(),
            poll_interval_secs: 1,
            preliq_buffer: 10_000_000, // $10 buffer for pre-liquidation
            max_liquidations_per_batch: 5,
            liquidation_threshold: 0, // Liquidate if health <= 0
        }
    }

    /// Write default config to file
    pub fn write_default(path: &str) -> Result<()> {
        let config = Self::default_devnet();
        let toml_str = toml::to_string_pretty(&config)
            .context("Failed to serialize config")?;

        std::fs::write(path, toml_str)
            .context(format!("Failed to write config to {}", path))?;

        log::info!("Created default config at {}", path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_creation() {
        let config = Config::default_devnet();
        assert_eq!(config.rpc_url, "https://api.devnet.solana.com");
        assert_eq!(config.poll_interval_secs, 1);
    }
}
