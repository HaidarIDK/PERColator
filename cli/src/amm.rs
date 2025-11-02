//! AMM creation and management commands

use anyhow::{anyhow, Context, Result};
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

/// Create a new AMM pool
///
/// This creates and initializes a new AMM (Automated Market Maker) pool
/// that can be used for router-based LP operations.
///
/// Parameters:
/// - registry: Registry address (for linking to router)
/// - symbol: Trading pair symbol (e.g., "BTC-USD")
/// - x_reserve: Initial base (X) reserve amount
/// - y_reserve: Initial quote (Y) reserve amount
pub async fn create_amm(
    config: &NetworkConfig,
    registry: String,
    symbol: String,
    x_reserve: u64,
    y_reserve: u64,
) -> Result<()> {
    println!("{}", "=== Create AMM Pool ===".bright_green().bold());
    println!("{} {}", "Network:".bright_cyan(), config.network);
    println!("{} {}", "Registry:".bright_cyan(), registry);
    println!("{} {}", "Symbol:".bright_cyan(), symbol);
    println!("{} {}", "X Reserve (Base):".bright_cyan(), x_reserve);
    println!("{} {}", "Y Reserve (Quote):".bright_cyan(), y_reserve);

    // Parse registry address
    let _registry_pubkey = Pubkey::from_str(&registry)
        .context("Invalid registry address")?;

    // Get RPC client and payer
    let rpc_client = client::create_rpc_client(config);
    let payer = &config.keypair;

    println!("\n{} {}", "Payer:".bright_cyan(), payer.pubkey());
    println!("{} {}", "AMM Program:".bright_cyan(), config.amm_program_id);

    // Generate new keypair for the AMM account
    let amm_keypair = Keypair::new();
    let amm_pubkey = amm_keypair.pubkey();

    println!("{} {}", "AMM Address:".bright_cyan(), amm_pubkey);

    // Calculate rent for AMM account
    // AMM state size: SlabHeader(256) + QuoteCache(136) + AmmPool(64) = 464 bytes
    const AMM_SIZE: usize = 464;
    let rent = rpc_client
        .get_minimum_balance_for_rent_exemption(AMM_SIZE)
        .context("Failed to get rent exemption amount")?;

    println!("{} {} lamports ({} bytes)", "Rent Required:".bright_cyan(), rent, AMM_SIZE);

    // Build CreateAccount instruction to allocate the AMM account
    let create_account_ix = solana_sdk::system_instruction::create_account(
        &payer.pubkey(),
        &amm_pubkey,
        rent,
        AMM_SIZE as u64,
        &config.amm_program_id,
    );

    // Build initialization instruction data
    // Format: [discriminator(1), lp_owner(32), router_id(32), instrument(32),
    //          mark_px(8), taker_fee_bps(8), contract_size(8), bump(1),
    //          x_reserve(8), y_reserve(8)]
    let mut instruction_data = Vec::with_capacity(138);
    instruction_data.push(0u8); // Initialize discriminator

    // lp_owner: Use payer as the LP owner
    instruction_data.extend_from_slice(&payer.pubkey().to_bytes());

    // router_id: Use router program ID
    instruction_data.extend_from_slice(&config.router_program_id.to_bytes());

    // instrument: Use a dummy instrument ID (system program for now)
    // In production, this would be the instrument registry entry
    let instrument = solana_sdk::system_program::id();
    instruction_data.extend_from_slice(&instrument.to_bytes());

    // mark_px: Calculate reasonable initial mark price from reserves
    // mark_px = (y_reserve / x_reserve) * SCALE
    // SCALE = 1_000_000 for Q32 fixed point
    let mark_px = if x_reserve > 0 {
        ((y_reserve as i128 * 1_000_000) / x_reserve as i128) as i64
    } else {
        60_000_000_000i64 // Default to $60k if no reserves
    };
    instruction_data.extend_from_slice(&mark_px.to_le_bytes());

    // taker_fee_bps: Default to 20 bps (0.2%)
    let taker_fee_bps = 20i64;
    instruction_data.extend_from_slice(&taker_fee_bps.to_le_bytes());

    // contract_size: Default to 1.0 (scaled by 1M)
    let contract_size = 1_000_000i64;
    instruction_data.extend_from_slice(&contract_size.to_le_bytes());

    // bump: Not using PDA, so 0
    instruction_data.push(0u8);

    // x_reserve: Base reserve (scaled by 1M for Q32)
    let x_reserve_scaled = (x_reserve as i64) * 1_000_000;
    instruction_data.extend_from_slice(&x_reserve_scaled.to_le_bytes());

    // y_reserve: Quote reserve (scaled by 1M for Q32)
    let y_reserve_scaled = (y_reserve as i64) * 1_000_000;
    instruction_data.extend_from_slice(&y_reserve_scaled.to_le_bytes());

    // Build Initialize instruction
    let initialize_ix = Instruction {
        program_id: config.amm_program_id,
        accounts: vec![
            AccountMeta::new(amm_pubkey, false),       // AMM account (writable)
            AccountMeta::new(payer.pubkey(), true),    // Payer (signer, writable for fees)
        ],
        data: instruction_data,
    };

    // Send transaction with both instructions
    println!("\n{}", "Creating AMM account and initializing...".bright_green());

    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let transaction = Transaction::new_signed_with_payer(
        &[create_account_ix, initialize_ix],
        Some(&payer.pubkey()),
        &[payer, &amm_keypair], // Both payer and AMM must sign
        recent_blockhash,
    );

    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .context("Failed to create and initialize AMM")?;

    println!("\n{} {}", "Success!".bright_green().bold(), "✓".bright_green());
    println!("{} {}", "Transaction:".bright_cyan(), signature);
    println!("{} {}", "AMM Address:".bright_cyan(), amm_pubkey);

    // Calculate and display spot price
    let spot_price = if x_reserve > 0 {
        (y_reserve as f64) / (x_reserve as f64)
    } else {
        0.0
    };
    println!("\n{}", "AMM Parameters:".bright_yellow());
    println!("  {} {} (scaled: {})", "X Reserve:".dimmed(), x_reserve, x_reserve_scaled);
    println!("  {} {} (scaled: {})", "Y Reserve:".dimmed(), y_reserve, y_reserve_scaled);
    println!("  {} {:.2}", "Spot Price:".dimmed(), spot_price);
    println!("  {} {} ({}%)", "Taker Fee:".dimmed(), taker_fee_bps, taker_fee_bps as f64 / 100.0);

    println!("\n{}", "Next step: Add liquidity using:".dimmed());
    println!("  {}", format!("percolator liquidity add {} <AMOUNT>", amm_pubkey).dimmed());

    Ok(())
}

/// List AMM pools (placeholder for future implementation)
pub async fn list_amms(_config: &NetworkConfig) -> Result<()> {
    // TODO: Implement AMM listing functionality
    // This should:
    // 1. Query program accounts for AMM state using getProgramAccounts
    // 2. Deserialize AMM account data to extract pool information
    // 3. Display pool liquidity, fees, and active status
    // 4. Show associated oracle and price information
    //
    // Not implemented - command will fail rather than silently succeed
    anyhow::bail!("AMM listing not yet implemented - requires getProgramAccounts integration")
}
