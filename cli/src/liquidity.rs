//! Liquidity provider operations using the LP Adapter infrastructure
//!
//! This module implements CLI commands for the new LP adapter system that provides
//! a custody-less design where the Router owns tokens and Matcher provides liquidity logic.
//!
//! ## LP Adapter Flow
//! 1. **Reserve**: Lock collateral from portfolio into LP seat
//! 2. **Liquidity**: Execute LP operation via matcher adapter (add/remove/modify)
//! 3. **Release**: Unlock collateral from LP seat back to portfolio
//!
//! ## Instructions
//! - RouterReserve (discriminator 9): Reserve collateral into seat
//! - RouterRelease (discriminator 10): Release collateral from seat
//! - RouterLiquidity (discriminator 11): Execute LP operation via CPI to matcher

use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use std::str::FromStr;

use crate::{client, config::NetworkConfig};

/// Derive LP seat PDA
/// Seeds: ["lp_seat", router_id, matcher_state, portfolio, context_id]
fn derive_lp_seat_pda(
    router_id: &Pubkey,
    matcher_state: &Pubkey,
    portfolio: &Pubkey,
    context_id: u32,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    let context_id_bytes = context_id.to_le_bytes();
    Pubkey::find_program_address(
        &[
            b"lp_seat",
            router_id.as_ref(),
            matcher_state.as_ref(),
            portfolio.as_ref(),
            &context_id_bytes,
        ],
        program_id,
    )
}

/// Derive venue PnL PDA
/// Seeds: ["venue_pnl", router_id, matcher_state]
fn derive_venue_pnl_pda(
    router_id: &Pubkey,
    matcher_state: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"venue_pnl", router_id.as_ref(), matcher_state.as_ref()],
        program_id,
    )
}

/// Derive portfolio PDA
fn derive_portfolio_pda(user: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"portfolio", user.as_ref()], program_id)
}

/// Derive router registry PDA
fn derive_registry_pda(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"registry"], program_id)
}

