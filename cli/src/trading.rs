//! Trading and order management operations

use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    transaction::Transaction,
};
use std::str::FromStr;

use crate::{client, config::NetworkConfig};

/// Derive portfolio PDA for a user
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

/// Derive receipt PDA for a slab
fn derive_receipt_pda(portfolio: &Pubkey, slab_id: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"receipt", portfolio.as_ref(), slab_id.as_ref()],
        program_id,
    )
}

/// Place a limit order on a specific slab
pub async fn place_limit_order(
    config: &NetworkConfig,
    slab: String,
    side: String,
    price: f64,
    size: u64,
    _post_only: bool,
) -> Result<()> {
    println!("{}", "=== Place Limit Order ===".bright_green().bold());
    println!("{} {}", "Slab:".bright_cyan(), slab);
    println!("{} {}", "Side:".bright_cyan(), side.to_uppercase());
    println!("{} {}", "Price:".bright_cyan(), price);
    println!("{} {}", "Size:".bright_cyan(), size);

    // Parse side
    let side_byte: u8 = match side.to_lowercase().as_str() {
        "buy" | "b" => 0,
        "sell" | "s" => 1,
        _ => return Err(anyhow!("Invalid side: must be 'buy' or 'sell'")),
    };

    // Parse slab pubkey
    let slab_pubkey = Pubkey::from_str(&slab).context("Invalid slab pubkey")?;

    // Convert price and size to fixed-point (1e6 scale)
    let price_fixed = (price * 1_000_000.0) as i64;
    let qty_fixed = size as i64;

    println!("\n{}", "Building transaction...".dimmed());

    // Derive PDAs
    let user_pubkey = config.pubkey();
    let (portfolio_pda, _) = derive_portfolio_pda(&user_pubkey, &config.router_program_id);
    let (vault_pda, _) = derive_vault_pda(&config.router_program_id);
    let (registry_pda, _) = derive_registry_pda(&config.router_program_id);
    let (router_authority_pda, _) = derive_router_authority_pda(&config.router_program_id);
    let (receipt_pda, _) = derive_receipt_pda(&portfolio_pda, &slab_pubkey, &config.router_program_id);

    println!("{} {}", "Portfolio PDA:".bright_cyan(), portfolio_pda);
    println!("{} {}", "Slab:".bright_cyan(), slab_pubkey);
    println!("{} {}", "Receipt PDA:".bright_cyan(), receipt_pda);

    // Build instruction data for ExecuteCrossSlab
    // Layout: discriminator (1) + num_splits (1) + [side (1) + qty (8) + limit_px (8)] per split
    let num_splits: u8 = 1;

    let mut instruction_data = Vec::with_capacity(1 + 1 + 17);
    instruction_data.push(4u8); // RouterInstruction::ExecuteCrossSlab discriminator
    instruction_data.push(num_splits); // Number of splits

    // Split data: side (1 byte) + qty (8 bytes) + limit_px (8 bytes)
    instruction_data.push(side_byte);
    instruction_data.extend_from_slice(&qty_fixed.to_le_bytes());
    instruction_data.extend_from_slice(&price_fixed.to_le_bytes());

    // Build account list
    // 0. [writable] Portfolio account
    // 1. [signer] User account
    // 2. [writable] Vault account
    // 3. [writable] Registry account
    // 4. [] Router authority PDA
    // 5. [] Slab account
    // 6. [writable] Receipt PDA
    let accounts = vec![
        AccountMeta::new(portfolio_pda, false),
        AccountMeta::new_readonly(user_pubkey, true),
        AccountMeta::new(vault_pda, false),
        AccountMeta::new(registry_pda, false),
        AccountMeta::new_readonly(router_authority_pda, false),
        AccountMeta::new_readonly(slab_pubkey, false),
        AccountMeta::new(receipt_pda, false),
    ];

    let execute_cross_slab_ix = Instruction {
        program_id: config.router_program_id,
        accounts,
        data: instruction_data,
    };

    // Build and send transaction
    let rpc_client = client::create_rpc_client(config);
    println!("{}", "Sending transaction...".dimmed());
    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let transaction = Transaction::new_signed_with_payer(
        &[execute_cross_slab_ix],
        Some(&user_pubkey),
        &[&config.keypair],
        recent_blockhash,
    );

    match rpc_client.send_and_confirm_transaction(&transaction) {
        Ok(signature) => {
            println!("\n{} Order placed successfully!", "✓".green().bold());
            println!("{} {}", "Transaction:".bright_cyan(), signature);
            println!("\n{} {:.6} @ {:.2}",
                if side_byte == 0 { "BUY".green() } else { "SELL".red() },
                qty_fixed as f64 / 1_000_000.0,
                price_fixed as f64 / 1_000_000.0
            );
        }
        Err(e) => {
            println!("\n{} Order failed: {}", "✗".red().bold(), e);
            println!("\n{}", "Common causes:".bright_yellow());
            println!("  {} Portfolio not initialized", "•".dimmed());
            println!("  {} Slab not registered in registry", "•".dimmed());
            println!("  {} Insufficient margin for position", "•".dimmed());
            println!("  {} Receipt PDA not initialized", "•".dimmed());
            return Err(anyhow!("Order transaction failed: {}", e));
        }
    }

    Ok(())
}

