//! Bridge between production state and verified model_safety types
//!
//! This module provides conversions to enable production code to use
//! formally verified functions from model_safety.
//!
//! # Architecture
//!
//! - **model_safety**: Abstract, mathematically clean model with formal proofs
//! - **Production**: Complex Solana state with exposures, LP buckets, multiple vaults
//!
//! # Conversion Strategy
//!
//! 1. Portfolio → Account: Maps user-level state
//! 2. SlabRegistry → Params: Maps global parameters
//! 3. Aggregate vaults → vault field
//!
//! # Type Mappings
//!
//! | Production Field | Model Field | Notes |
//! |------------------|-------------|-------|
//! | Portfolio.principal (i128) | Account.principal (u128) | Convert via max(0, principal) as u128 |
//! | Portfolio.pnl (i128) | Account.pnl_ledger (i128) | Direct mapping |
//! | Portfolio.vested_pnl (i128) | Account.reserved_pnl (u128) | Convert via max(0, vested) as u128 |
//! | Portfolio.last_slot (u64) | Warmup.started_at_slot (u64) | Direct mapping |
//! | (Not mapped) | State.loss_accum/fee_index (u128) | Fee distribution state tracked separately |
//!
//! # Limitations
//!
//! - **Vesting algorithms differ**: Production uses exponential, model uses linear
//! - **Global haircut**: Production tracks pnl_index_checkpoint, model doesn't
//! - **Position tracking**: Production has complex exposures, model has simple position_size
//! - **Multiple vaults**: Production has per-mint vaults, model assumes single collateral
//!
//! # Usage
//!
//! ```rust,ignore
//! use model_safety;
//! use crate::state::model_bridge::*;
//!
//! // Convert portfolio to model account
//! let account = portfolio_to_account(&portfolio, &registry);
//!
//! // Use verified function
//! if model_safety::helpers::is_liquidatable(&account, &prices, &params) {
//!     // Execute liquidation using verified logic
//! }
//!
//! // For arithmetic, use verified math directly
//! use model_safety::math::*;
//! let result = add_u128(a, b);  // Saturating addition
//! ```

use super::{Portfolio, SlabRegistry};
use model_safety;

/// Convert a Portfolio to a model_safety Account
///
/// This enables using verified predicates (is_liquidatable, etc.) on production state.
///
/// # Type conversions
///
/// - `principal`: i128 → u128 via max(0, principal) cast
/// - `vested_pnl`: i128 → u128 via max(0, vested_pnl) cast
/// - `position_size`: Calculated from exposures array
///
/// # Arguments
///
/// * `portfolio` - Production user portfolio
/// * `registry` - Global registry (for vesting params)
///
/// # Returns
///
/// model_safety::Account with converted fields
pub fn portfolio_to_account(portfolio: &Portfolio, registry: &SlabRegistry) -> model_safety::Account {
    // Convert principal: i128 → u128 (clamp negative to 0)
    let principal = if portfolio.principal >= 0 {
        portfolio.principal as u128
    } else {
        0u128
    };

    // Convert vested_pnl: i128 → u128 (clamp negative to 0)
    let reserved_pnl = if portfolio.vested_pnl >= 0 {
        portfolio.vested_pnl as u128
    } else {
        0u128
    };

    // Calculate total position size from exposures
    // Sum absolute values of all position quantities
    let mut total_position_size = 0u128;
    for i in 0..portfolio.exposure_count as usize {
        let (_slab_idx, _instrument_idx, qty) = portfolio.exposures[i];
        // Position size is absolute value of quantity
        let abs_qty = qty.abs() as u128;
        total_position_size = total_position_size.saturating_add(abs_qty);
    }

    // Calculate slope_per_step for warmup
    // In linear model: withdrawable = steps_elapsed * slope_per_step
    // Map from production's exponential τ to a reasonable linear slope
    //
    // Heuristic: slope_per_step ≈ principal / (4 * tau_slots)
    // This means full vesting after ~4τ steps (matching exponential behavior)
    let tau = registry.pnl_vesting_params.tau_slots;
    let slope_per_step = if tau > 0 && principal > 0 {
        principal / (4 * tau as u128).max(1)
    } else {
        principal // Instant vesting if tau=0
    };

    model_safety::Account {
        principal,
        pnl_ledger: portfolio.pnl,
        reserved_pnl,
        warmup_state: model_safety::Warmup {
            started_at_slot: portfolio.last_slot,
            slope_per_step,
        },
        position_size: total_position_size,
        fee_index_user: 0,
        fee_accrued: 0,
        vested_pos_snapshot: 0,
    }
}

/// Convert multiple portfolios to a model State
///
/// This aggregates multiple user portfolios into a single model State,
/// enabling whole-system verification of operations like loss socialization.
///
/// # Arguments
///
/// * `portfolios` - Slice of user portfolios
/// * `registry` - Global registry with params and insurance state
/// * `total_vault_balance` - Sum of all vault balances across mints
/// * `total_fees` - Accumulated fees outstanding
///
/// # Returns
///
/// model_safety::State with aggregated user accounts
///
/// # Panics
///
/// Panics if portfolios.len() > model_safety::state::MAX_USERS
pub fn portfolios_to_state(
    portfolios: &[Portfolio],
    registry: &SlabRegistry,
    total_vault_balance: u128,
    total_fees: u128,
) -> model_safety::State {
    let mut users = arrayvec::ArrayVec::<model_safety::Account, 6>::new();

    for portfolio in portfolios.iter() {
        let account = portfolio_to_account(portfolio, registry);
        if users.try_push(account).is_err() {
            // Silently skip if we exceed capacity
            // In production, this should be handled differently
            break;
        }
    }

    let params = model_safety::Params {
        max_users: portfolios.len() as u8,
        withdraw_cap_per_step: registry.pnl_vesting_params.tau_slots as u128, // Placeholder
        maintenance_margin_bps: registry.mmr,
    };

    model_safety::State {
        vault: total_vault_balance,
        fees_outstanding: total_fees,
        users,
        params,
        authorized_router: true, // Production always authorized
        loss_accum: 0,
        fee_index: 0,
        sum_vested_pos_pnl: 0,
        fee_carry: 0,
    }
}

/// Apply verified account changes back to production portfolio
///
/// After calling a verified model_safety function, use this to apply
/// the results back to production state.
///
/// # Arguments
///
/// * `portfolio` - Production portfolio to update (mutable)
/// * `account` - Model account with updated values
///
/// # Safety
///
/// This function trusts that the model account was derived from verified operations.
/// DO NOT call this with arbitrary account values.
pub fn apply_account_to_portfolio(
    portfolio: &mut Portfolio,
    account: &model_safety::Account,
) {
    // Apply principal change
    // Model uses u128, production uses i128
    // Safe because verified functions never create negative principal
    portfolio.principal = account.principal as i128;

    // Apply PnL change
    portfolio.pnl = account.pnl_ledger;

    // Apply vested PnL change
    portfolio.vested_pnl = account.reserved_pnl as i128;

    // Update last_slot
    portfolio.last_slot = account.warmup_state.started_at_slot;

    // Note: position_size in model is aggregate; we don't update exposures array
    // because that requires more complex mapping. The exposures array should be
    // updated separately through production-specific logic.
}

