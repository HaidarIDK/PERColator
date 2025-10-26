//! Solana client utilities

use crate::error::CliError;
use crate::Result;
use solana_sdk::signature::{Keypair, Signer};
use std::fs;

/// Load wallet keypair from file
pub fn load_wallet(path: &str) -> Result<Keypair> {
    let secret_key_bytes = fs::read_to_string(path)
        .map_err(|e| CliError::Wallet(format!("Failed to read wallet file: {}", e)))?;

    let secret_key: Vec<u8> = serde_json::from_str(&secret_key_bytes)
        .map_err(|e| CliError::Wallet(format!("Failed to parse wallet file: {}", e)))?;

    Keypair::from_bytes(&secret_key)
        .map_err(|e| CliError::Wallet(format!("Invalid keypair: {}", e)))
}

