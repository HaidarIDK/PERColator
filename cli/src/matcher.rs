//! Matcher/slab management operations

use anyhow::{Context, Result};
use colored::Colorize;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
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
    config: &NetworkConfig,
    exchange: String,
    symbol: String,
    tick_size: u64,
    lot_size: u64,
) -> Result<()> {
    println!("{}", "=== Create Matcher (Slab) ===".bright_green().bold());
    println!("{} {}", "Network:".bright_cyan(), config.network);
    println!("{} {}", "Exchange:".bright_cyan(), exchange);
    println!("{} {}", "Symbol:".bright_cyan(), symbol);
    println!("{} {}", "Tick Size:".bright_cyan(), tick_size);
    println!("{} {}", "Lot Size:".bright_cyan(), lot_size);

    // Get RPC client and payer
    let rpc_client = client::create_rpc_client(config);
    let payer = &config.keypair;

    println!("\n{} {}", "Payer:".bright_cyan(), payer.pubkey());
    println!("{} {}", "Slab Program:".bright_cyan(), config.slab_program_id);

    // Generate new keypair for the slab account
    let slab_keypair = Keypair::new();
    let slab_pubkey = slab_keypair.pubkey();

    println!("{} {}", "Slab Address:".bright_cyan(), slab_pubkey);

    // Calculate rent for ~4KB account
    const SLAB_SIZE: usize = 4096;
    let rent = rpc_client
        .get_minimum_balance_for_rent_exemption(SLAB_SIZE)
        .context("Failed to get rent exemption amount")?;

    println!("{} {} lamports", "Rent Required:".bright_cyan(), rent);

    // Build CreateAccount instruction to allocate the slab account
    let create_account_ix = solana_sdk::system_instruction::create_account(
        &payer.pubkey(),
        &slab_pubkey,
        rent,
        SLAB_SIZE as u64,
        &config.slab_program_id,
    );

    // Build initialization instruction data
    // Format: [discriminator(1), lp_owner(32), router_id(32), instrument(32),
    //          mark_px(8), taker_fee_bps(8), contract_size(8), bump(1)]
    let mut instruction_data = Vec::with_capacity(122);
    instruction_data.push(0u8); // Initialize discriminator

    // lp_owner: Use payer as the LP owner
    instruction_data.extend_from_slice(&payer.pubkey().to_bytes());

    // router_id: Use router program ID
    instruction_data.extend_from_slice(&config.router_program_id.to_bytes());

    // instrument: Use a dummy instrument ID (system program for now)
    let instrument = solana_sdk::system_program::id();
    instruction_data.extend_from_slice(&instrument.to_bytes());

    // mark_px: Use tick_size * 100 as initial mark price (e.g., $1.00 if tick_size=1)
    let mark_px = (tick_size as i64) * 100;
    instruction_data.extend_from_slice(&mark_px.to_le_bytes());

    // taker_fee_bps: Default to 20 bps (0.2%)
    let taker_fee_bps = 20i64;
    instruction_data.extend_from_slice(&taker_fee_bps.to_le_bytes());

    // contract_size: Use lot_size as contract size
    let contract_size = lot_size as i64;
    instruction_data.extend_from_slice(&contract_size.to_le_bytes());

    // bump: Not using PDA, so 0
    instruction_data.push(0u8);

    // Build Initialize instruction
    let initialize_ix = Instruction {
        program_id: config.slab_program_id,
        accounts: vec![
            AccountMeta::new(slab_pubkey, false),      // Slab account (writable)
            AccountMeta::new(payer.pubkey(), true),    // Payer (signer, writable for fees)
        ],
        data: instruction_data,
    };

    // Send transaction with both instructions
    println!("\n{}", "Creating slab account and initializing...".bright_green());

    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let transaction = Transaction::new_signed_with_payer(
        &[create_account_ix, initialize_ix],
        Some(&payer.pubkey()),
        &[payer, &slab_keypair], // Both payer and slab must sign
        recent_blockhash,
    );

    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .context("Failed to create and initialize slab")?;

    println!("\n{} {}", "Success!".bright_green().bold(), "✓".bright_green());
    println!("{} {}", "Transaction:".bright_cyan(), signature);
    println!("{} {}", "Slab Address:".bright_cyan(), slab_pubkey);
    println!("\n{}", "Next step: Register this slab with the router using:".dimmed());
    println!("  {}", format!("percolator matcher register-slab --slab-id {}", slab_pubkey).dimmed());

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

/// Update funding rate for a slab
///
/// Calls the UpdateFunding instruction (discriminator = 5) on the slab program.
/// This updates the cumulative funding index based on mark-oracle price deviation.
///
/// # Arguments
/// * `config` - Network configuration
/// * `slab_address` - Slab pubkey as string
/// * `oracle_price` - Oracle price (scaled by 1e6, e.g., 100_000_000 for price 100)
/// * `wait_time` - Optional time to wait before calling (simulates time passage)
///
/// # Returns
/// * Ok(()) on success
pub async fn update_funding(
    config: &NetworkConfig,
    slab_address: String,
    oracle_price: i64,
    wait_time: Option<u64>,
) -> Result<()> {
    println!("{}", "=== Update Funding ===".bright_green().bold());
    println!("{} {}", "Network:".bright_cyan(), config.network);
    println!("{} {}", "Slab:".bright_cyan(), slab_address);
    println!("{} {} ({})", "Oracle Price:".bright_cyan(), oracle_price, oracle_price as f64 / 1_000_000.0);

    // Wait if requested (simulates time passage for funding accrual)
    if let Some(seconds) = wait_time {
        println!("\n{} Waiting {} seconds to simulate funding accrual...", "⏱".bright_yellow(), seconds);
        std::thread::sleep(std::time::Duration::from_secs(seconds));
    }

    // Parse slab address
    let slab_pubkey = Pubkey::from_str(&slab_address)
        .context("Invalid slab address")?;

    // Get RPC client
    let rpc_client = client::create_rpc_client(config);
    let authority = &config.keypair;

    // Use slab program ID from config
    let slab_program_id = config.slab_program_id;

    // Build instruction data:
    // - Byte 0: discriminator = 5 (UpdateFunding)
    // - Bytes 1-8: oracle_price (i64 little-endian)
    let mut instruction_data = Vec::with_capacity(9);
    instruction_data.push(5); // UpdateFunding discriminator
    instruction_data.extend_from_slice(&oracle_price.to_le_bytes());

    // Build UpdateFunding instruction
    // Accounts:
    // 0. [writable] slab_account
    // 1. [signer] authority (LP owner)
    let instruction = Instruction {
        program_id: slab_program_id,
        accounts: vec![
            AccountMeta::new(slab_pubkey, false),
            AccountMeta::new_readonly(authority.pubkey(), true),
        ],
        data: instruction_data,
    };

    // Create and send transaction
    let recent_blockhash = rpc_client
        .get_latest_blockhash()
        .context("Failed to get recent blockhash")?;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&authority.pubkey()),
        &[authority],
        recent_blockhash,
    );

    println!("\n{}", "Sending transaction...".dimmed());
    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .context("Failed to send UpdateFunding transaction")?;

    println!("\n{} {}", "✓ Funding updated! Signature:".bright_green(), signature);

    Ok(())
}