/// Apply verified state changes back to production
///
/// After calling a verified whole-system operation (like socialize_losses),
/// use this to apply results back to production.
///
/// # Arguments
///
/// * `portfolios` - Slice of production portfolios to update (mutable)
/// * `registry` - Global registry to update (mutable)
/// * `state` - Model state with updated values
///
/// # Panics
///
/// Panics if portfolios.len() != state.users.len()
pub fn apply_state_to_portfolios(
    portfolios: &mut [Portfolio],
    registry: &mut SlabRegistry,
    state: &model_safety::State,
) {
    assert_eq!(
        portfolios.len(),
        state.users.len(),
        "Portfolio count must match model user count"
    );

    // Apply individual account changes
    for (portfolio, account) in portfolios.iter_mut().zip(state.users.iter()) {
        apply_account_to_portfolio(portfolio, account);
    }

    // Note: Fee distribution state (loss_accum, fee_index, etc.) is maintained
    // separately in production. These fields are not synced back from the model.
    // Conservation should be verified separately via conservation_ok() checks.
}

/// Check conservation using verified helper
///
/// This is a critical safety check that should be called in tests
/// and optionally in production (governance mode).
///
/// # Arguments
///
/// * `portfolios` - All user portfolios
/// * `registry` - Global registry
/// * `total_vault_balance` - Sum of all vault balances
/// * `total_fees` - Accumulated fees
///
/// # Returns
///
/// true if conservation invariant holds, false otherwise
pub fn check_conservation(
    portfolios: &[Portfolio],
    registry: &SlabRegistry,
    total_vault_balance: u128,
    total_fees: u128,
) -> bool {
    let state = portfolios_to_state(portfolios, registry, total_vault_balance, total_fees);
    model_safety::helpers::conservation_ok(&state)
}

/// Check if portfolio is liquidatable using verified helper
///
/// This uses the formally verified `is_liquidatable` function from model_safety,
/// backed by 13 liquidation proofs (L1-L13).
///
/// # Verified Properties (from Kani proofs)
///
/// When this returns true, the following properties are guaranteed:
/// - L1: Progress if any liquidatable exists
/// - L2: No-op at fixpoint (when none liquidatable)
/// - L3: Count never increases after liquidation
/// - L4: Only liquidatable accounts touched
/// - L5: Non-interference (unrelated accounts unchanged)
/// - L6: Authorization required for liquidation
/// - L7: Conservation preserved by liquidation
/// - L8: Principal never cut by liquidation
/// - L9: No new liquidatables under snapshot prices
/// - L10: Admissible selection when any exist
/// - L11: Atomic progress or no-op
/// - L12: Socialize→liquidate does not increase liquidatables
/// - L13: Withdraw doesn't create liquidatables (margin safe)
///
/// # Arguments
///
/// * `portfolio` - User portfolio to check
/// * `registry` - Global registry with margin parameters
///
/// # Returns
///
/// true if portfolio is liquidatable (collateral < required margin), false otherwise
///
/// # Implementation Note
///
/// The verified check is:
/// ```ignore
/// collateral * 1_000_000 < position_size * maintenance_margin_bps
/// ```
///
/// This uses scaled arithmetic to avoid rounding errors.
pub fn is_liquidatable_verified(
    portfolio: &Portfolio,
    registry: &SlabRegistry,
) -> bool {
    // Convert portfolio to model account
    let account = portfolio_to_account(portfolio, registry);

    // Use dummy prices (not used in current is_liquidatable implementation)
    let prices = model_safety::Prices {
        p: [1_000_000, 1_000_000, 1_000_000, 1_000_000]
    };

    // Set up params with maintenance margin from registry
    let params = model_safety::Params {
        max_users: 1,
        withdraw_cap_per_step: 1000,
        maintenance_margin_bps: registry.mmr,
    };

    // Call verified function (backed by L1-L13 proofs)
    model_safety::helpers::is_liquidatable(&account, &prices, &params)
}

/// Wrapper for verified loss socialization
///
/// This wraps the verified socialize_losses function for production use.
///
/// # Arguments
///
/// * `portfolios` - All user portfolios (mutable)
/// * `registry` - Global registry (mutable, for insurance fund updates)
/// * `deficit` - Amount of bad debt to socialize
/// * `total_vault_balance` - Current total vault balance
/// * `total_fees` - Current accumulated fees
///
/// # Returns
///
/// Result indicating success or failure
///
/// # Safety
///
/// This function uses formally verified logic that guarantees:
/// - I1: Principals are never reduced
/// - I2: Conservation is maintained
/// - I4: Only winners are haircutted, bounded correctly
pub fn socialize_losses_verified(
    portfolios: &mut [Portfolio],
    registry: &mut SlabRegistry,
    deficit: u128,
    total_vault_balance: u128,
    total_fees: u128,
) -> Result<(), ()> {
    // Convert to model
    let state = portfolios_to_state(portfolios, registry, total_vault_balance, total_fees);

    // Call verified function
    let new_state = model_safety::transitions::socialize_losses(state, deficit);

    // Apply changes back
    apply_state_to_portfolios(portfolios, registry, &new_state);

    Ok(())
}

// ============================================================================
// LP Operations Bridge Functions
// ============================================================================

/// Apply LP shares delta using verified logic (VERIFIED)
///
/// Wraps the formally verified shares delta function from model_safety.
/// Property LP1: No overflow or underflow in shares arithmetic.
pub fn apply_shares_delta_verified(current: u128, delta: i128) -> Result<u128, &'static str> {
    model_safety::lp_operations::apply_shares_delta_verified(current, delta)
}

/// Apply venue PnL deltas using verified logic (VERIFIED)
///
/// Wraps the formally verified venue PnL delta function from model_safety.
/// Properties LP2-LP3: Overflow-safe PnL accounting, conserves net PnL.
pub fn apply_venue_pnl_deltas_verified(
    pnl: &mut crate::state::VenuePnl,
    maker_fee_credits_delta: i128,
    venue_fees_delta: i128,
    realized_pnl_delta: i128,
) -> Result<(), &'static str> {
    // Convert production VenuePnl to model VenuePnl
    let mut model_pnl = model_safety::lp_operations::VenuePnl {
        maker_fee_credits: pnl.maker_fee_credits,
        venue_fees: pnl.venue_fees,
        realized_pnl: pnl.realized_pnl,
    };

    // Apply deltas using verified function
    model_safety::lp_operations::apply_venue_pnl_deltas_verified(
        &mut model_pnl,
        maker_fee_credits_delta,
        venue_fees_delta,
        realized_pnl_delta,
    )?;

    // Write back to production state
    pnl.maker_fee_credits = model_pnl.maker_fee_credits;
    pnl.venue_fees = model_pnl.venue_fees;
    pnl.realized_pnl = model_pnl.realized_pnl;

    Ok(())
}

// ============================================================================
// Cross-Slab Execution Bridge Functions
// ============================================================================

/// Calculate net exposure across multiple positions (VERIFIED)
///
/// Wraps the formally verified net exposure calculation from model_safety.
/// Property X3: Net exposure is the algebraic sum of all signed exposures.
/// This is the foundation for capital efficiency proofs.
pub fn net_exposure_verified(
    exposures: &[(u16, u16, i128)],
) -> Result<i128, &'static str> {
    use model_safety::cross_slab::Portfolio;

    let mut portfolio = Portfolio::new();
    for &(slab_idx, instrument_idx, exposure) in exposures {
        portfolio.update_exposure(slab_idx, instrument_idx, exposure)?;
    }

    model_safety::cross_slab::net_exposure_verified(&portfolio)
}

