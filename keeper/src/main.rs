//! Percolator Liquidation Keeper
//!
//! Off-chain service that monitors portfolio health and triggers liquidations
//! for undercollateralized users.

mod config;
mod health;
mod priority_queue;
mod tx_builder;

use anyhow::{Context, Result};
use config::Config;
use priority_queue::{HealthQueue, UserHealth};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use std::time::Duration;
use tokio::time;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Starting Percolator Liquidation Keeper");

    // Load configuration
    let config = Config::load().unwrap_or_else(|_| {
        log::warn!("Failed to load config, using default devnet config");
        Config::default_devnet()
    });

    log::info!("Connected to RPC: {}", config.rpc_url);
    log::info!("Monitoring router program: {}", config.router_program);

    // Initialize RPC client
    let client = RpcClient::new_with_commitment(
        config.rpc_url.clone(),
        CommitmentConfig::confirmed(),
    );

    // Load keeper wallet
    let keeper = load_keypair(&config.keypair_path)?;
    log::info!("Keeper wallet: {}", keeper.pubkey());

    // Initialize health queue
    let mut queue = HealthQueue::new();

    log::info!("Keeper service started. Monitoring for liquidations...");

    // Main event loop
    let mut interval = time::interval(Duration::from_secs(config.poll_interval_secs));

    loop {
        interval.tick().await;

        // Process liquidations
        if let Err(e) = process_liquidations(&mut queue, &client, &config, &keeper).await {
            log::error!("Error processing liquidations: {}", e);
        }

        // Log queue status
        if !queue.is_empty() {
            log::debug!("Health queue size: {}", queue.len());

            if let Some(worst) = queue.peek() {
                log::debug!("Worst health: {}", worst.health as f64 / 1e6);
            }
        }
    }
}

/// Process liquidations for users in the queue
async fn process_liquidations(
    queue: &mut HealthQueue,
    client: &RpcClient,
    config: &Config,
    keeper: &Keypair,
) -> Result<()> {
    // Update health queue with latest portfolio data
    if let Err(e) = update_health_queue(queue, client, config).await {
        log::warn!("Failed to update health queue: {}", e);
    }

    // Get liquidatable users
    let liquidatable = queue.get_liquidatable(config.liquidation_threshold);

    if liquidatable.is_empty() {
        log::debug!("No users need liquidation");
        return Ok(());
    }

    log::info!("Found {} users needing liquidation", liquidatable.len());

    // Process up to max batch size
    let batch_size = config.max_liquidations_per_batch.min(liquidatable.len());

    for user_health in liquidatable.iter().take(batch_size) {
        log::info!(
            "Liquidating user {} (health: {})",
            user_health.user,
            user_health.health as f64 / 1e6
        );

        // Determine if pre-liquidation or hard liquidation
        let is_preliq = user_health.health > 0 && user_health.health < config.preliq_buffer;

        // Build and submit liquidation transaction
        match execute_liquidation(
            client,
            config,
            keeper,
            &user_health.portfolio,
            is_preliq,
        ) {
            Ok(signature) => {
                log::info!("Liquidation submitted: {}", signature);

                // Remove from queue
                queue.remove(&user_health.user);
            }
            Err(e) => {
                log::error!(
                    "Failed to liquidate user {}: {}",
                    user_health.user,
                    e
                );
            }
        }
    }

    Ok(())
}

