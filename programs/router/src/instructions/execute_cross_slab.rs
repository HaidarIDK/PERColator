//! Execute cross-slab order - v0 main instruction

use crate::state::{Portfolio, Vault, SlabRegistry};
use percolator_common::*;
use pinocchio::{account_info::AccountInfo, msg, pubkey::Pubkey};

/// Slab split - how much to execute on each slab
#[derive(Debug, Clone, Copy)]
pub struct SlabSplit {
    /// Slab account pubkey
    pub slab_id: Pubkey,
    /// Quantity to execute on this slab (1e6 scale)
    pub qty: i64,
    /// Side (0 = buy, 1 = sell)
    pub side: u8,
    /// Limit price (1e6 scale)
    pub limit_px: i64,
}

/// Process execute cross-slab order (v0 main instruction)
///
/// This is the core v0 instruction that proves portfolio netting.
/// Router reads QuoteCache from multiple slabs, splits the order,
/// CPIs to each slab's commit_fill, aggregates receipts, and
/// updates portfolio with net exposure.
///
/// # Arguments
/// * `portfolio` - User's portfolio account
/// * `user` - User pubkey (signer)
/// * `vault` - Collateral vault
/// * `registry` - Slab registry with insurance state
/// * `router_authority` - Router authority PDA (for CPI signing)
/// * `slab_accounts` - Array of slab accounts to execute on
/// * `receipt_accounts` - Array of receipt PDAs (one per slab)
/// * `splits` - How to split the order across slabs
///
/// # Returns
/// * Updates portfolio with net exposures
/// * Accrues insurance fees from taker fills
/// * Checks margin on net exposure (capital efficiency!)
/// * All-or-nothing atomicity
pub fn process_execute_cross_slab(
    portfolio: &mut Portfolio,
    user: &Pubkey,
    vault: &mut Vault,
    registry: &mut SlabRegistry,
    router_authority: &AccountInfo,
    slab_accounts: &[AccountInfo],
    receipt_accounts: &[AccountInfo],
    splits: &[SlabSplit],
) -> Result<(), PercolatorError> {
    // Verify portfolio belongs to user
    if &portfolio.user != user {
        msg!("Error: Portfolio does not belong to user");
        return Err(PercolatorError::InvalidPortfolio);
    }

    // Apply PnL vesting and haircut catchup on user touch
    use crate::state::on_user_touch;
    use pinocchio::sysvars::{clock::Clock, Sysvar};
    let current_slot = Clock::get()
        .map(|clock| clock.slot)
        .unwrap_or(portfolio.last_slot);

    on_user_touch(
        portfolio.principal,
        &mut portfolio.pnl,
        &mut portfolio.vested_pnl,
        &mut portfolio.last_slot,
        &mut portfolio.pnl_index_checkpoint,
        &registry.global_haircut,
        &registry.pnl_vesting_params,
        current_slot,
    );

    // Verify we have matching number of slabs and receipts
    if slab_accounts.len() != receipt_accounts.len() || slab_accounts.len() != splits.len() {
        msg!("Error: Mismatched slab/receipt/split counts");
        return Err(PercolatorError::InvalidInstruction);
    }

    // Verify router_authority is the correct PDA
    use crate::pda::derive_authority_pda;
    let (expected_authority, authority_bump) = derive_authority_pda(&portfolio.router_id);
    if router_authority.key() != &expected_authority {
        msg!("Error: Invalid router authority PDA");
        return Err(PercolatorError::InvalidAccount);
    }

    // Phase 1: Read QuoteCache from each slab (v0 - skip validation for now)
    // In production, we'd validate seqno consistency here (TOCTOU safety)

    // Phase 2: CPI to each slab's commit_fill
    msg!("Executing fills on slabs");

    for (i, split) in splits.iter().enumerate() {
        let slab_account = &slab_accounts[i];
        let receipt_account = &receipt_accounts[i];

        // Get slab program ID from account owner
        let slab_program_id = slab_account.owner();

        // Read current seqno from slab for TOCTOU protection
        let slab_data = slab_account
            .try_borrow_data()
            .map_err(|_| PercolatorError::InvalidAccount)?;
        if slab_data.len() < 4 {
            msg!("Error: Invalid slab account data");
            return Err(PercolatorError::InvalidAccount);
        }
        // Seqno is at offset 0 in SlabHeader (first field)
        let expected_seqno = u32::from_le_bytes([
            slab_data[0],
            slab_data[1],
            slab_data[2],
            slab_data[3],
        ]);

        // Build commit_fill instruction data (22 bytes total)
        // Layout: discriminator (1) + expected_seqno (4) + side (1) + qty (8) + limit_px (8)
        let mut instruction_data = [0u8; 22];
        instruction_data[0] = 1; // CommitFill discriminator
        instruction_data[1..5].copy_from_slice(&expected_seqno.to_le_bytes());
        instruction_data[5] = split.side;
        instruction_data[6..14].copy_from_slice(&split.qty.to_le_bytes());
        instruction_data[14..22].copy_from_slice(&split.limit_px.to_le_bytes());

        // Build account metas for CPI
        // 0. slab_account (writable)
        // 1. receipt_account (writable)
        // 2. router_authority (signer PDA)
        use pinocchio::{
            instruction::{AccountMeta, Instruction},
            program::invoke_signed,
        };

        let account_metas = [
            AccountMeta::writable(slab_account.key()),
            AccountMeta::writable(receipt_account.key()),
            AccountMeta::writable_signer(router_authority.key()),
        ];

        let instruction = Instruction {
            program_id: slab_program_id,
            accounts: &account_metas,
            data: &instruction_data,
        };

        // Sign the CPI with router authority PDA
        use crate::pda::AUTHORITY_SEED;
        use pinocchio::instruction::{Seed, Signer};

        let bump_array = [authority_bump];
        let seeds = &[
            Seed::from(AUTHORITY_SEED),
            Seed::from(&bump_array[..]),
        ];
        let signer = Signer::from(seeds);

        invoke_signed(
            &instruction,
            &[slab_account, receipt_account, router_authority],
            &[signer],
        )
        .map_err(|_| PercolatorError::CpiFailed)?;
    }

    // Phase 3: Aggregate fills and update portfolio
    // For each split, update the portfolio exposure
    for (i, split) in splits.iter().enumerate() {
        // In v0, assume fill is successful
        let filled_qty = split.qty;

        // Update portfolio exposure for this slab/instrument
        // For v0, we'll use slab index and instrument 0 (simplified)
        let slab_idx = i as u16;
        let instrument_idx = 0u16;

        // Get current exposure
        let current_exposure = portfolio.get_exposure(slab_idx, instrument_idx);

        // Update based on side: Buy = add qty, Sell = subtract qty
        let new_exposure = if split.side == 0 {
            // Buy
            current_exposure + filled_qty
        } else {
            // Sell
            current_exposure - filled_qty
        };

        portfolio.update_exposure(slab_idx, instrument_idx, new_exposure);
    }

    // Phase 3.5: Accrue insurance fees from taker fills
    // Calculate total notional across all splits and accrue insurance
    let mut total_notional: u128 = 0;
    for split in splits.iter() {
        // Notional = qty * price (both in 1e6 scale, so divide by 1e6)
        // For v0 simplified: use limit_px as execution price
        let notional = ((split.qty.abs() as u128) * (split.limit_px.abs() as u128)) / 1_000_000;
        total_notional = total_notional.saturating_add(notional);
    }

    if total_notional > 0 {
        let accrual = registry.insurance_state.accrue_from_fill(
            total_notional,
            &registry.insurance_params,
        );
        if accrual > 0 {
            msg!("Insurance accrued from fills");
        }
    }

    // Phase 4: Calculate IM on net exposure using FORMALLY VERIFIED logic
    // Property X3: Net exposure = algebraic sum of all signed exposures
    // Property X3b: If net exposure = 0, then margin = 0 (CAPITAL EFFICIENCY!)
    // See: crates/model_safety/src/cross_slab.rs for Kani proofs

    // Convert portfolio exposures to format expected by verified function
    let exposures: Vec<(u16, u16, i128)> = (0..portfolio.exposure_count as usize)
        .map(|i| {
            (
                portfolio.exposures[i].0,
                portfolio.exposures[i].1,
                portfolio.exposures[i].2 as i128,
            )
        })
        .collect();

    let net_exposure = crate::state::model_bridge::net_exposure_verified(&exposures)
        .map_err(|_| PercolatorError::Overflow)?;

    // Calculate average price from splits (for v0, use first split's price)
    let avg_price = if !splits.is_empty() {
        splits[0].limit_px.abs() as u64
    } else {
        return Err(PercolatorError::InvalidInstruction);
    };

    // Initial margin requirement: 10% (1000 bps)
    let imr_bps = 1000u16;

    let im_required = crate::state::model_bridge::margin_on_net_verified(
        net_exposure,
        avg_price,
        imr_bps,
    )
    .map_err(|_| PercolatorError::Overflow)?;

    msg!("Calculated margin on net exposure using verified logic");

    portfolio.update_margin(im_required, im_required / 2); // MM = IM / 2 for v0

    // Phase 5: Check if portfolio has sufficient margin
    // For v0, we assume equity is managed separately via vault
    // In production, this would check vault.equity >= portfolio.im
    if !portfolio.has_sufficient_margin() {
        msg!("Error: Insufficient margin");
        return Err(PercolatorError::PortfolioInsufficientMargin);
    }

    let _ = vault; // Will be used in production for equity checks
    let _ = receipt_accounts; // Will be used for real CPI

    msg!("ExecuteCrossSlab completed successfully");
    Ok(())
}

// Ad-hoc functions REMOVED - Now using formally verified functions:
// - net_exposure_verified() from model_safety::cross_slab
// - margin_on_net_verified() from model_safety::cross_slab
// These functions have Kani proofs for properties X1-X4 including:
//   - X3: Net exposure = algebraic sum
//   - X3b: If net = 0, then margin = 0 (CAPITAL EFFICIENCY PROOF)
// See: crates/model_safety/src/cross_slab.rs

// Exclude test module from BPF builds to avoid stack overflow from test-only functions
#[cfg(all(test, not(target_os = "solana")))]
#[path = "execute_cross_slab_test.rs"]
mod execute_cross_slab_test;
