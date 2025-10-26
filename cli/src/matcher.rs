//! Matcher/slab management operations

use anyhow::{Context, Result};
use colored::Colorize;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signer::Signer,
    transaction::Transaction,
};
use std::str::FromStr;

use crate::{client, config::NetworkConfig};

/// Register a slab in the router registry
///
/// This allows the router to route orders to the slab
pub async fn register_slab(
    config: &NetworkConfig,
    registry_address: String,
    slab_id: String,
    oracle_id: String,
    imr_bps: u64,           // Initial margin ratio in basis points (e.g., 500 = 5%)
    mmr_bps: u64,           // Maintenance margin ratio in basis points
    maker_fee_bps: u64,     // Maker fee cap in basis points
    taker_fee_bps: u64,     // Taker fee cap in basis points
    latency_sla_ms: u64,    // Latency SLA in milliseconds
    max_exposure: u128,     // Maximum position exposure
) -> Result<()> {
    println!("{}", "=== Register Slab ===".bright_green().bold());
    println!("{} {}", "Network:".bright_cyan(), config.network);
    println!("{} {}", "Registry:".bright_cyan(), registry_address);
    println!("{} {}", "Slab ID:".bright_cyan(), slab_id);
    println!("{} {}", "Oracle ID:".bright_cyan(), oracle_id);
    println!("{} {}bps ({}%)", "IMR:".bright_cyan(), imr_bps, imr_bps as f64 / 100.0);
    println!("{} {}bps ({}%)", "MMR:".bright_cyan(), mmr_bps, mmr_bps as f64 / 100.0);

    // Parse addresses
    let registry = Pubkey::from_str(&registry_address)
        .context("Invalid registry address")?;
    let slab = Pubkey::from_str(&slab_id)
        .context("Invalid slab ID")?;
    let oracle = Pubkey::from_str(&oracle_id)
        .context("Invalid oracle ID")?;

    // Get RPC client and governance keypair (payer)
    let rpc_client = client::create_rpc_client(config);
    let governance = &config.keypair;

    println!("\n{} {}", "Governance:".bright_cyan(), governance.pubkey());

    // Build instruction data: [discriminator(8), slab_id(32), version_hash(32), oracle_id(32),
    //                           imr(8), mmr(8), maker_fee(8), taker_fee(8), latency(8), exposure(16)]
    let mut instruction_data = Vec::with_capacity(153);
    instruction_data.push(8u8); // RegisterSlab discriminator
    instruction_data.extend_from_slice(&slab.to_bytes());
    instruction_data.extend_from_slice(&[0u8; 32]); // version_hash (placeholder)
    instruction_data.extend_from_slice(&oracle.to_bytes());
    instruction_data.extend_from_slice(&imr_bps.to_le_bytes());
    instruction_data.extend_from_slice(&mmr_bps.to_le_bytes());
    instruction_data.extend_from_slice(&maker_fee_bps.to_le_bytes());
    instruction_data.extend_from_slice(&taker_fee_bps.to_le_bytes());
    instruction_data.extend_from_slice(&latency_sla_ms.to_le_bytes());
    instruction_data.extend_from_slice(&max_exposure.to_le_bytes());

    // Build RegisterSlab instruction
    let register_ix = Instruction {
        program_id: config.router_program_id,
        accounts: vec![
            AccountMeta::new(registry, false),            // Registry account (writable)
            AccountMeta::new(governance.pubkey(), true),  // Governance (signer, writable)
        ],
        data: instruction_data,
    };

    // Send transaction
    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let transaction = Transaction::new_signed_with_payer(
        &[register_ix],
        Some(&governance.pubkey()),
        &[governance],
        recent_blockhash,
    );

    println!("{}", "Sending RegisterSlab transaction...".bright_green());
    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .context("Failed to send RegisterSlab transaction")?;

    println!("\n{} {}", "Success!".bright_green().bold(), "✓".bright_green());
    println!("{} {}", "Signature:".bright_cyan(), signature);
    println!("{}", "Slab registered successfully".bright_green());

    Ok(())
}

pub async fn create_matcher(
    _config: &NetworkConfig,
    exchange: String,
    symbol: String,
    tick_size: u64,
    lot_size: u64,
) -> Result<()> {
    println!("{}", "=== Create Matcher ===".bright_green().bold());
    println!("{} {}", "Exchange:".bright_cyan(), exchange);
    println!("{} {}", "Symbol:".bright_cyan(), symbol);
    println!("{} {}", "Tick Size:".bright_cyan(), tick_size);
    println!("{} {}", "Lot Size:".bright_cyan(), lot_size);

    println!("\n{}", "Matcher creation not yet implemented".yellow());
    println!("{}", "Use 'percolator matcher register-slab' to register an existing slab".dimmed());
    Ok(())
}