/// Add liquidity to a matcher venue
///
/// This executes a two-step process:
/// 1. RouterReserve: Lock collateral from portfolio into LP seat
/// 2. RouterLiquidity: Execute LP add operation via matcher adapter
///
/// Supports two modes:
/// - AMM mode (default): Adds continuous liquidity with AmmAdd intent
/// - Orderbook mode: Places discrete orders with ObAdd intent
pub async fn add_liquidity(
    config: &NetworkConfig,
    matcher: String,
    amount: u64,
    price: Option<f64>,
    mode: String,
    side: Option<String>,
    post_only: bool,
    reduce_only: bool,
    lower_price: Option<f64>,
    upper_price: Option<f64>,
) -> Result<()> {
    println!("{}", "=== Add Liquidity ===".bright_green().bold());
    println!("{} {}", "Matcher:".bright_cyan(), matcher);
    println!("{} {}", "Amount:".bright_cyan(), amount);
    println!("{} {}", "Mode:".bright_cyan(), mode);
    if let Some(p) = price {
        println!("{} {}", "Price:".bright_cyan(), p);
    }
    if mode == "orderbook" {
        if let Some(s) = &side {
            println!("{} {}", "Side:".bright_cyan(), s);
        }
        if post_only {
            println!("{} {}", "Post Only:".bright_cyan(), "yes");
        }
        if reduce_only {
            println!("{} {}", "Reduce Only:".bright_cyan(), "yes");
        }
    }

    // Parse matcher state pubkey
    let matcher_state = Pubkey::from_str(&matcher)
        .context("Invalid matcher state pubkey")?;

    // Derive PDAs
    let user_pubkey = config.pubkey();
    let (portfolio_pda, _) = derive_portfolio_pda(&user_pubkey, &config.router_program_id);
    let (registry_pda, _) = derive_registry_pda(&config.router_program_id);

    // For this example, use context_id = 0 (first LP seat for this matcher)
    let context_id: u32 = 0;
    let (lp_seat_pda, _) = derive_lp_seat_pda(
        &registry_pda,
        &matcher_state,
        &portfolio_pda,
        context_id,
        &config.router_program_id,
    );
    let (venue_pnl_pda, _) = derive_venue_pnl_pda(
        &registry_pda,
        &matcher_state,
        &config.router_program_id,
    );

    println!("\n{}", "Derived PDAs:".dimmed());
    println!("{} {}", "Portfolio:".bright_cyan(), portfolio_pda);
    println!("{} {}", "LP Seat:".bright_cyan(), lp_seat_pda);
    println!("{} {}", "Venue PnL:".bright_cyan(), venue_pnl_pda);

    // Convert amount to Q64 fixed-point (for simplicity, treating amount as whole units)
    // In production, this should be properly scaled
    let base_amount_q64: u128 = (amount as u128) << 32; // Simple Q32 for demo
    let quote_amount_q64: u128 = base_amount_q64; // 1:1 for demo

    println!("\n{}", "Building RouterReserve instruction...".dimmed());

    // Step 1: RouterReserve instruction (discriminator 9)
    // Layout: discriminator (1) + base_amount_q64 (16) + quote_amount_q64 (16)
    let mut reserve_data = Vec::with_capacity(33);
    reserve_data.push(9u8); // RouterInstruction::RouterReserve
    reserve_data.extend_from_slice(&base_amount_q64.to_le_bytes());
    reserve_data.extend_from_slice(&quote_amount_q64.to_le_bytes());

    let reserve_accounts = vec![
        AccountMeta::new(portfolio_pda, false),
        AccountMeta::new(lp_seat_pda, false),
    ];

    let reserve_ix = Instruction {
        program_id: config.router_program_id,
        accounts: reserve_accounts,
        data: reserve_data,
    };

    println!("{}", "Building RouterLiquidity instruction...".dimmed());

    // Step 2: RouterLiquidity instruction (discriminator 11)
    // Layout: discriminator (1) + RiskGuard (8) + LiquidityIntent (variable)
    //
    // DEFERRED: Borsh serialization (Phase 4 - Production Deployment)
    //   Current: Simplified serialization for testing router infrastructure
    //   Production requires:
    //     1. Add BorshSerialize/Deserialize derives to adapter_core types
    //     2. Replace simplified encoding below with proper Borsh
    //     3. Test serialization roundtrip for all LiquidityIntent variants
    //   See docs/LP_ADAPTER_CPI_INTEGRATION.md "Next Steps for Production"
    let mut liquidity_data = Vec::with_capacity(64);
    liquidity_data.push(11u8); // RouterInstruction::RouterLiquidity

    // RiskGuard (8 bytes total: 3 × u16 + 2 padding)
    liquidity_data.extend_from_slice(&100u16.to_le_bytes()); // max_slippage_bps = 1%
    liquidity_data.extend_from_slice(&50u16.to_le_bytes());  // max_fee_bps = 0.5%
    liquidity_data.extend_from_slice(&200u16.to_le_bytes()); // oracle_bound_bps = 2%
    liquidity_data.extend_from_slice(&[0u8; 2]); // padding

    // LiquidityIntent: serialize based on mode
    // In production, use proper Borsh serialization
    if mode == "orderbook" {
        // LiquidityIntent::ObAdd (discriminator 2)
        // Format: [intent_disc(1), orders_count(4), order_data..., post_only(1), reduce_only(1)]
        liquidity_data.push(2u8); // ObAdd variant discriminator

        // Validate required parameters
        let price_value = price.ok_or_else(|| anyhow!("--price is required for orderbook mode"))?;
        let side_str = side.as_ref().ok_or_else(|| anyhow!("--side is required for orderbook mode (buy/sell)"))?;

        // Parse side
        let side_byte = match side_str.to_lowercase().as_str() {
            "buy" | "bid" => 0u8,
            "sell" | "ask" => 1u8,
            _ => return Err(anyhow!("Invalid side: {}. Use 'buy' or 'sell'", side_str)),
        };

        // Convert price to Q64 (simplified as Q32 for demo)
        let price_q64: u128 = ((price_value * (1u64 << 32) as f64) as u128);
        let qty_q64: u128 = base_amount_q64; // Use the amount as quantity
        let tif_slots: u32 = 1000; // Default time-in-force: 1000 slots

        // Orders count: 1 order
        liquidity_data.extend_from_slice(&1u32.to_le_bytes());

        // Order data
        liquidity_data.push(side_byte);
        liquidity_data.extend_from_slice(&price_q64.to_le_bytes());
        liquidity_data.extend_from_slice(&qty_q64.to_le_bytes());
        liquidity_data.extend_from_slice(&tif_slots.to_le_bytes());

        // Post-only and reduce-only flags
        liquidity_data.push(post_only as u8);
        liquidity_data.push(reduce_only as u8);

        println!("{}", format!("Order: {} {} @ {} (TIF: {} slots)",
            if side_byte == 0 { "BUY" } else { "SELL" },
            amount,
            price_value,
            tif_slots
        ).dimmed());
    } else {
        // LiquidityIntent::AmmAdd (discriminator 0)
        liquidity_data.push(0u8); // AmmAdd variant discriminator

        // Use provided price bounds or defaults
        let lower_px = lower_price.unwrap_or(0.0);
        let upper_px = upper_price.unwrap_or(f64::MAX);
        let lower_px_q64: u128 = ((lower_px * (1u64 << 32) as f64) as u128);
        let upper_px_q64: u128 = if upper_px == f64::MAX {
            u128::MAX
        } else {
            ((upper_px * (1u64 << 32) as f64) as u128)
        };

        liquidity_data.extend_from_slice(&lower_px_q64.to_le_bytes()); // lower_px_q64
        liquidity_data.extend_from_slice(&upper_px_q64.to_le_bytes()); // upper_px_q64
        liquidity_data.extend_from_slice(&base_amount_q64.to_le_bytes()); // quote_notional_q64
        liquidity_data.extend_from_slice(&0u32.to_le_bytes()); // curve_id
        liquidity_data.extend_from_slice(&30u16.to_le_bytes()); // fee_bps = 0.3%
    }

    let liquidity_accounts = vec![
        AccountMeta::new(portfolio_pda, false),
        AccountMeta::new(lp_seat_pda, false),
        AccountMeta::new(venue_pnl_pda, false),
        AccountMeta::new_readonly(matcher_state, false), // Matcher program
    ];

    let liquidity_ix = Instruction {
        program_id: config.router_program_id,
        accounts: liquidity_accounts,
        data: liquidity_data,
    };

    println!("{}", "Sending transaction...".dimmed());

    // Send both instructions in one transaction
    match client::send_and_confirm_transaction(config, vec![reserve_ix, liquidity_ix]).await {
        Ok(signature) => {
            println!("\n{} Liquidity added successfully!", "✓".green().bold());
            println!("{} {}", "Transaction:".bright_cyan(), signature);
            println!("\n{} Reserved: {} (base) + {} (quote)",
                "📊".bright_yellow(),
                base_amount_q64 >> 32,
                quote_amount_q64 >> 32
            );
            println!("{} LP seat: {}", "🪑".bright_yellow(), lp_seat_pda);
        }
        Err(e) => {
            println!("\n{} Liquidity add failed: {}", "✗".red().bold(), e);
            println!("\n{}", "Common causes:".bright_yellow());
            println!("  {} Portfolio not initialized", "•".dimmed());
            println!("  {} LP seat account not created", "•".dimmed());
            println!("  {} Insufficient free collateral", "•".dimmed());
            println!("  {} Matcher adapter CPI failed", "•".dimmed());
            return Err(anyhow!("Liquidity add transaction failed: {}", e));
        }
    }

    Ok(())
}