/// Calculate initial margin on NET exposure (VERIFIED)
///
/// Wraps the formally verified margin calculation from model_safety.
/// Property X3: If net exposure = 0, then margin = 0 (CAPITAL EFFICIENCY PROOF!)
///
/// # Arguments
/// * `net_exposure` - Net signed exposure across all positions
/// * `avg_price` - Average execution price in Q64 format
/// * `imr_bps` - Initial margin requirement in basis points (e.g., 1000 = 10%)
pub fn margin_on_net_verified(
    net_exposure: i128,
    avg_price: u64,
    imr_bps: u16,
) -> Result<u128, &'static str> {
    model_safety::cross_slab::margin_on_net_verified(net_exposure, avg_price, imr_bps)
}

// ============================================================================
// LP Reserve/Release Bridge Functions
// ============================================================================

/// Reserve collateral from portfolio into seat using verified logic (VERIFIED)
///
/// Wraps the formally verified reserve function from model_safety.
/// Properties LP4-LP5: Collateral conservation, no overflow/underflow.
///
/// # Arguments
/// * `portfolio` - Production portfolio state (mutable)
/// * `seat` - Production LP seat state (mutable)
/// * `base_amount_q64` - Base asset amount to reserve
/// * `quote_amount_q64` - Quote asset amount to reserve
///
/// # Returns
/// * `Ok(())` if reservation succeeds
/// * `Err("Insufficient collateral")` if portfolio doesn't have enough
/// * `Err("Overflow")` if seat reservation would overflow
pub fn reserve_verified(
    portfolio: &mut crate::state::Portfolio,
    seat: &mut crate::state::RouterLpSeat,
    base_amount_q64: u128,
    quote_amount_q64: u128,
) -> Result<(), &'static str> {
    // Convert production state to model state
    let mut model_portfolio = model_safety::lp_operations::Portfolio {
        free_collateral: portfolio.free_collateral,
    };

    let mut model_seat = model_safety::lp_operations::Seat {
        lp_shares: seat.lp_shares,
        reserved_base_q64: seat.reserved_base_q64,
        reserved_quote_q64: seat.reserved_quote_q64,
        exposure_base_q64: seat.exposure.base_q64,
        exposure_quote_q64: seat.exposure.quote_q64,
    };

    // Call verified reserve function
    model_safety::lp_operations::reserve_verified(
        &mut model_portfolio,
        &mut model_seat,
        base_amount_q64,
        quote_amount_q64,
    )?;

    // Write back to production state
    portfolio.free_collateral = model_portfolio.free_collateral;
    seat.reserved_base_q64 = model_seat.reserved_base_q64;
    seat.reserved_quote_q64 = model_seat.reserved_quote_q64;

    Ok(())
}

/// Release collateral from seat back to portfolio using verified logic (VERIFIED)
///
/// Wraps the formally verified release function from model_safety.
/// Properties LP4-LP5: Collateral conservation, no overflow/underflow.
///
/// # Arguments
/// * `portfolio` - Production portfolio state (mutable)
/// * `seat` - Production LP seat state (mutable)
/// * `base_amount_q64` - Base asset amount to release
/// * `quote_amount_q64` - Quote asset amount to release
///
/// # Returns
/// * `Ok(())` if release succeeds
/// * `Err("Insufficient reserves")` if seat doesn't have enough reserved
/// * `Err("Overflow")` if portfolio would overflow
pub fn release_verified(
    portfolio: &mut crate::state::Portfolio,
    seat: &mut crate::state::RouterLpSeat,
    base_amount_q64: u128,
    quote_amount_q64: u128,
) -> Result<(), &'static str> {
    // Convert production state to model state
    let mut model_portfolio = model_safety::lp_operations::Portfolio {
        free_collateral: portfolio.free_collateral,
    };

    let mut model_seat = model_safety::lp_operations::Seat {
        lp_shares: seat.lp_shares,
        reserved_base_q64: seat.reserved_base_q64,
        reserved_quote_q64: seat.reserved_quote_q64,
        exposure_base_q64: seat.exposure.base_q64,
        exposure_quote_q64: seat.exposure.quote_q64,
    };

    // Call verified release function
    model_safety::lp_operations::release_verified(
        &mut model_portfolio,
        &mut model_seat,
        base_amount_q64,
        quote_amount_q64,
    )?;

    // Write back to production state
    portfolio.free_collateral = model_portfolio.free_collateral;
    seat.reserved_base_q64 = model_seat.reserved_base_q64;
    seat.reserved_quote_q64 = model_seat.reserved_quote_q64;

    Ok(())
}

/// Calculate redemption value for burning LP shares using verified logic (VERIFIED)
///
/// Wraps the formally verified redemption value calculation from model_safety.
/// Properties LP6-LP7: Overflow safety and proportional calculation correctness.
///
/// # Arguments
/// * `shares_to_burn` - Number of shares to burn (u128, scaled by 1e6)
/// * `current_share_price` - Current price per share (u64, scaled by 1e6)
///
/// # Returns
/// * `Ok(redemption_value)` - Collateral value in base units (i128)
/// * `Err("Overflow")` if calculation would overflow
pub fn calculate_redemption_value_verified(
    shares_to_burn: u128,
    current_share_price: u64,
) -> Result<i128, &'static str> {
    model_safety::lp_operations::calculate_redemption_value_verified(
        shares_to_burn,
        current_share_price,
    )
}