/// Execute a single liquidation
fn execute_liquidation(
    client: &RpcClient,
    config: &Config,
    keeper: &Keypair,
    portfolio: &Pubkey,
    is_preliq: bool,
) -> Result<String> {
    log::debug!(
        "Executing {} liquidation for portfolio {}",
        if is_preliq { "pre" } else { "hard" },
        portfolio
    );

    // Derive PDAs
    let (registry, _) = tx_builder::derive_registry(&config.router_program);
    let (vault, _) = tx_builder::derive_vault(&config.collateral_mint, &config.router_program);
    let (router_authority, _) = tx_builder::derive_router_authority(&config.router_program);

    // Get current timestamp
    let current_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Fetch recent blockhash
    let recent_blockhash = client
        .get_latest_blockhash()
        .context("Failed to get recent blockhash")?;

    // For v0, we submit with empty oracle/slab/amm lists
    // TODO: In production, fetch portfolio data and include relevant oracles/slabs/amms
    let oracle_accounts = vec![];
    let slab_accounts = vec![];
    let receipt_accounts = vec![];
    let amm_accounts = vec![];

    // Build transaction
    let transaction = tx_builder::build_liquidation_transaction(
        &config.router_program,
        portfolio,
        &registry,
        &vault,
        &router_authority,
        keeper,
        is_preliq,
        current_ts,
        &oracle_accounts,
        &slab_accounts,
        &receipt_accounts,
        &amm_accounts,
        recent_blockhash,
    )?;

    // Submit transaction
    let signature = client
        .send_and_confirm_transaction(&transaction)
        .context("Failed to send liquidation transaction")?;

    log::info!(
        "Liquidation transaction confirmed: {}",
        signature
    );

    Ok(signature.to_string())
}

/// Load keeper keypair from file
fn load_keypair(path: &str) -> Result<Keypair> {
    let expanded_path = shellexpand::tilde(path);
    let bytes = std::fs::read(expanded_path.as_ref())
        .context(format!("Failed to read keypair from {}", path))?;

    let keypair = if bytes[0] == b'[' {
        // JSON format
        let json_data: Vec<u8> = serde_json::from_slice(&bytes)
            .context("Failed to parse keypair JSON")?;
        Keypair::try_from(&json_data[..])
            .context("Failed to create keypair from bytes")?
    } else {
        // Binary format
        Keypair::try_from(&bytes[..])
            .context("Failed to create keypair from bytes")?
    };

    Ok(keypair)
}

/// Fetch portfolio accounts and update health queue
async fn update_health_queue(
    queue: &mut HealthQueue,
    client: &RpcClient,
    config: &Config,
) -> Result<()> {
    use solana_account_decoder::UiAccountEncoding;
    use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};

    log::debug!("Updating health queue from on-chain data");

    // Query all portfolio accounts from the router program
    // Portfolio accounts are discriminated by the first 8 bytes (anchor discriminator)
    // For now, we'll fetch all accounts and filter in code

    let config_filter = RpcProgramAccountsConfig {
        filters: None, // TODO: Add discriminator filter for Portfolio accounts
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            ..Default::default()
        },
        with_context: Some(false),
        sort_results: None,
    };

    let accounts = client
        .get_program_accounts_with_config(&config.router_program, config_filter)
        .context("Failed to fetch portfolio accounts")?;

    log::debug!("Fetched {} accounts from router program", accounts.len());

    let current_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Parse each account and update health queue
    for (pubkey, account) in accounts {
        // Try to parse as portfolio
        match health::parse_portfolio(&account.data) {
            Ok(portfolio) => {
                // Calculate health: equity - MM
                let mm_i128 = portfolio.mm as i128;
                let health = portfolio.equity.saturating_sub(mm_i128);

                // Extract user from portfolio (first 32 bytes after router_id)
                let user = if account.data.len() >= 64 {
                    Pubkey::try_from(&account.data[32..64])
                        .unwrap_or_else(|_| Pubkey::default())
                } else {
                    Pubkey::default()
                };

                let user_health = UserHealth {
                    user,
                    portfolio: pubkey,
                    health,
                    equity: portfolio.equity,
                    mm: portfolio.mm,
                    last_update: current_ts,
                };

                // Update or insert into queue
                queue.push(user_health);

                log::trace!(
                    "Updated portfolio {} (health: {}, equity: {}, mm: {})",
                    pubkey,
                    health as f64 / 1e6,
                    portfolio.equity as f64 / 1e6,
                    portfolio.mm as f64 / 1e6
                );
            }
            Err(e) => {
                // Not a portfolio account or invalid format
                log::trace!("Skipped account {} (not portfolio): {}", pubkey, e);
            }
        }
    }

    log::debug!(
        "Health queue updated: {} portfolios tracked",
        queue.len()
    );

    Ok(())
}