/// Remove liquidity from a matcher venue
///
/// This executes a two-step process:
/// 1. RouterLiquidity: Execute LP remove operation via matcher adapter
/// 2. RouterRelease: Unlock collateral from LP seat back to portfolio
pub async fn remove_liquidity(
    config: &NetworkConfig,
    matcher: String,
    amount: u64,
) -> Result<()> {
    println!("{}", "=== Remove Liquidity ===".bright_green().bold());
    println!("{} {}", "Matcher:".bright_cyan(), matcher);
    println!("{} {}", "Amount:".bright_cyan(), amount);

    // Parse matcher state pubkey
    let matcher_state = Pubkey::from_str(&matcher)
        .context("Invalid matcher state pubkey")?;

    // Derive PDAs
    let user_pubkey = config.pubkey();
    let (portfolio_pda, _) = derive_portfolio_pda(&user_pubkey, &config.router_program_id);
    let (registry_pda, _) = derive_registry_pda(&config.router_program_id);

    let context_id: u32 = 0;
    let (lp_seat_pda, _) = derive_lp_seat_pda(
        &registry_pda,
        &matcher_state,
        &portfolio_pda,
        context_id,
        &config.router_program_id,
    );
    let (venue_pnl_pda, _) = derive_venue_pnl_pda(
        &registry_pda,
        &matcher_state,
        &config.router_program_id,
    );

    println!("\n{}", "Derived PDAs:".dimmed());
    println!("{} {}", "Portfolio:".bright_cyan(), portfolio_pda);
    println!("{} {}", "LP Seat:".bright_cyan(), lp_seat_pda);

    let base_amount_q64: u128 = (amount as u128) << 32;
    let quote_amount_q64: u128 = base_amount_q64;

    println!("\n{}", "Building RouterLiquidity instruction...".dimmed());

    // Step 1: RouterLiquidity with Remove intent (discriminator 11)
    let mut liquidity_data = Vec::with_capacity(64);
    liquidity_data.push(11u8);

    // RiskGuard
    liquidity_data.extend_from_slice(&100u16.to_le_bytes());
    liquidity_data.extend_from_slice(&50u16.to_le_bytes());
    liquidity_data.extend_from_slice(&200u16.to_le_bytes());
    liquidity_data.extend_from_slice(&[0u8; 2]);

    // LiquidityIntent::Remove { selector: RemoveSel::ObAll }
    // In production, use proper Borsh serialization
    liquidity_data.push(3u8); // Remove variant discriminator
    liquidity_data.push(2u8); // RemoveSel::ObAll discriminator

    let liquidity_accounts = vec![
        AccountMeta::new(portfolio_pda, false),
        AccountMeta::new(lp_seat_pda, false),
        AccountMeta::new(venue_pnl_pda, false),
        AccountMeta::new_readonly(matcher_state, false),
    ];

    let liquidity_ix = Instruction {
        program_id: config.router_program_id,
        accounts: liquidity_accounts,
        data: liquidity_data,
    };

    println!("{}", "Building RouterRelease instruction...".dimmed());

    // Step 2: RouterRelease instruction (discriminator 10)
    let mut release_data = Vec::with_capacity(33);
    release_data.push(10u8);
    release_data.extend_from_slice(&base_amount_q64.to_le_bytes());
    release_data.extend_from_slice(&quote_amount_q64.to_le_bytes());

    let release_accounts = vec![
        AccountMeta::new(portfolio_pda, false),
        AccountMeta::new(lp_seat_pda, false),
    ];

    let release_ix = Instruction {
        program_id: config.router_program_id,
        accounts: release_accounts,
        data: release_data,
    };

    println!("{}", "Sending transaction...".dimmed());

    match client::send_and_confirm_transaction(config, vec![liquidity_ix, release_ix]).await {
        Ok(signature) => {
            println!("\n{} Liquidity removed successfully!", "✓".green().bold());
            println!("{} {}", "Transaction:".bright_cyan(), signature);
            println!("\n{} Released: {} (base) + {} (quote)",
                "📊".bright_yellow(),
                base_amount_q64 >> 32,
                quote_amount_q64 >> 32
            );
        }
        Err(e) => {
            println!("\n{} Liquidity remove failed: {}", "✗".red().bold(), e);
            println!("\n{}", "Common causes:".bright_yellow());
            println!("  {} LP seat not found", "•".dimmed());
            println!("  {} Insufficient LP shares", "•".dimmed());
            println!("  {} Seat is frozen", "•".dimmed());
            println!("  {} Insufficient reserved collateral for release", "•".dimmed());
            return Err(anyhow!("Liquidity remove transaction failed: {}", e));
        }
    }

    Ok(())
}