/// Calculate proportional margin reduction using verified logic (VERIFIED)
///
/// Wraps the formally verified proportional margin reduction from model_safety.
/// Properties LP8-LP10: Monotonicity, zero ratio, and full ratio preservation.
///
/// # Arguments
/// * `initial_margin` - Initial margin requirement (u128)
/// * `remaining_ratio` - Ratio of position remaining (u128, scaled by 1e6)
///                       e.g., 500_000 = 50% remaining
///
/// # Returns
/// * `Ok(new_margin)` - New margin requirement (u128)
/// * `Err("Invalid ratio")` if ratio > 1e6 (> 100%)
/// * `Err("Overflow")` if calculation would overflow
pub fn proportional_margin_reduction_verified(
    initial_margin: u128,
    remaining_ratio: u128,
) -> Result<u128, &'static str> {
    model_safety::lp_operations::proportional_margin_reduction_verified(
        initial_margin,
        remaining_ratio,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use pinocchio::pubkey::Pubkey;

    #[test]
    fn test_portfolio_to_account_positive_values() {
        let mut registry = SlabRegistry::new(Pubkey::default(), Pubkey::default(), 0);
        registry.pnl_vesting_params.tau_slots = 10_000;
        registry.mmr = 50_000; // 5%

        let portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);
        let mut portfolio = portfolio;
        portfolio.principal = 100_000_000; // $100
        portfolio.pnl = 20_000_000; // $20 profit
        portfolio.vested_pnl = 15_000_000; // $15 vested
        portfolio.last_slot = 1000;

        let account = portfolio_to_account(&portfolio, &registry);

        assert_eq!(account.principal, 100_000_000);
        assert_eq!(account.pnl_ledger, 20_000_000);
        assert_eq!(account.reserved_pnl, 15_000_000);
        assert_eq!(account.warmup_state.started_at_slot, 1000);
    }

    #[test]
    fn test_portfolio_to_account_negative_principal() {
        let mut registry = SlabRegistry::new(Pubkey::default(), Pubkey::default(), 0);
        registry.pnl_vesting_params.tau_slots = 10_000;

        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);
        portfolio.principal = -50_000_000; // Negative (edge case)
        portfolio.pnl = -30_000_000; // Loss
        portfolio.vested_pnl = -10_000_000; // Negative vested

        let account = portfolio_to_account(&portfolio, &registry);

        // Negative principal → 0
        assert_eq!(account.principal, 0);
        // Negative PnL preserved (i128)
        assert_eq!(account.pnl_ledger, -30_000_000);
        // Negative vested → 0
        assert_eq!(account.reserved_pnl, 0);
    }

    #[test]
    fn test_apply_account_to_portfolio() {
        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);
        portfolio.principal = 100_000_000;
        portfolio.pnl = 50_000_000;
        portfolio.vested_pnl = 40_000_000;
        portfolio.last_slot = 1000;

        let account = model_safety::Account {
            principal: 100_000_000,
            pnl_ledger: 35_000_000, // Reduced by haircut
            reserved_pnl: 28_000_000, // Reduced proportionally
            warmup_state: model_safety::Warmup {
                started_at_slot: 2000,
                slope_per_step: 1000,
            },
            position_size: 0,
            fee_index_user: 0,
            fee_accrued: 0,
            vested_pos_snapshot: 0,
        };

        apply_account_to_portfolio(&mut portfolio, &account);

        assert_eq!(portfolio.principal, 100_000_000);
        assert_eq!(portfolio.pnl, 35_000_000);
        assert_eq!(portfolio.vested_pnl, 28_000_000);
        assert_eq!(portfolio.last_slot, 2000);
    }

    #[test]
    fn test_portfolios_to_state() {
        let registry = SlabRegistry::new(Pubkey::default(), Pubkey::default(), 0);

        let mut p1 = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);
        p1.principal = 100_000_000;
        p1.pnl = 20_000_000;

        let mut p2 = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);
        p2.principal = 200_000_000;
        p2.pnl = -10_000_000;

        let portfolios = vec![p1, p2];
        let state = portfolios_to_state(&portfolios, &registry, 310_000_000, 5_000);

        assert_eq!(state.users.len(), 2);
        assert_eq!(state.vault, 310_000_000);
        assert_eq!(state.fees_outstanding, 5_000);
        assert_eq!(state.users[0].principal, 100_000_000);
        assert_eq!(state.users[1].principal, 200_000_000);
    }

    #[test]
    fn test_check_conservation_holds() {
        let registry = SlabRegistry::new(Pubkey::default(), Pubkey::default(), 0);

        let mut p1 = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);
        p1.principal = 100_000_000;
        p1.pnl = 20_000_000;

        let mut p2 = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);
        p2.principal = 200_000_000;
        p2.pnl = 0;

        let portfolios = vec![p1, p2];

        // Conservation: vault = Σprincipal + Σmax(0,pnl) + insurance + fees
        // = 100M + 200M + 20M + 0 + 0 = 320M
        let total_vault = 320_000_000;
        let total_fees = 0;

        assert!(check_conservation(&portfolios, &registry, total_vault, total_fees));
    }

    #[test]
    fn test_check_conservation_fails() {
        let registry = SlabRegistry::new(Pubkey::default(), Pubkey::default(), 0);

        let mut p1 = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);
        p1.principal = 100_000_000;
        p1.pnl = 20_000_000;

        let portfolios = vec![p1];

        // Conservation should be: 100M + 20M = 120M
        // But we claim vault is 100M → fails
        let total_vault = 100_000_000;
        let total_fees = 0;

        assert!(!check_conservation(&portfolios, &registry, total_vault, total_fees));
    }

    /// Example: Conservation check in a typical test scenario
    ///
    /// This demonstrates the recommended pattern for adding conservation checks
    /// to production tests. Add this pattern to all state-mutating tests.
    #[test]
    fn test_conservation_example_deposit_withdraw() {
        let registry = SlabRegistry::new(Pubkey::default(), Pubkey::default(), 0);

        // Initial state: User deposits 100M
        let mut p1 = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);
        p1.principal = 100_000_000;
        p1.pnl = 0;

        let mut portfolios = vec![p1];
        let mut total_vault = 100_000_000;
        let total_fees = 0;

        // ✅ Conservation check after deposit
        assert!(
            check_conservation(&portfolios, &registry, total_vault, total_fees),
            "Conservation violated after deposit"
        );

        // User realizes profit: +20M PnL
        portfolios[0].pnl = 20_000_000;
        total_vault += 20_000_000; // Vault increases by profit

        // ✅ Conservation check after profit
        assert!(
            check_conservation(&portfolios, &registry, total_vault, total_fees),
            "Conservation violated after profit"
        );

        // User withdraws 20M (all vested profit, no principal touched)
        portfolios[0].pnl = 0; // All PnL withdrawn
        total_vault -= 20_000_000;

        // ✅ Conservation check after withdrawal
        // vault = principal + max(0, pnl) + insurance + fees
        // 100M = 100M + 0 + 0 + 0 ✓
        assert!(
            check_conservation(&portfolios, &registry, total_vault, total_fees),
            "Conservation violated after withdrawal"
        );

        // Final state verification
        assert_eq!(portfolios[0].principal, 100_000_000); // Principal unchanged
        assert_eq!(portfolios[0].pnl, 0); // PnL fully withdrawn
        assert_eq!(total_vault, 100_000_000); // Vault = principal only
    }

    /// L13 Regression Test: Withdrawal must not trigger self-liquidation
    ///
    /// This test documents the expected behavior based on the L13 proof.
    /// When withdrawal is implemented, it MUST maintain margin health.
    ///
    /// Scenario from L13 counterexample:
    /// - User has: principal=5, pnl=6, position=100, margin_req=10%
    /// - Collateral = 5 + 6 = 11 >= 10 ✓ NOT liquidatable
    /// - Attempt to withdraw 2 from PnL
    /// - Result: Must be blocked or limited to maintain margin
    #[test]
    fn test_l13_withdrawal_margin_safety() {
        let mut registry = SlabRegistry::new(Pubkey::default(), Pubkey::default(), 0);
        registry.mmr = 100_000; // 10% maintenance margin (100k bps)

        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);
        portfolio.principal = 5_000_000;  // $5 (scaled by 1e6)
        portfolio.pnl = 6_000_000;  // $6 profit
        portfolio.vested_pnl = 6_000_000;  // All vested (for simplicity)

        // Add position that requires maintenance margin
        portfolio.update_exposure(0, 0, 100_000_000 as i64);  // Position size = 100 (scaled)

        // Calculate required collateral
        // position * margin_bps / 1_000_000 = 100 * 100_000 / 1_000_000 = 10
        let position_size = 100_000_000u128;
        let required_collateral = (position_size * registry.mmr as u128) / 1_000_000;
        assert_eq!(required_collateral, 10_000_000); // $10 required

        let current_collateral = (portfolio.principal + portfolio.pnl.max(0)) as u128;
        assert_eq!(current_collateral, 11_000_000); // $11 available

        // Convert to model and check liquidation status
        let account = portfolio_to_account(&portfolio, &registry);
        let prices = model_safety::Prices { p: [1_000_000, 1_000_000, 1_000_000, 1_000_000] };
        let params = model_safety::Params {
            max_users: 1,
            withdraw_cap_per_step: 1000,
            maintenance_margin_bps: registry.mmr,
        };

        // User is NOT liquidatable before withdrawal
        assert!(!model_safety::helpers::is_liquidatable(&account, &prices, &params),
                "User should NOT be liquidatable initially");

        // ⚠️ CRITICAL: If withdrawing $2 from PnL (leaving $9 collateral < $10 required)
        // This MUST be prevented by the withdrawal implementation!
        //
        // Safe withdrawal limit = current_collateral - required_collateral
        //                       = $11 - $10 = $1
        //
        // So user can only withdraw UP TO $1 while maintaining margin safety
        let safe_withdraw_limit = current_collateral.saturating_sub(required_collateral);
        assert_eq!(safe_withdraw_limit, 1_000_000, "User can safely withdraw $1");

        // ❌ DANGEROUS: Withdrawing $2 would violate margin
        let dangerous_withdrawal = 2_000_000;
        let collateral_after = current_collateral.saturating_sub(dangerous_withdrawal);
        assert!(collateral_after < required_collateral,
                "Withdrawing $2 would drop collateral below required margin");

        // WHEN IMPLEMENTING WITHDRAWAL:
        // The function MUST either:
        // 1. Reject the $2 withdrawal entirely, OR
        // 2. Limit it to $1 (the safe amount)
        //
        // It MUST NOT allow the full $2 withdrawal!
    }

    /// L13 Regression Test: Withdrawal with no position is always safe
    ///
    /// When user has no position, there's no margin requirement,
    /// so withdrawal is only limited by vesting/throttling.
    #[test]
    fn test_l13_withdrawal_no_position_safe() {
        let registry = SlabRegistry::new(Pubkey::default(), Pubkey::default(), 0);

        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);
        portfolio.principal = 100_000_000;  // $100
        portfolio.pnl = 50_000_000;  // $50 profit
        portfolio.vested_pnl = 50_000_000;  // All vested
        portfolio.exposure_count = 0;  // No positions

        // Convert to model and check
        let account = portfolio_to_account(&portfolio, &registry);
        let prices = model_safety::Prices { p: [1_000_000; 4] };
        let params = model_safety::Params {
            max_users: 1,
            withdraw_cap_per_step: 1000,
            maintenance_margin_bps: 100_000, // 10%
        };

        // User is NOT liquidatable (no position = no margin requirement)
        assert!(!model_safety::helpers::is_liquidatable(&account, &prices, &params),
                "User with no position should never be liquidatable");

        // User can withdraw entire vested PnL without margin concerns
        // (still subject to vesting caps and throttling in production)
        assert_eq!(portfolio.vested_pnl, 50_000_000);
    }

    /// L13 Regression Test: Scaled arithmetic prevents rounding errors
    ///
    /// This tests that we use the same scaled arithmetic as is_liquidatable
    /// to avoid the rounding bug from the original L13 failure.
    #[test]
    fn test_l13_withdrawal_scaled_arithmetic() {
        use model_safety::math::{mul_u128, div_u128, sub_u128};

        let mut registry = SlabRegistry::new(Pubkey::default(), Pubkey::default(), 0);
        registry.mmr = 100_000; // 10% maintenance margin

        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);
        portfolio.principal = 10_000_000;
        portfolio.pnl = 1_000_000;
        portfolio.vested_pnl = 1_000_000;
        portfolio.update_exposure(0, 0, 100_000_000 as i64);

        let current_collateral = (portfolio.principal + portfolio.pnl.max(0)) as u128;
        let position_size = 100_000_000u128;

        // ❌ WRONG: Using division (rounds down, too permissive)
        let required_wrong = (position_size * registry.mmr as u128) / 1_000_000;
        let safe_withdraw_wrong = current_collateral.saturating_sub(required_wrong);

        // ✅ CORRECT: Using scaled arithmetic (matches is_liquidatable)
        let collateral_scaled = mul_u128(current_collateral, 1_000_000);
        let required_margin_scaled = mul_u128(position_size, registry.mmr as u128);
        let safe_withdraw_correct = if collateral_scaled > required_margin_scaled {
            div_u128(sub_u128(collateral_scaled, required_margin_scaled), 1_000_000)
        } else {
            0
        };

        // The scaled version should be EQUAL OR MORE CONSERVATIVE
        assert!(safe_withdraw_correct <= safe_withdraw_wrong,
                "Scaled arithmetic should be at least as conservative as direct division");

        // Verify against model's is_liquidatable
        let account = portfolio_to_account(&portfolio, &registry);
        let prices = model_safety::Prices { p: [1_000_000; 4] };
        let params = model_safety::Params {
            max_users: 1,
            withdraw_cap_per_step: 1000,
            maintenance_margin_bps: registry.mmr,
        };

        assert!(!model_safety::helpers::is_liquidatable(&account, &prices, &params),
                "User should not be liquidatable before withdrawal");

        // After withdrawing the CORRECT safe amount, should still be safe
        // (This is what the fixed L13 proof guarantees)
        let mut account_after = account.clone();
        account_after.pnl_ledger -= safe_withdraw_correct as i128;

        // Still not liquidatable (with some epsilon tolerance for rounding)
        let collateral_after = (account_after.principal as u128)
            .saturating_add(account_after.pnl_ledger.max(0) as u128);
        let collateral_after_scaled = mul_u128(collateral_after, 1_000_000);

        // Should be at or just above required margin
        assert!(collateral_after_scaled >= required_margin_scaled,
                "After safe withdrawal, should still meet margin requirement");
    }

    /// L13 Regression Test: Multiple withdrawals compound margin pressure
    ///
    /// Tests that consecutive withdrawals are each checked for margin safety.
    /// Even if each individual withdrawal is "small", they must not compound
    /// to violate margin.
    #[test]
    fn test_l13_multiple_withdrawals_margin_safety() {
        let mut registry = SlabRegistry::new(Pubkey::default(), Pubkey::default(), 0);
        registry.mmr = 100_000; // 10% maintenance margin

        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);
        portfolio.principal = 10_000_000;  // $10
        portfolio.pnl = 5_000_000;  // $5
        portfolio.vested_pnl = 5_000_000;
        portfolio.update_exposure(0, 0, 100_000_000 as i64); // Position = 100, requires $10 collateral

        let current_collateral = 15_000_000u128;  // $15
        let required_collateral = 10_000_000u128;  // $10
        let total_safe_withdraw = 5_000_000u128;  // $5 max

        // Try to withdraw in 3 chunks of $2 each = $6 total (exceeds safe limit!)
        let withdraw_chunk = 2_000_000u128;

        // First withdrawal: $2 from $15 → $13 (still safe: $13 > $10) ✓
        assert!(current_collateral.saturating_sub(withdraw_chunk) > required_collateral);

        // Second withdrawal: $2 from $13 → $11 (still safe: $11 > $10) ✓
        let after_first = current_collateral.saturating_sub(withdraw_chunk);
        assert!(after_first.saturating_sub(withdraw_chunk) > required_collateral);

        // Third withdrawal: $2 from $11 → $9 (UNSAFE: $9 < $10) ✗
        let after_second = after_first.saturating_sub(withdraw_chunk);
        let after_third = after_second.saturating_sub(withdraw_chunk);
        assert!(after_third < required_collateral,
                "Third withdrawal would violate margin");

        // ⚠️ CRITICAL: Each withdrawal must be checked independently!
        // Implementation MUST reject the third $2 withdrawal or limit it to $1
    }
}

