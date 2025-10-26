//! Liquidation operations

use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Signer,
    transaction::Transaction,
};
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{client, config::NetworkConfig};

/// Derive portfolio PDA for a user
/// Matches router/src/pda.rs::derive_portfolio_pda
fn derive_portfolio_pda(user: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"portfolio", user.as_ref()], program_id)
}

/// Derive registry PDA
fn derive_registry_pda(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"registry"], program_id)
}

/// Derive router authority PDA
fn derive_router_authority_pda(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"router_authority"], program_id)
}

/// Derive vault PDA
fn derive_vault_pda(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"vault"], program_id)
}

pub async fn execute_liquidation(
    config: &NetworkConfig,
    user: String,
    max_size: Option<u64>,
) -> Result<()> {
    println!("{}", "=== Execute Liquidation ===".bright_green().bold());
    println!("{} {}", "User:".bright_cyan(), user);
    if let Some(size) = max_size {
        println!("{} {}", "Max Size:".bright_cyan(), size);
    }

    // Parse user pubkey
    let user_pubkey = Pubkey::from_str(&user)
        .context("Invalid user pubkey")?;

    // Derive portfolio PDA
    let (portfolio_pda, _) = derive_portfolio_pda(&user_pubkey, &config.router_program_id);
    println!("{} {}", "Portfolio PDA:".bright_cyan(), portfolio_pda);

    // Fetch portfolio account
    println!("\n{}", "Fetching portfolio account...".dimmed());
    let rpc_client = client::create_rpc_client(config);

    let account = rpc_client.get_account(&portfolio_pda)
        .context("Failed to fetch portfolio account - does it exist?")?;

    // Verify account size
    let expected_size = percolator_router::state::Portfolio::LEN;
    if account.data.len() != expected_size {
        return Err(anyhow!(
            "Invalid portfolio account size: expected {}, got {}",
            expected_size,
            account.data.len()
        ));
    }

    // SAFETY: Portfolio has #[repr(C)] and we verified the size matches exactly
    let portfolio = unsafe {
        &*(account.data.as_ptr() as *const percolator_router::state::Portfolio)
    };

    // Display portfolio state
    let equity_sol = portfolio.equity as f64 / 1_000_000_000.0;
    let health_sol = portfolio.health as f64 / 1_000_000_000.0;
    let im_sol = portfolio.im as f64 / 1_000_000_000.0;
    let mm_sol = portfolio.mm as f64 / 1_000_000_000.0;

    println!("\n{}", "Portfolio Status:".bright_yellow().bold());
    println!("  {} {:.4} SOL", "Equity:".bright_cyan(), equity_sol);
    println!("  {} {:.4} SOL", "Health:".bright_cyan(), health_sol);
    println!("  {} {:.4} SOL", "Initial Margin:".bright_cyan(), im_sol);
    println!("  {} {:.4} SOL", "Maintenance Margin:".bright_cyan(), mm_sol);

    // Check if liquidatable
    if portfolio.health >= 0 {
        println!("\n{}", "Portfolio is NOT liquidatable (health >= 0)".green());
        println!("{}", "Liquidation can only be executed when health < 0".dimmed());
        return Ok(());
    }

    println!("\n{} {}", "Portfolio is liquidatable!".bright_red().bold(), "(health < 0)".red());

    // Build and execute liquidation transaction
    println!("\n{}", "Building liquidation transaction...".dimmed());

    let (registry_pda, _) = derive_registry_pda(&config.router_program_id);
    let (router_authority_pda, _) = derive_router_authority_pda(&config.router_program_id);
    let (vault_pda, _) = derive_vault_pda(&config.router_program_id);

    // Get current timestamp
    let current_ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs();

    // Instruction data layout (from entrypoint.rs:384-390):
    // - num_oracles: u8 (1 byte)
    // - num_slabs: u8 (1 byte)
    // - is_preliq: u8 (1 byte, 0 = auto, 1 = force pre-liq)
    // - current_ts: u64 (8 bytes)
    let num_oracles: u8 = 0; // No oracles configured yet
    let num_slabs: u8 = 0;   // No slabs configured yet
    let is_preliq: u8 = 0;   // Auto-determine mode

    let mut instruction_data = Vec::with_capacity(11);
    instruction_data.push(5u8); // RouterInstruction::LiquidateUser discriminator
    instruction_data.push(num_oracles);
    instruction_data.push(num_slabs);
    instruction_data.push(is_preliq);
    instruction_data.extend_from_slice(&current_ts.to_le_bytes());

    // Build account list (from entrypoint.rs:375-382)
    // Note: Oracle and slab accounts would be added here when configured
    let accounts = vec![
        AccountMeta::new(portfolio_pda, false),               // Portfolio (writable)
        AccountMeta::new(registry_pda, false),                // Registry (writable)
        AccountMeta::new(vault_pda, false),                   // Vault (writable)
        AccountMeta::new_readonly(router_authority_pda, false), // Router authority
        // Oracle and slab accounts would be appended here based on num_oracles/num_slabs
    ];

    let liquidate_ix = Instruction {
        program_id: config.router_program_id,
        accounts,
        data: instruction_data,
    };

    // Build and send transaction
    println!("{}", "Sending transaction...".dimmed());
    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let transaction = Transaction::new_signed_with_payer(
        &[liquidate_ix],
        Some(&config.pubkey()),
        &[&config.keypair],
        recent_blockhash,
    );

    match rpc_client.send_and_confirm_transaction(&transaction) {
        Ok(signature) => {
            println!("\n{} Liquidation executed successfully!", "✓".green().bold());
            println!("{} {}", "Transaction:".bright_cyan(), signature);

            if max_size.is_some() {
                println!("\n{}", "Note: max_size parameter requires slab integration".dimmed());
            }
        }
        Err(e) => {
            println!("\n{} Liquidation failed: {}", "✗".red().bold(), e);
            println!("\n{}", "Common causes:".bright_yellow());
            println!("  {} Oracle infrastructure not configured", "•".dimmed());
            println!("  {} Slab infrastructure not configured", "•".dimmed());
            println!("  {} Vault address not properly set", "•".dimmed());
            println!("  {} Portfolio may have been liquidated by another keeper", "•".dimmed());
            return Err(anyhow!("Liquidation transaction failed: {}", e));
        }
    }

    Ok(())
}