/// Place a limit order on the order book
///
/// Calls the PlaceOrder instruction (discriminator = 2) on the slab program.
///
/// # Arguments
/// * `config` - Network configuration
/// * `slab_address` - Slab pubkey as string
/// * `side` - "buy" or "sell"
/// * `price` - Order price (scaled by 1e6, e.g., 100_000_000 for price 100)
/// * `qty` - Order quantity (scaled by 1e6, e.g., 1_000_000 for quantity 1.0)
///
/// # Returns
/// * Ok(()) on success, prints order_id from transaction logs
pub async fn place_order(
    config: &NetworkConfig,
    slab_address: String,
    side: String,
    price: i64,
    qty: i64,
    post_only: bool,
    reduce_only: bool,
) -> Result<()> {
    println!("{}", "=== Place Order ===".bright_green().bold());
    println!("{} {}", "Network:".bright_cyan(), config.network);
    println!("{} {}", "Slab:".bright_cyan(), slab_address);
    println!("{} {}", "Side:".bright_cyan(), side);
    println!("{} {} ({})", "Price:".bright_cyan(), price, price as f64 / 1_000_000.0);
    println!("{} {} ({})", "Quantity:".bright_cyan(), qty, qty as f64 / 1_000_000.0);
    if post_only {
        println!("{} {}", "Post-only:".bright_cyan(), "true");
    }
    if reduce_only {
        println!("{} {}", "Reduce-only:".bright_cyan(), "true");
    }

    // Parse side
    let side_u8 = match side.to_lowercase().as_str() {
        "buy" | "bid" => 0u8,
        "sell" | "ask" => 1u8,
        _ => anyhow::bail!("Invalid side '{}'. Use 'buy' or 'sell'", side),
    };

    // Validate inputs
    if price <= 0 {
        anyhow::bail!("Price must be > 0");
    }
    if qty <= 0 {
        anyhow::bail!("Quantity must be > 0");
    }

    // Parse slab address
    let slab_pubkey = Pubkey::from_str(&slab_address)
        .context("Invalid slab address")?;

    // Get RPC client
    let rpc_client = client::create_rpc_client(config);
    let authority = &config.keypair;

    // Use slab program ID from config
    let slab_program_id = config.slab_program_id;

    // Build instruction data:
    // - Byte 0: discriminator = 2 (PlaceOrder)
    // - Byte 1: side (u8)
    // - Bytes 2-9: price (i64 little-endian)
    // - Bytes 10-17: qty (i64 little-endian)
    // - Byte 18: post_only (u8, optional)
    // - Byte 19: reduce_only (u8, optional)
    let mut instruction_data = Vec::with_capacity(20);
    instruction_data.push(2); // PlaceOrder discriminator
    instruction_data.push(side_u8);
    instruction_data.extend_from_slice(&price.to_le_bytes());
    instruction_data.extend_from_slice(&qty.to_le_bytes());
    instruction_data.push(post_only as u8);
    instruction_data.push(reduce_only as u8);

    // Build PlaceOrder instruction
    // Accounts:
    // 0. [writable] slab_account
    // 1. [signer] authority (trader)
    let instruction = Instruction {
        program_id: slab_program_id,
        accounts: vec![
            AccountMeta::new(slab_pubkey, false),
            AccountMeta::new_readonly(authority.pubkey(), true),
        ],
        data: instruction_data,
    };

    // Create and send transaction
    let recent_blockhash = rpc_client
        .get_latest_blockhash()
        .context("Failed to get recent blockhash")?;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&authority.pubkey()),
        &[authority],
        recent_blockhash,
    );

    println!("\n{}", "Sending transaction...".dimmed());
    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .context("Failed to send PlaceOrder transaction")?;

    println!("\n{} {}", "✓ Order placed! Signature:".bright_green(), signature);
    println!("{}", "Note: order_id can be extracted from transaction logs".dimmed());

    Ok(())
}