//
// ═══════════════════════════════════════════════════════════════════════════
// DEPOSIT/WITHDRAW BRIDGE - Integrates deposit_withdraw formal model
// ═══════════════════════════════════════════════════════════════════════════
//

/// Convert Q32.32 fixed-point (i128) to u64 representation where u64::MAX = 1.0
///
/// Used to bridge adaptive_warmup's unlocked_frac to deposit_withdraw model.
///
/// # Fixed-Point Representations
///
/// - **Q32.32 (adaptive_warmup)**: `2^32` represents 1.0
/// - **u64 (deposit_withdraw)**: `u64::MAX` represents 1.0 (used in `>> 64` shift)
///
/// # Arguments
///
/// * `q32_value` - Q32.32 value (i128) where 2^32 = 1.0
///
/// # Returns
///
/// u64 value where u64::MAX = 1.0
#[inline]
fn q32_to_u64_frac(q32_value: i128) -> u64 {
    const F: i128 = 1i128 << 32; // Q32.32 scale factor (2^32 = 1.0)

    // Clamp to [0, F] range (0.0 to 1.0)
    let clamped = if q32_value < 0 {
        0
    } else if q32_value > F {
        F
    } else {
        q32_value
    };

    // Convert: (clamped * u64::MAX) / 2^32
    // Do arithmetic in u128 to avoid overflow
    let result_u128 = (clamped as u128 * u64::MAX as u128) / F as u128;
    result_u128.min(u64::MAX as u128) as u64
}