pub async fn list_liquidatable(config: &NetworkConfig, _exchange: String) -> Result<()> {
    println!("{}", "=== Liquidatable Accounts ===".bright_green().bold());
    println!("{} {}", "Network:".bright_cyan(), config.network);

    let rpc_client = client::create_rpc_client(config);

    // Get all portfolio accounts using getProgramAccounts
    println!("\n{}", "Scanning for portfolio accounts...".dimmed());

    let accounts = rpc_client.get_program_accounts(&config.router_program_id)
        .context("Failed to fetch program accounts")?;

    if accounts.is_empty() {
        println!("{}", "No portfolio accounts found".yellow());
        return Ok(());
    }

    println!("{} {} portfolio accounts found", "Found".bright_cyan(), accounts.len());

    let mut liquidatable_count = 0;

    for (pubkey, account) in accounts {
        // Verify account size matches Portfolio
        let expected_size = percolator_router::state::Portfolio::LEN;
        if account.data.len() != expected_size {
            continue; // Skip non-portfolio accounts
        }

        // SAFETY: Portfolio has #[repr(C)] and we verified the size matches exactly
        let portfolio = unsafe {
            &*(account.data.as_ptr() as *const percolator_router::state::Portfolio)
        };

        // Check if liquidatable (health < 0)
        if portfolio.health < 0 {
            liquidatable_count += 1;

            let equity_sol = portfolio.equity as f64 / 1_000_000_000.0;
            let health_sol = portfolio.health as f64 / 1_000_000_000.0;
            let im_sol = portfolio.im as f64 / 1_000_000_000.0;
            let mm_sol = portfolio.mm as f64 / 1_000_000_000.0;
            let free_collateral_sol = portfolio.free_collateral as f64 / 1_000_000_000.0;

            // Convert pinocchio Pubkey ([u8; 32]) to Solana SDK Pubkey for display
            let user_pubkey = Pubkey::new_from_array(portfolio.user);

            println!("\n{} {}", "Liquidatable:".bright_red().bold(), pubkey);
            println!("  {} {}", "User:".bright_cyan(), user_pubkey);
            println!("  {} {:.4} SOL", "Equity:".bright_cyan(), equity_sol);
            println!("  {} {:.4} SOL {}", "Health:".bright_red(), health_sol, "(UNDERWATER)".bright_red());
            println!("  {} {:.4} SOL", "Initial Margin:".bright_cyan(), im_sol);
            println!("  {} {:.4} SOL", "Maintenance Margin:".bright_cyan(), mm_sol);
            println!("  {} {:.4} SOL", "Free Collateral:".bright_cyan(), free_collateral_sol);
            println!("  {} {}", "Exposure Count:".bright_cyan(), portfolio.exposure_count);
        }
    }

    println!();
    if liquidatable_count == 0 {
        println!("{}", "No liquidatable accounts found".green());
    } else {
        println!("{} {} {} liquidatable",
            "Found".bright_red().bold(),
            liquidatable_count,
            if liquidatable_count == 1 { "account" } else { "accounts" }
        );
    }

    Ok(())
}