/// Place a market order (limit order with aggressive price)
pub async fn place_market_order(
    config: &NetworkConfig,
    slab: String,
    side: String,
    size: u64,
) -> Result<()> {
    println!("{}", "=== Place Market Order ===".bright_green().bold());
    println!("{} {}", "Slab:".bright_cyan(), slab);
    println!("{} {}", "Side:".bright_cyan(), side.to_uppercase());
    println!("{} {}", "Size:".bright_cyan(), size);

    // For market orders, use extremely aggressive limit price
    // Buy: very high price (e.g., $1B)
    // Sell: very low price (e.g., $0.01)
    let aggressive_price = match side.to_lowercase().as_str() {
        "buy" | "b" => 1_000_000_000.0, // $1B
        "sell" | "s" => 0.01,            // $0.01
        _ => return Err(anyhow!("Invalid side: must be 'buy' or 'sell'")),
    };

    println!("\n{}", "Converting to aggressive limit order...".dimmed());
    println!("{} {}", "Limit Price:".bright_cyan(), aggressive_price);

    // Delegate to place_limit_order with aggressive price
    place_limit_order(config, slab, side, aggressive_price, size, false).await
}

/// Cancel an order by receipt PDA
pub async fn cancel_order(config: &NetworkConfig, receipt_id: String) -> Result<()> {
    println!("{}", "=== Cancel Order ===".bright_green().bold());
    println!("{} {}", "Receipt ID:".bright_cyan(), receipt_id);

    println!("\n{}", "Order Cancellation:".bright_yellow().bold());
    println!("  {} v0 uses fill-or-kill execution model", "ℹ".bright_cyan());
    println!("  {} Orders are executed immediately, not resting on books", "ℹ".bright_cyan());
    println!("  {} No cancellation needed for cross-slab execution", "ℹ".bright_cyan());

    println!("\n{}", "For future resting orders:".dimmed());
    println!("  {} Would use CancelLpOrders instruction (discriminator 7)", "•".dimmed());
    println!("  {} Would require receipt PDA to identify order", "•".dimmed());

    Ok(())
}

/// List open orders for a user
pub async fn list_orders(config: &NetworkConfig, user: Option<String>) -> Result<()> {
    println!("{}", "=== Open Orders ===".bright_green().bold());

    let target_user = if let Some(u) = user {
        let pubkey = Pubkey::from_str(&u).context("Invalid user pubkey")?;
        println!("{} {}", "User:".bright_cyan(), pubkey);
        pubkey
    } else {
        let pubkey = config.pubkey();
        println!("{} {} {}", "User:".bright_cyan(), pubkey, "(self)".dimmed());
        pubkey
    };

    // Derive portfolio PDA
    let (portfolio_pda, _) = derive_portfolio_pda(&target_user, &config.router_program_id);
    println!("{} {}", "Portfolio PDA:".bright_cyan(), portfolio_pda);

    let rpc_client = client::create_rpc_client(config);

    // Fetch portfolio account
    println!("\n{}", "Fetching portfolio...".dimmed());
    let account = rpc_client
        .get_account(&portfolio_pda)
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

    println!("\n{}", "Portfolio Exposures:".bright_yellow().bold());

    if portfolio.exposure_count == 0 {
        println!("  {}", "No open positions".dimmed());
        return Ok(());
    }

    // Display exposures
    // Exposures are stored as tuples: (slab_idx: u16, instrument_idx: u16, qty: i64)
    for i in 0..portfolio.exposure_count as usize {
        let (slab_idx, instrument_idx, qty) = portfolio.exposures[i];
        if qty != 0 {
            let size_display = if qty > 0 {
                format!("{} {}", "LONG".green(), (qty as f64 / 1_000_000.0))
            } else {
                format!("{} {}", "SHORT".red(), (qty.abs() as f64 / 1_000_000.0))
            };

            println!(
                "  {} Slab {} / Instrument {}: {}",
                "•".bright_cyan(),
                slab_idx,
                instrument_idx,
                size_display
            );
        }
    }

    println!("\n{}", "Order Model:".bright_yellow().bold());
    println!("  {} v0 uses immediate cross-slab execution", "ℹ".bright_cyan());
    println!("  {} Orders don't rest on books - they fill or fail", "ℹ".bright_cyan());
    println!("  {} Positions shown above are net exposures after fills", "ℹ".bright_cyan());

    Ok(())
}