pub async fn list_matchers(config: &NetworkConfig, registry_address: String) -> Result<()> {
    println!("{}", "=== List Registered Slabs ===".bright_green().bold());
    println!("{} {}", "Network:".bright_cyan(), config.network);
    println!("{} {}", "Registry:".bright_cyan(), registry_address);

    // Parse registry address
    let registry = Pubkey::from_str(&registry_address)
        .context("Invalid registry address")?;

    // Get RPC client
    let rpc_client = client::create_rpc_client(config);

    // Fetch account data
    let account = rpc_client
        .get_account(&registry)
        .context("Failed to fetch registry account")?;

    // Verify ownership
    if account.owner != config.router_program_id {
        anyhow::bail!("Account is not owned by router program");
    }

    // Deserialize registry data
    const REGISTRY_SIZE_BPF: usize = 43688;
    if account.data.len() != REGISTRY_SIZE_BPF {
        println!("\n{} Registry size: {} bytes", "Warning:".yellow(), account.data.len());
    }

    let registry_data = unsafe {
        &*(account.data.as_ptr() as *const percolator_router::state::SlabRegistry)
    };

    println!("\n{} {}", "Total Registered Slabs:".bright_cyan(), registry_data.slab_count);

    if registry_data.slab_count == 0 {
        println!("{}", "\nNo slabs registered yet".dimmed());
        return Ok(());
    }

    if registry_data.slab_count > 0 {
        println!("\n{}", "=== Registered Slabs ===".bright_yellow());
        for i in 0..registry_data.slab_count as usize {
            let slab = &registry_data.slabs[i];

            println!("\n{} {}", "Slab #".bright_green(), i);
            // Convert pinocchio Pubkeys to SDK Pubkeys for display (same as status command)
            let slab_id_sdk = Pubkey::new_from_array(slab.slab_id);
            let oracle_id_sdk = Pubkey::new_from_array(slab.oracle_id);

            println!("  {} {}", "Slab ID:".bright_cyan(), slab_id_sdk);
            println!("  {} {}", "Oracle:".bright_cyan(), oracle_id_sdk);
            println!("  {} {}bps ({}%)", "IMR:".bright_cyan(), slab.imr, slab.imr as f64 / 100.0);
            println!("  {} {}bps ({}%)", "MMR:".bright_cyan(), slab.mmr, slab.mmr as f64 / 100.0);
            println!("  {} {}bps", "Maker Fee Cap:".bright_cyan(), slab.maker_fee_cap);
            println!("  {} {}bps", "Taker Fee Cap:".bright_cyan(), slab.taker_fee_cap);
            println!("  {} {}ms", "Latency SLA:".bright_cyan(), slab.latency_sla_ms);
            println!("  {} {}", "Max Exposure:".bright_cyan(), slab.max_exposure);
            println!("  {} {}", "Registered:".bright_cyan(), slab.registered_ts);
            println!("  {} {}", "Active:".bright_cyan(), if slab.active { "✓" } else { "✗" });
        }
    }

    println!("\n{} {}\n", "Status:".bright_green(), "OK ✓".bright_green());
    Ok(())
}

pub async fn show_matcher_info(config: &NetworkConfig, slab_id: String) -> Result<()> {
    println!("{}", "=== Slab Info ===".bright_green().bold());
    println!("{} {}", "Network:".bright_cyan(), config.network);
    println!("{} {}", "Slab ID:".bright_cyan(), slab_id);

    // Parse slab address
    let slab_pubkey = Pubkey::from_str(&slab_id)
        .context("Invalid slab address")?;

    // Get RPC client
    let rpc_client = client::create_rpc_client(config);

    // Check if account exists
    match rpc_client.get_account(&slab_pubkey) {
        Ok(account) => {
            println!("\n{}", "=== Account Info ===".bright_yellow());
            println!("{} {}", "Owner:".bright_cyan(), account.owner);
            println!("{} {} bytes", "Data Size:".bright_cyan(), account.data.len());
            println!("{} {} lamports", "Balance:".bright_cyan(), account.lamports);
            println!("{} {}", "Executable:".bright_cyan(), account.executable);

            // Note: Full slab account deserialization would require slab program types
            println!("\n{}", "Note: Full slab details require slab program deployed".dimmed());
        }
        Err(_) => {
            println!("\n{} Slab account not found - this may be a test address", "Warning:".yellow());
        }
    }

    Ok(())
}