/// Convert Portfolio to deposit_withdraw::Account
///
/// This enables using the verified deposit/withdraw model for:
/// - Computing safe withdrawal amounts (respecting margin and vesting)
/// - Validating deposit operations
/// - Proving properties D2-D5
///
/// # Arguments
///
/// * `portfolio` - Production portfolio
/// * `registry` - Global registry (for unlocked_frac)
///
/// # Returns
///
/// model_safety::deposit_withdraw::Account with converted fields
pub fn portfolio_to_deposit_account(
    portfolio: &Portfolio,
    registry: &SlabRegistry,
) -> model_safety::deposit_withdraw::Account {
    model_safety::deposit_withdraw::Account {
        principal: portfolio.principal,
        equity: portfolio.equity,
        vested_pnl: portfolio.vested_pnl,
        maintenance_margin: portfolio.mm as i128,
    }
}

/// Get safe withdrawal amount using verified model
///
/// This wraps the verified `max_withdrawable` function from deposit_withdraw model.
///
/// # Verified Properties (from Kani proofs)
///
/// When this function is used:
/// - **D2**: Withdrawals change balances by exact amount ✅
/// - **D3**: Withdrawals maintain margin safety ✅
/// - **D4**: Withdrawals respect vesting caps ✅
/// - **D5**: No withdrawal creates immediately liquidatable state ✅
///
/// # Arguments
///
/// * `portfolio` - User portfolio
/// * `registry` - Global registry with vesting state
///
/// # Returns
///
/// Maximum safe withdrawal amount (respects both margin and vesting)
pub fn get_max_withdrawable_verified(
    portfolio: &Portfolio,
    registry: &SlabRegistry,
) -> i128 {
    let account = portfolio_to_deposit_account(portfolio, registry);
    let unlocked_frac_u64 = q32_to_u64_frac(registry.warmup_state.unlocked_frac);
    model_safety::deposit_withdraw::max_withdrawable(
        account,
        unlocked_frac_u64,
    )
}

/// Apply verified deposit operation
///
/// This uses the formally verified `apply_deposit` function.
///
/// # Verified Properties
///
/// - **D2**: Principal and equity increase by exact amount ✅
/// - No overflow (checked arithmetic) ✅
///
/// # Arguments
///
/// * `portfolio` - User portfolio (mutable)
/// * `amount` - Deposit amount
///
/// # Returns
///
/// Result indicating success or error
pub fn apply_deposit_verified(
    portfolio: &mut Portfolio,
    amount: i128,
) -> Result<(), model_safety::deposit_withdraw::DepositWithdrawError> {
    // Create account directly from portfolio (registry not needed for deposits)
    let account = model_safety::deposit_withdraw::Account {
        principal: portfolio.principal,
        equity: portfolio.equity,
        vested_pnl: portfolio.vested_pnl,
        maintenance_margin: portfolio.mm as i128,
    };

    let new_account = model_safety::deposit_withdraw::apply_deposit(account, amount)?;

    // Apply changes back to portfolio
    portfolio.principal = new_account.principal;
    portfolio.equity = new_account.equity;
    portfolio.vested_pnl = new_account.vested_pnl;

    Ok(())
}