pub async fn show_history(config: &NetworkConfig, limit: usize) -> Result<()> {
    println!("{}", "=== Liquidation History ===".bright_green().bold());
    println!("{} {}", "Network:".bright_cyan(), config.network);
    println!("{} {}", "Limit:".bright_cyan(), limit);

    println!("\n{}", "Liquidation Event Tracking:".bright_yellow().bold());
    println!("  {} Liquidation history requires event log monitoring", "ℹ".bright_cyan());
    println!("  {} Events are emitted when liquidations are executed", "ℹ".bright_cyan());

    println!("\n{}", "To track liquidation history:".bright_yellow());
    println!("  {} Monitor program logs for LiquidateUser events", "1.".bright_cyan());
    println!("  {} Parse event data from transaction logs", "2.".bright_cyan());
    println!("  {} Store events in database (PostgreSQL, SQLite, etc.)", "3.".bright_cyan());
    println!("  {} Query stored events for historical analysis", "4.".bright_cyan());

    println!("\n{}", "Alternative approaches:".bright_yellow());
    println!("  {} Query recent transactions via RPC", "•".dimmed());
    println!("  {} Use Solana transaction history API", "•".dimmed());
    println!("  {} Subscribe to program logs via WebSocket", "•".dimmed());
    println!("  {} Use indexing services (e.g., The Graph, Helius)", "•".dimmed());

    println!("\n{}", "Example event monitoring setup:".bright_green().bold());
    println!("  {}", "// Subscribe to program logs".dimmed());
    println!("  {}", "let (mut notifications, _) = client.logs_subscribe(".dimmed());
    println!("  {}", "    RpcTransactionLogsFilter::Mentions(vec![program_id.to_string()]),".dimmed());
    println!("  {}", "    RpcTransactionLogsConfig { commitment: Some(CommitmentConfig::confirmed()) }".dimmed());
    println!("  {}", ")?;".dimmed());
    println!();
    println!("  {}", "// Process incoming logs".dimmed());
    println!("  {}", "while let Some(log) = notifications.next() {".dimmed());
    println!("  {}", "    if log.value.logs.iter().any(|l| l.contains(\"LiquidateUser\")) {".dimmed());
    println!("  {}", "        // Parse and store liquidation event".dimmed());
    println!("  {}", "    }".dimmed());
    println!("  {}", "}".dimmed());

    println!("\n{}", "Production Implementation Checklist:".bright_yellow().bold());
    println!("  {} Set up event listener daemon/service", "☐".dimmed());
    println!("  {} Configure database schema for events", "☐".dimmed());
    println!("  {} Implement event parser for LiquidateUser logs", "☐".dimmed());
    println!("  {} Add indexing by user, timestamp, and amount", "☐".dimmed());
    println!("  {} Create API for querying historical events", "☐".dimmed());
    println!("  {} Add metrics and monitoring for liquidations", "☐".dimmed());

    Ok(())
}