/// Show order book for a slab (QuoteCache)
pub async fn show_order_book(config: &NetworkConfig, slab: String, depth: usize) -> Result<()> {
    println!("{}", "=== Order Book ===".bright_green().bold());
    println!("{} {}", "Slab:".bright_cyan(), slab);
    println!("{} {}", "Depth:".bright_cyan(), depth);

    let slab_pubkey = Pubkey::from_str(&slab).context("Invalid slab pubkey")?;

    let rpc_client = client::create_rpc_client(config);

    // Fetch slab account
    println!("\n{}", "Fetching slab state...".dimmed());
    let account = rpc_client
        .get_account(&slab_pubkey)
        .context("Failed to fetch slab account - does it exist?")?;

    // Check if slab account has expected size
    let expected_size = 4096; // SlabState::LEN from slab program
    if account.data.len() != expected_size {
        println!("\n{}", "Warning: Unexpected slab account size".yellow());
        println!("  {} Expected: {}", "•".dimmed(), expected_size);
        println!("  {} Got: {}", "•".dimmed(), account.data.len());
    }

    // Parse QuoteCache (starts at offset 256 after Header)
    let quote_cache_offset = 256;
    if account.data.len() < quote_cache_offset + 256 {
        return Err(anyhow!("Slab account too small to contain QuoteCache"));
    }

    let quote_cache_data = &account.data[quote_cache_offset..quote_cache_offset + 256];

    // QuoteCache structure (from slab/src/state/slab.rs):
    // - seqno: u32 (4 bytes)
    // - best_bid_px: i64 (8 bytes)
    // - best_bid_qty: i64 (8 bytes)
    // - best_ask_px: i64 (8 bytes)
    // - best_ask_qty: i64 (8 bytes)
    // - ... (rest is padding/future use)

    let seqno = u32::from_le_bytes([
        quote_cache_data[0],
        quote_cache_data[1],
        quote_cache_data[2],
        quote_cache_data[3],
    ]);

    let best_bid_px = i64::from_le_bytes([
        quote_cache_data[4],
        quote_cache_data[5],
        quote_cache_data[6],
        quote_cache_data[7],
        quote_cache_data[8],
        quote_cache_data[9],
        quote_cache_data[10],
        quote_cache_data[11],
    ]);

    let best_bid_qty = i64::from_le_bytes([
        quote_cache_data[12],
        quote_cache_data[13],
        quote_cache_data[14],
        quote_cache_data[15],
        quote_cache_data[16],
        quote_cache_data[17],
        quote_cache_data[18],
        quote_cache_data[19],
    ]);

    let best_ask_px = i64::from_le_bytes([
        quote_cache_data[20],
        quote_cache_data[21],
        quote_cache_data[22],
        quote_cache_data[23],
        quote_cache_data[24],
        quote_cache_data[25],
        quote_cache_data[26],
        quote_cache_data[27],
    ]);

    let best_ask_qty = i64::from_le_bytes([
        quote_cache_data[28],
        quote_cache_data[29],
        quote_cache_data[30],
        quote_cache_data[31],
        quote_cache_data[32],
        quote_cache_data[33],
        quote_cache_data[34],
        quote_cache_data[35],
    ]);

    println!("\n{}", "QuoteCache (Router-Readable State):".bright_yellow().bold());
    println!("  {} {}", "Sequence Number:".bright_cyan(), seqno);

    if best_bid_qty > 0 || best_ask_qty > 0 {
        println!("\n  {:<12} {:<15} {:<15}", "Side", "Price", "Quantity");
        println!("  {}", "─".repeat(42).dimmed());

        if best_ask_qty > 0 {
            println!(
                "  {:<12} {:<15.2} {:<15.6}",
                "ASK".red(),
                best_ask_px as f64 / 1_000_000.0,
                best_ask_qty as f64 / 1_000_000.0
            );
        }

        if best_bid_qty > 0 {
            println!(
                "  {:<12} {:<15.2} {:<15.6}",
                "BID".green(),
                best_bid_px as f64 / 1_000_000.0,
                best_bid_qty as f64 / 1_000_000.0
            );
        }

        if best_bid_qty > 0 && best_ask_qty > 0 {
            let spread = best_ask_px - best_bid_px;
            let spread_bps = (spread as f64 / best_bid_px as f64) * 10_000.0;
            println!("\n  {} {:.2} ({:.2} bps)",
                "Spread:".bright_cyan(),
                spread as f64 / 1_000_000.0,
                spread_bps
            );
        }
    } else {
        println!("  {}", "No liquidity available".dimmed());
    }

    println!("\n{}", "Note:".bright_yellow());
    println!("  {} v0 QuoteCache shows top-of-book only", "•".dimmed());
    println!("  {} Full book depth requires BookArea parsing", "•".dimmed());
    println!("  {} Router reads this data for order splitting", "•".dimmed());

    Ok(())
}