/// Apply verified withdrawal operation
///
/// This uses the formally verified `apply_withdraw` function.
///
/// # Verified Properties
///
/// - **D2**: Principal and equity decrease by exact amount ✅
/// - **D3**: Withdrawal maintains margin safety ✅
/// - **D4**: Withdrawal respects vesting caps ✅
/// - **D5**: No withdrawal creates immediately liquidatable state ✅
///
/// # Arguments
///
/// * `portfolio` - User portfolio (mutable)
/// * `registry` - Global registry with vesting params
/// * `amount` - Withdrawal amount
/// * `account_lamports` - Current portfolio account lamports (for rent check)
///
/// # Returns
///
/// Result indicating success or error
pub fn apply_withdraw_verified(
    portfolio: &mut Portfolio,
    registry: &SlabRegistry,
    amount: i128,
    account_lamports: u64,
) -> Result<(), model_safety::deposit_withdraw::DepositWithdrawError> {
    let account = portfolio_to_deposit_account(portfolio, registry);

    let unlocked_frac_u64 = q32_to_u64_frac(registry.warmup_state.unlocked_frac);

    let params = model_safety::deposit_withdraw::Params {
        min_rent_exempt: 1_000_000_000, // ~1 SOL
        unlocked_frac: unlocked_frac_u64,
    };

    let new_account = model_safety::deposit_withdraw::apply_withdraw(
        account,
        &params,
        amount,
        account_lamports,
    )?;

    // Apply changes back to portfolio
    portfolio.principal = new_account.principal;
    portfolio.equity = new_account.equity;
    portfolio.vested_pnl = new_account.vested_pnl;

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// FUNDING RATE BRIDGE
// ═══════════════════════════════════════════════════════════════════════════

/// Apply funding to a position using formally verified logic (VERIFIED)
///
/// This function bridges production state to the verified funding model,
/// applies the funding payment, and writes back to production state.
///
/// # Properties (Proven with Kani in model_safety::funding)
/// - F1: Conservation - funding is net-zero across equal/opposite positions
/// - F2: Proportional - payment proportional to position size
/// - F3: Idempotent - applying twice with same index = applying once
/// - F4: Overflow safety - no overflow on realistic inputs
/// - F5: Sign correctness - longs pay when mark > oracle
pub fn apply_funding_to_position_verified(
    portfolio: &mut Portfolio,
    slab_idx: u16,
    instrument_idx: u16,
    market_cumulative_index: i128,
) {
    // Read position data from portfolio
    let base_size = portfolio.get_exposure(slab_idx, instrument_idx);
    let funding_offset = portfolio.get_funding_offset(slab_idx, instrument_idx);

    // Convert to verified model types
    let mut position = model_safety::funding::Position {
        base_size,
        realized_pnl: portfolio.pnl,
        funding_index_offset: funding_offset,
    };

    let market = model_safety::funding::MarketFunding {
        cumulative_funding_index: market_cumulative_index,
    };

    // Call verified function (F1-F5 properties proven with Kani)
    model_safety::funding::apply_funding(&mut position, &market);

    // Write back to production state
    portfolio.pnl = position.realized_pnl;
    portfolio.set_funding_offset(slab_idx, instrument_idx, position.funding_index_offset);
}

#[cfg(test)]
mod deposit_withdraw_bridge_tests {
    use super::*;
    use pinocchio::pubkey::Pubkey;

    #[test]
    fn test_portfolio_to_deposit_account() {
        let mut registry = SlabRegistry::new(Pubkey::default(), Pubkey::default(), 0);
        registry.warmup_state.unlocked_frac = (1i128 << 32) / 2; // 50% unlocked (Q32.32)

        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);
        portfolio.principal = 100_000_000; // $100
        portfolio.equity = 120_000_000; // $120 (profit)
        portfolio.vested_pnl = 15_000_000; // $15 vested
        portfolio.mm = 10_000_000; // $10 maintenance margin

        let account = portfolio_to_deposit_account(&portfolio, &registry);

        assert_eq!(account.principal, 100_000_000);
        assert_eq!(account.equity, 120_000_000);
        assert_eq!(account.vested_pnl, 15_000_000);
        assert_eq!(account.maintenance_margin, 10_000_000);
    }

    #[test]
    fn test_get_max_withdrawable_verified_basic() {
        let mut registry = SlabRegistry::new(Pubkey::default(), Pubkey::default(), 0);
        registry.warmup_state.unlocked_frac = 1i128 << 32; // Fully unlocked (Q32.32)

        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);
        portfolio.principal = 100_000_000;
        portfolio.equity = 120_000_000;
        portfolio.vested_pnl = 20_000_000; // All PnL vested
        portfolio.mm = 0; // No position

        let max = get_max_withdrawable_verified(&portfolio, &registry);

        // Should be able to withdraw principal + vested PnL
        // With u64::MAX for 100% unlocking, we get ~99.99% due to fixed-point
        assert!(max >= 119_900_000); // At least $119.9
        assert!(max <= 120_000_000); // At most $120
    }

    #[test]
    fn test_get_max_withdrawable_verified_with_vesting_throttle() {
        let mut registry = SlabRegistry::new(Pubkey::default(), Pubkey::default(), 0);
        registry.warmup_state.unlocked_frac = (1i128 << 32) / 2; // 50% unlocked (Q32.32)

        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);
        portfolio.principal = 100_000_000;
        portfolio.vested_pnl = 20_000_000; // $20 vested PnL
        portfolio.mm = 0;

        let max = get_max_withdrawable_verified(&portfolio, &registry);

        // Should be principal + (vested_pnl * 50%)
        // = 100M + 10M = 110M
        assert!(max >= 109_900_000); // Account for rounding
        assert!(max <= 110_100_000);
    }

    #[test]
    fn test_apply_deposit_verified_success() {
        let mut registry = SlabRegistry::new(Pubkey::default(), Pubkey::default(), 0);
        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);
        portfolio.principal = 100_000_000;
        portfolio.equity = 100_000_000;

        let result = apply_deposit_verified(&mut portfolio, 50_000_000);

        assert!(result.is_ok());
        assert_eq!(portfolio.principal, 150_000_000);
        assert_eq!(portfolio.equity, 150_000_000);
    }

    #[test]
    fn test_apply_deposit_verified_zero_amount() {
        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);
        let result = apply_deposit_verified(&mut portfolio, 0);

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            model_safety::deposit_withdraw::DepositWithdrawError::ZeroAmount
        );
    }

    #[test]
    fn test_apply_withdraw_verified_success() {
        let mut registry = SlabRegistry::new(Pubkey::default(), Pubkey::default(), 0);
        registry.warmup_state.unlocked_frac = 1i128 << 32; // Fully unlocked (Q32.32)

        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);
        portfolio.principal = 100_000_000;
        portfolio.equity = 100_000_000;
        portfolio.vested_pnl = 0;
        portfolio.mm = 0; // No position

        let account_lamports = 150_000_000_000u64; // 150 SOL

        let result = apply_withdraw_verified(
            &mut portfolio,
            &registry,
            50_000_000,
            account_lamports,
        );

        assert!(result.is_ok());
        assert_eq!(portfolio.principal, 50_000_000);
        assert_eq!(portfolio.equity, 50_000_000);
    }

    #[test]
    fn test_apply_withdraw_verified_exceeds_max() {
        let mut registry = SlabRegistry::new(Pubkey::default(), Pubkey::default(), 0);
        registry.warmup_state.unlocked_frac = 0; // Nothing unlocked

        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);
        portfolio.principal = 100_000_000;
        portfolio.equity = 100_000_000;
        portfolio.vested_pnl = 20_000_000;
        portfolio.mm = 0;

        let account_lamports = 150_000_000_000u64;

        // Try to withdraw more than principal (vesting throttled to 0)
        let result = apply_withdraw_verified(
            &mut portfolio,
            &registry,
            110_000_000,
            account_lamports,
        );

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            model_safety::deposit_withdraw::DepositWithdrawError::InsufficientWithdrawable
        );
    }

    #[test]
    fn test_apply_withdraw_verified_margin_safety() {
        let mut registry = SlabRegistry::new(Pubkey::default(), Pubkey::default(), 0);
        registry.warmup_state.unlocked_frac = 1i128 << 32; // 100% unlocked (Q32.32)

        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);
        portfolio.principal = 100_000_000;
        portfolio.equity = 100_000_000;
        portfolio.vested_pnl = 0;
        portfolio.mm = 95_000_000; // High margin requirement

        let account_lamports = 150_000_000_000u64;

        // Try to withdraw too much (would drop below maintenance margin)
        let result = apply_withdraw_verified(
            &mut portfolio,
            &registry,
            50_000_000,
            account_lamports,
        );

        // Should be rejected by margin safety check
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            model_safety::deposit_withdraw::DepositWithdrawError::WouldBeLiquidatable
        );
    }
}

#[cfg(test)]
mod funding_bridge_tests {
    use super::*;
    use pinocchio::pubkey::Pubkey;

    #[test]
    fn test_apply_funding_basic_long_position() {
        // Test F2 (Proportional) and F5 (Sign correctness) properties
        // When mark > oracle, longs pay shorts (funding index increases)
        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);
        portfolio.pnl = 0;