/// Show LP positions for a user
pub async fn show_positions(config: &NetworkConfig, user: Option<String>) -> Result<()> {
    println!("{}", "=== Liquidity Positions ===".bright_green().bold());

    let target_user = if let Some(u) = user {
        Pubkey::from_str(&u).context("Invalid user pubkey")?
    } else {
        config.pubkey()
    };

    println!("{} {}", "User:".bright_cyan(), target_user);

    // Derive portfolio PDA
    let (portfolio_pda, _) = derive_portfolio_pda(&target_user, &config.router_program_id);
    println!("{} {}", "Portfolio:".bright_cyan(), portfolio_pda);

    // Query portfolio account
    println!("\n{}", "Querying LP seats...".dimmed());

    match client::get_account_data(config, &portfolio_pda) {
        Ok(data) => {
            if data.len() < 32 {
                println!("{} Portfolio account too small", "⚠".yellow());
                return Ok(());
            }

            // DEFERRED: Full position inspection (Phase 4 - Enhanced CLI Features)
            //   Current: Shows raw account data for verification
            //   Production requires:
            //     1. Deserialize Portfolio to find all associated LP seats
            //     2. Query each LP seat PDA and deserialize RouterLpSeat struct
            //     3. Format and display: lp_shares, exposure, reserved amounts
            //     4. Query VenuePnl for each unique matcher
            println!("\n{}", "⚠ LP position inspection requires proper account deserialization".yellow());
            println!("{}", "  Enhancement roadmap:".dimmed());
            println!("{}", "  1. Deserialize Portfolio account".dimmed());
            println!("{}", "  2. Query all LP seat PDAs for this portfolio".dimmed());
            println!("{}", "  3. Deserialize RouterLpSeat accounts".dimmed());
            println!("{}", "  4. Display lp_shares, exposure, reserved amounts".dimmed());

            println!("\n{} bytes", data.len());
            println!("{}", "Raw account data (truncated):".dimmed());
            let preview = &data[0..std::cmp::min(64, data.len())];
            println!("{:02x?}...", preview);
        }
        Err(e) => {
            println!("\n{} Portfolio not found: {}", "✗".red().bold(), e);
            println!("{}", "  User may not have initialized their portfolio".dimmed());
        }
    }

    Ok(())
}
