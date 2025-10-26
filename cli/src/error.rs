//! CLI error types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Wallet error: {0}")]
    Wallet(String),

    #[error("RPC error: {0}")]
    Rpc(String),

    #[error("Transaction error: {0}")]
    Transaction(String),

    #[error("Program error: {0}")]
    Program(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Solana client error: {0}")]
    SolanaClient(#[from] solana_client::client_error::ClientError),

    #[error("Solana SDK error: {0}")]
    SolanaSdk(#[from] solana_sdk::pubkey::ParsePubkeyError),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("{0}")]
    Custom(String),
}

impl From<String> for CliError {
    fn from(s: String) -> Self {
        CliError::Custom(s)
    }
}

impl From<&str> for CliError {
    fn from(s: &str) -> Self {
        CliError::Custom(s.to_string())
    }
}