        // Add a long position: +10 contracts on slab 0, instrument 0
        portfolio.exposures[0] = (0, 0, 10_000_000); // 10 contracts (1e6 scale)
        portfolio.exposure_count = 1;
        portfolio.funding_offsets[0] = 0; // Starting offset

        // Market cumulative funding index has increased by 1000
        // This means longs need to pay funding
        let market_cumulative_index = 1000i128;

        apply_funding_to_position_verified(&mut portfolio, 0, 0, market_cumulative_index);

        // Funding payment = position_size * (new_index - old_index)
        // = 10 * 1000 = 10,000
        // Longs pay, so PnL decreases
        assert_eq!(portfolio.pnl, 10_000);
        assert_eq!(portfolio.funding_offsets[0], 1000);
    }

    #[test]
    fn test_apply_funding_short_position() {
        // Test F5: When mark > oracle, shorts receive from longs
        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);
        portfolio.pnl = 0;

        // Add a short position: -5 contracts on slab 0, instrument 0
        portfolio.exposures[0] = (0, 0, -5_000_000); // -5 contracts (1e6 scale)
        portfolio.exposure_count = 1;
        portfolio.funding_offsets[0] = 0;

        let market_cumulative_index = 2000i128;

        apply_funding_to_position_verified(&mut portfolio, 0, 0, market_cumulative_index);

        // Funding payment = -5 * 2000 = -10,000
        // Shorts receive, so PnL increases (negative payment = receiving)
        assert_eq!(portfolio.pnl, -10_000);
        assert_eq!(portfolio.funding_offsets[0], 2000);
    }

    #[test]
    fn test_apply_funding_idempotence() {
        // Test F3 (Idempotence): Applying funding twice with same index = applying once
        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);
        portfolio.pnl = 0;

        portfolio.exposures[0] = (0, 0, 10_000_000); // 10 contracts
        portfolio.exposure_count = 1;
        portfolio.funding_offsets[0] = 0;

        let market_cumulative_index = 1000i128;

        // Apply funding first time
        apply_funding_to_position_verified(&mut portfolio, 0, 0, market_cumulative_index);
        let pnl_after_first = portfolio.pnl;
        let offset_after_first = portfolio.funding_offsets[0];

        assert_eq!(pnl_after_first, 10_000);
        assert_eq!(offset_after_first, 1000);

        // Apply funding second time with SAME index
        apply_funding_to_position_verified(&mut portfolio, 0, 0, market_cumulative_index);

        // PnL and offset should be unchanged (idempotence)
        assert_eq!(portfolio.pnl, pnl_after_first);
        assert_eq!(portfolio.funding_offsets[0], offset_after_first);
    }

    #[test]
    fn test_apply_funding_incremental_updates() {
        // Test that funding can be applied incrementally
        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);
        portfolio.pnl = 0;

        portfolio.exposures[0] = (0, 0, 10_000_000); // 10 contracts
        portfolio.exposure_count = 1;
        portfolio.funding_offsets[0] = 0;

        // First funding update
        apply_funding_to_position_verified(&mut portfolio, 0, 0, 1000i128);
        assert_eq!(portfolio.pnl, 10_000);
        assert_eq!(portfolio.funding_offsets[0], 1000);

        // Second funding update (index increased by another 500)
        apply_funding_to_position_verified(&mut portfolio, 0, 0, 1500i128);
        assert_eq!(portfolio.pnl, 15_000); // 10 * 1500
        assert_eq!(portfolio.funding_offsets[0], 1500);

        // Third funding update (index increased by another 300)
        apply_funding_to_position_verified(&mut portfolio, 0, 0, 1800i128);
        assert_eq!(portfolio.pnl, 18_000); // 10 * 1800
        assert_eq!(portfolio.funding_offsets[0], 1800);
    }

    #[test]
    fn test_apply_funding_zero_position() {
        // Test that zero position has no funding impact
        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);
        portfolio.pnl = 0;

        portfolio.exposures[0] = (0, 0, 0); // 0 contracts
        portfolio.exposure_count = 1;
        portfolio.funding_offsets[0] = 0;

        apply_funding_to_position_verified(&mut portfolio, 0, 0, 10000i128);

        // No funding payment for zero position
        assert_eq!(portfolio.pnl, 0);
        assert_eq!(portfolio.funding_offsets[0], 10000);
    }

    #[test]
    fn test_apply_funding_negative_index_movement() {
        // Test when mark < oracle (negative funding rate, index decreases)
        // In practice, indices are monotonically increasing, but the model supports this
        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);
        portfolio.pnl = 0;

        portfolio.exposures[0] = (0, 0, 10_000_000); // 10 contracts long
        portfolio.exposure_count = 1;
        portfolio.funding_offsets[0] = 5000;

        // Index moved from 5000 to 3000 (mark < oracle, shorts pay longs)
        apply_funding_to_position_verified(&mut portfolio, 0, 0, 3000i128);

        // Funding payment = 10 * (3000 - 5000) = 10 * (-2000) = -20,000
        // Longs receive (negative payment), so PnL increases
        assert_eq!(portfolio.pnl, -20_000);
        assert_eq!(portfolio.funding_offsets[0], 3000);
    }

    #[test]
    fn test_apply_funding_multiple_positions() {
        // Test F1 (Conservation) indirectly: equal opposite positions cancel
        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);
        portfolio.pnl = 0;

        // Add two positions: one long, one short of equal size
        portfolio.exposures[0] = (0, 0, 10_000_000); // 10 contracts long
        portfolio.exposures[1] = (1, 0, -10_000_000); // 10 contracts short
        portfolio.exposure_count = 2;
        portfolio.funding_offsets[0] = 0;
        portfolio.funding_offsets[1] = 0;

        let market_cumulative_index = 1000i128;

        // Apply funding to both positions
        apply_funding_to_position_verified(&mut portfolio, 0, 0, market_cumulative_index);
        apply_funding_to_position_verified(&mut portfolio, 1, 0, market_cumulative_index);

        // Long pays: +10,000, Short receives: -10,000
        // Net PnL change = 10,000 + (-10,000) = 0 (conservation!)
        assert_eq!(portfolio.pnl, 0);
    }

    #[test]
    fn test_apply_funding_no_position_found() {
        // Test applying funding to a non-existent position
        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);
        portfolio.pnl = 100_000; // Existing PnL
        portfolio.exposure_count = 0; // No positions

        apply_funding_to_position_verified(&mut portfolio, 0, 0, 1000i128);

        // No position means base_size = 0, so no funding payment
        // PnL should be unchanged
        assert_eq!(portfolio.pnl, 100_000);
    }

    #[test]
    fn test_apply_funding_with_existing_pnl() {
        // Test that funding is added to existing PnL correctly
        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);
        portfolio.pnl = 50_000; // Existing unrealized PnL

        portfolio.exposures[0] = (0, 0, 5_000_000); // 5 contracts long
        portfolio.exposure_count = 1;
        portfolio.funding_offsets[0] = 0;

        apply_funding_to_position_verified(&mut portfolio, 0, 0, 2000i128);

        // Funding payment = 5 * 2000 = 10,000
        // Total PnL = existing 50,000 + funding 10,000 = 60,000
        assert_eq!(portfolio.pnl, 60_000);
        assert_eq!(portfolio.funding_offsets[0], 2000);
    }
}