/// Cancel an order from the order book
///
/// Calls the CancelOrder instruction (discriminator = 3) on the slab program.
///
/// # Arguments
/// * `config` - Network configuration
/// * `slab_address` - Slab pubkey as string
/// * `order_id` - Order ID to cancel
///
/// # Returns
/// * Ok(()) on success
pub async fn cancel_order(
    config: &NetworkConfig,
    slab_address: String,
    order_id: u64,
) -> Result<()> {
    println!("{}", "=== Cancel Order ===".bright_green().bold());
    println!("{} {}", "Network:".bright_cyan(), config.network);
    println!("{} {}", "Slab:".bright_cyan(), slab_address);
    println!("{} {}", "Order ID:".bright_cyan(), order_id);

    // Parse slab address
    let slab_pubkey = Pubkey::from_str(&slab_address)
        .context("Invalid slab address")?;

    // Get RPC client
    let rpc_client = client::create_rpc_client(config);
    let authority = &config.keypair;

    // Use slab program ID from config
    let slab_program_id = config.slab_program_id;

    // Build instruction data:
    // - Byte 0: discriminator = 3 (CancelOrder)
    // - Bytes 1-8: order_id (u64 little-endian)
    let mut instruction_data = Vec::with_capacity(9);
    instruction_data.push(3); // CancelOrder discriminator
    instruction_data.extend_from_slice(&order_id.to_le_bytes());

    // Build CancelOrder instruction
    // Accounts:
    // 0. [writable] slab_account
    // 1. [signer] authority (order owner)
    let instruction = Instruction {
        program_id: slab_program_id,
        accounts: vec![
            AccountMeta::new(slab_pubkey, false),
            AccountMeta::new_readonly(authority.pubkey(), true),
        ],
        data: instruction_data,
    };

    // Create and send transaction
    let recent_blockhash = rpc_client
        .get_latest_blockhash()
        .context("Failed to get recent blockhash")?;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&authority.pubkey()),
        &[authority],
        recent_blockhash,
    );

    println!("\n{}", "Sending transaction...".dimmed());
    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .context("Failed to send CancelOrder transaction")?;

    println!("\n{} {}", "✓ Order cancelled! Signature:".bright_green(), signature);

    Ok(())
}

/// Get order book state from a slab
///
/// Fetches and displays the current order book state.
///
/// # Arguments
/// * `config` - Network configuration
/// * `slab_address` - Slab pubkey as string
///
/// # Returns
/// * Ok(()) on success
pub async fn get_orderbook(
    config: &NetworkConfig,
    slab_address: String,
) -> Result<()> {
    println!("{}", "=== Order Book ===".bright_green().bold());
    println!("{} {}", "Network:".bright_cyan(), config.network);
    println!("{} {}", "Slab:".bright_cyan(), slab_address);

    // Parse slab address
    let slab_pubkey = Pubkey::from_str(&slab_address)
        .context("Invalid slab address")?;

    // Get RPC client
    let rpc_client = client::create_rpc_client(config);

    // Fetch account data
    let account = rpc_client
        .get_account(&slab_pubkey)
        .context("Failed to fetch slab account")?;

    // Verify ownership
    if account.owner != config.slab_program_id {
        anyhow::bail!("Account is not owned by slab program");
    }

    println!("\n{}", "=== Account Info ===".bright_yellow());
    println!("{} {} bytes", "Data Size:".bright_cyan(), account.data.len());
    println!("{} {} lamports", "Balance:".bright_cyan(), account.lamports);

    // Note: Full deserialization would require importing slab program types
    // For now, we just show the account exists and is owned correctly
    println!("\n{}", "Order book data:".bright_yellow());
    println!("{}", "  (Full deserialization requires slab program types)".dimmed());
    println!("{}", "  Account exists and is owned by slab program".bright_green());

    Ok(())
}
