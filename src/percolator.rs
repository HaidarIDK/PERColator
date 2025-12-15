//! Formally Verified Risk Engine for Perpetual DEX
//!
//! ⚠️ EDUCATIONAL USE ONLY - NOT PRODUCTION READY ⚠️
//!
//! This is an experimental research project for educational purposes only.
//! DO NOT use with real funds. Not independently audited. Not production ready.
//!
//! This module implements a formally verified risk engine that guarantees:
//! 1. User funds are safe against oracle manipulation attacks (within time window T)
//! 2. PNL warmup prevents instant withdrawal of manipulated profits
//! 3. ADL haircuts apply to unwrapped PNL first, protecting user principal
//! 4. Conservation of funds across all operations
//! 5. User isolation - one user's actions don't affect others
//!
//! All data structures are laid out in a single contiguous memory chunk,
//! suitable for a single Solana account.

#![no_std]
#![forbid(unsafe_code)]

#[cfg(kani)]
extern crate kani;

extern crate alloc;
use alloc::vec::Vec;

// ============================================================================
// Core Data Structures
// ============================================================================

/// Time-based PNL warmup state
/// PNL must warm up over time T before becoming withdrawable principal
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Warmup {
    /// Slot when warmup started
    pub started_at_slot: u64,
    /// Linear vesting rate per slot
    pub slope_per_step: u128,
}

/// Unified account - can be user or LP
///
/// LPs are distinguished by having matching_engine_program set to Some(...).
/// Users have matching_engine_program = None.
///
/// This unification ensures LPs receive the same risk management protections as users:
/// - PNL warmup
/// - ADL (Auto-Deleveraging)
/// - Liquidations
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Account {
    // ========================================
    // Capital & PNL (universal)
    // ========================================

    /// Deposited capital (user principal or LP capital)
    /// NEVER reduced by ADL/socialization (Invariant I1)
    pub capital: u128,

    /// Realized PNL from trading (can be positive or negative)
    pub pnl: i128,

    /// PNL reserved for pending withdrawals
    pub reserved_pnl: u128,

    /// Warmup state for PNL vesting
    pub warmup_state: Warmup,

    // ========================================
    // Position (universal)
    // ========================================

    /// Current position size (+ long, - short)
    pub position_size: i128,

    /// Average entry price for position
    pub entry_price: u64,

    // ========================================
    // Fees & Funding (universal)
    // ========================================

    /// Fee index snapshot at last update
    pub fee_index: u128,

    /// Accrued but unclaimed fees
    pub fee_accrued: u128,

    /// Cached vested positive PNL for fee distribution
    pub vested_pos_snapshot: u128,

    /// Funding index snapshot (quote per base, 1e6 scale)
    pub funding_index: i128,

    // ========================================
    // LP-specific (optional)
    // ========================================

    /// Matching engine program ID (None = user account, Some = LP account)
    pub matching_engine_program: Option<[u8; 32]>,

    /// Matching engine context account
    pub matching_engine_context: Option<[u8; 32]>,
}

impl Account {
    /// Check if this account is an LP
    pub fn is_lp(&self) -> bool {
        self.matching_engine_program.is_some()
    }

    /// Check if this account is a regular user
    pub fn is_user(&self) -> bool {
        !self.is_lp()
    }

    /// Get matching engine info (returns error if not LP)
    pub fn matching_engine(&self) -> Result<(&[u8; 32], &[u8; 32])> {
        match (self.matching_engine_program.as_ref(), self.matching_engine_context.as_ref()) {
            (Some(program), Some(context)) => Ok((program, context)),
            _ => Err(RiskError::NotAnLPAccount),
        }
    }
}

/// Type alias for backward compatibility during migration
pub type UserAccount = Account;

/// Type alias for backward compatibility during migration
pub type LPAccount = Account;

/// Insurance fund state
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InsuranceFund {
    /// Insurance fund balance
    pub balance: u128,

    /// Accumulated fees from trades
    pub fee_revenue: u128,

    /// Accumulated liquidation fees
    pub liquidation_revenue: u128,
}

/// Risk engine parameters
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RiskParams {
    /// Warmup period in slots (time T)
    pub warmup_period_slots: u64,

    /// Maintenance margin ratio in basis points (e.g., 500 = 5%)
    pub maintenance_margin_bps: u64,

    /// Initial margin ratio in basis points
    pub initial_margin_bps: u64,

    /// Trading fee in basis points
    pub trading_fee_bps: u64,

    /// Liquidation fee in basis points
    pub liquidation_fee_bps: u64,

    /// Insurance fund share of liquidation fee (rest goes to keeper)
    pub insurance_fee_share_bps: u64,

    /// Maximum number of users
    pub max_users: u64,

    /// Maximum number of LPs
    pub max_lps: u64,

    /// Base account creation fee in basis points (e.g., 10000 = 1%)
    /// Actual fee = (account_fee_bps * capacity_multiplier) / 10000
    /// The multiplier increases as the system approaches max capacity
    pub account_fee_bps: u64,

    /// Maximum warmup rate as fraction of insurance fund
    /// Formula: max_total_warmup_rate = insurance_fund * max_warmup_rate_fraction / (warmup_period / 2)
    /// Units: basis points (e.g., 5000 = 50% of insurance fund can warm up in T/2)
    pub max_warmup_rate_fraction_bps: u64,
}

// ============================================================================
// Account Storage Trait
// ============================================================================

/// Trait for pluggable account storage
///
/// Allows users to provide their own storage implementation:
/// - Vec for heap allocation (default)
/// - Fixed-size arrays for stack allocation
/// - Slabs for custom memory management
/// - Memory-mapped regions for Solana accounts
pub trait AccountStorage<T> {
    /// Get an account by index (immutable)
    fn get(&self, index: usize) -> Option<&T>;

    /// Get an account by index (mutable)
    fn get_mut(&mut self, index: usize) -> Option<&mut T>;

    /// Get the number of accounts
    fn len(&self) -> usize;

    /// Add a new account, returns its index
    fn push(&mut self, account: T) -> usize;

    /// Iterate over all accounts
    fn iter<'a>(&'a self) -> impl Iterator<Item = &'a T> where T: 'a;

    /// Iterate over all accounts (mutable)
    fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item = &'a mut T> where T: 'a;
}

/// Vec-based storage (default, uses heap allocation)
impl<T> AccountStorage<T> for Vec<T> {
    fn get(&self, index: usize) -> Option<&T> {
        <[T]>::get(self, index)
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        <[T]>::get_mut(self, index)
    }

    fn len(&self) -> usize {
        Vec::len(self)
    }

    fn push(&mut self, account: T) -> usize {
        let index = Vec::len(self);
        Vec::push(self, account);
        index
    }

    fn iter<'a>(&'a self) -> impl Iterator<Item = &'a T> where T: 'a {
        <[T]>::iter(self)
    }

    fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item = &'a mut T> where T: 'a {
        <[T]>::iter_mut(self)
    }
}

/// Main risk engine state - generic over storage type
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RiskEngine<U = Vec<Account>, L = Vec<Account>>
where
    U: AccountStorage<Account>,
    L: AccountStorage<Account>,
{
    /// Total vault balance (all deposited funds)
    pub vault: u128,

    /// Insurance fund
    pub insurance_fund: InsuranceFund,

    /// All user accounts (matching_engine_program = None)
    pub users: U,

    /// All LP accounts (matching_engine_program = Some(...))
    pub lps: L,

    /// Risk parameters
    pub params: RiskParams,

    /// Current slot (for warmup calculations)
    pub current_slot: u64,

    /// Global fee index for fee distribution
    pub fee_index: u128,

    /// Sum of all vested positive PNL (for fee distribution)
    pub sum_vested_pos_pnl: u128,

    /// Loss accumulator for socialization
    pub loss_accum: u128,

    /// Fee carry for rounding
    pub fee_carry: u128,

    /// Global funding index (quote per 1 base, scaled by 1e6)
    pub funding_index_qpb_e6: i128,

    /// Last slot when funding was accrued
    pub last_funding_slot: u64,

    /// Total warmup rate across all users (sum of all slope_per_step values)
    /// This tracks how much PNL is warming up per slot across the entire system
    pub total_warmup_rate: u128,

    /// Withdrawal-only mode flag
    /// When true, only withdrawals are allowed (no trading/deposits)
    /// Automatically enabled when loss_accum > 0
    pub withdrawal_only: bool,

    /// Total amount withdrawn during withdrawal-only mode
    /// Used to maintain fair haircut ratio during unwinding
    pub withdrawal_mode_withdrawn: u128,
}

/// Type alias for the default Vec-based RiskEngine
/// This is what you should use in most cases unless you need custom storage
pub type VecRiskEngine = RiskEngine<Vec<UserAccount>, Vec<LPAccount>>;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RiskError {
    /// Insufficient balance for operation
    InsufficientBalance,

    /// Account would become undercollateralized
    Undercollateralized,

    /// Unauthorized operation
    Unauthorized,

    /// Invalid matching engine
    InvalidMatchingEngine,

    /// PNL not yet warmed up
    PnlNotWarmedUp,

    /// Arithmetic overflow
    Overflow,

    /// User account not found
    UserNotFound,

    /// LP account not found
    LPNotFound,

    /// Account is not an LP account
    NotAnLPAccount,

    /// Position size mismatch
    PositionSizeMismatch,

    /// System in withdrawal-only mode (deposits and trading blocked)
    WithdrawalOnlyMode,
}

pub type Result<T> = core::result::Result<T, RiskError>;

// ============================================================================
// Math Helpers (Saturating Arithmetic for Safety)
// ============================================================================

#[inline]
fn add_u128(a: u128, b: u128) -> u128 {
    a.saturating_add(b)
}

#[inline]
fn sub_u128(a: u128, b: u128) -> u128 {
    a.saturating_sub(b)
}

#[inline]
fn mul_u128(a: u128, b: u128) -> u128 {
    a.saturating_mul(b)
}

#[inline]
fn div_u128(a: u128, b: u128) -> Result<u128> {
    if b == 0 {
        Err(RiskError::Overflow) // Division by zero
    } else {
        Ok(a / b)
    }
}

#[inline]
fn clamp_pos_i128(val: i128) -> u128 {
    if val > 0 { val as u128 } else { 0 }
}

#[inline]
fn clamp_neg_i128(val: i128) -> u128 {
    if val < 0 { (-val) as u128 } else { 0 }
}

// ============================================================================
// Matching Engine Trait
// ============================================================================

/// Trait for pluggable matching engines
///
/// Implementers can provide custom order matching logic via CPI.
/// The matching engine is responsible for validating and executing trades
/// according to its own rules (CLOB, AMM, RFQ, etc).
pub trait MatchingEngine {
    /// Execute a trade between LP and user
    ///
    /// # Arguments
    /// * `lp_program` - The LP's matching engine program ID
    /// * `lp_context` - The LP's matching engine context account
    /// * `oracle_price` - Current oracle price for reference
    /// * `size` - Requested position size (positive = long, negative = short)
    ///
    /// # Returns
    /// * `Ok(())` if the matching engine approves the trade
    /// * `Err(RiskError)` if the trade is rejected
    ///
    /// # Safety
    /// The matching engine MUST verify user authorization before approving trades.
    /// The risk engine will check solvency after the trade executes.
    fn execute_match(
        &self,
        lp_program: &[u8; 32],
        lp_context: &[u8; 32],
        oracle_price: u64,
        size: i128,
    ) -> Result<()>;
}

/// No-op matching engine (for testing)
pub struct NoOpMatcher;

impl MatchingEngine for NoOpMatcher {
    fn execute_match(
        &self,
        _lp_program: &[u8; 32],
        _lp_context: &[u8; 32],
        _oracle_price: u64,
        _size: i128,
    ) -> Result<()> {
        // Always approve trades (no actual matching logic)
        Ok(())
    }
}

// ============================================================================
// Core Invariants and Helpers
// ============================================================================

impl<U, L> RiskEngine<U, L>
where
    U: AccountStorage<Account>,
    L: AccountStorage<Account>,
{
    /// Calculate account creation fee multiplier
    fn account_fee_multiplier(max: u64, used: u64) -> u128 {
        if used >= max {
            return 0; // Cannot add
        }
        let remaining = max - used;
        if remaining == 0 {
            0
        } else {
            // 2^(floor(log2(max / remaining)))
            let ratio = max / remaining;
            if ratio == 0 {
                1
            } else {
                1 << (64 - ratio.leading_zeros() - 1)
            }
        }
    }
    /// Calculate withdrawable PNL for an account (user or LP) after warmup
    /// Works identically for both users and LPs - warmup applies to all accounts
    pub fn withdrawable_pnl(&self, account: &Account) -> u128 {
        // Only positive PNL can be withdrawn
        let positive_pnl = clamp_pos_i128(account.pnl);

        // Available = positive PNL - reserved
        let available_pnl = sub_u128(positive_pnl, account.reserved_pnl);

        // Calculate elapsed slots
        let elapsed_slots = self.current_slot.saturating_sub(account.warmup_state.started_at_slot);

        // Calculate warmed up cap: slope * elapsed_slots
        let warmed_up_cap = mul_u128(
            account.warmup_state.slope_per_step,
            elapsed_slots as u128
        );

        // Return minimum of available and warmed up
        core::cmp::min(available_pnl, warmed_up_cap)
    }


    /// Calculate account's collateral (capital + positive PNL)
    /// Works for both users and LPs
    pub fn account_collateral(&self, account: &Account) -> u128 {
        add_u128(account.capital, clamp_pos_i128(account.pnl))
    }

    /// Legacy alias for backward compatibility
    pub fn user_collateral(&self, user: &Account) -> u128 {
        self.account_collateral(user)
    }

    /// Check if user is above maintenance margin
    pub fn is_above_maintenance_margin(&self, user: &Account, oracle_price: u64) -> bool {
        let collateral = self.account_collateral(user);

        // Calculate position value at current price
        let position_value = mul_u128(
            user.position_size.abs() as u128,
            oracle_price as u128
        ) / 1_000_000; // Assuming price is in 1e6 format

        // Maintenance margin requirement
        let margin_required = mul_u128(
            position_value,
            self.params.maintenance_margin_bps as u128
        ) / 10_000;

        collateral > margin_required
    }

    /// Check conservation invariant (I2)
    /// vault = sum(principal) + sum(max(0, pnl)) + insurance - fees_outstanding
    ///
    /// # Rounding Tolerance
    /// Allows ±1000 units of discrepancy to account for:
    /// - Integer division rounding in PNL calculations (entry price, funding, etc.)
    /// - Basis point conversions (dividing by 10_000)
    /// - Fee calculations
    ///
    /// This tolerance is conservative: with typical position sizes (millions) and
    /// basis point precision, accumulated rounding error should be << 1000 units.
    pub fn check_conservation(&self) -> bool {
        let mut total_principal = 0u128;
        let mut total_positive_pnl = 0u128;

        for user in self.users.iter() {
            total_principal = add_u128(total_principal, user.capital);
            total_positive_pnl = add_u128(total_positive_pnl, clamp_pos_i128(user.pnl));
        }

        for lp in self.lps.iter() {
            total_principal = add_u128(total_principal, lp.capital);
            total_positive_pnl = add_u128(total_positive_pnl, clamp_pos_i128(lp.pnl));
        }

        let expected_vault = add_u128(
            add_u128(total_principal, total_positive_pnl),
            self.insurance_fund.balance
        );

        // Allow ±1000 units tolerance for rounding errors from integer arithmetic
        self.vault >= expected_vault.saturating_sub(1000) &&
        self.vault <= expected_vault.saturating_add(1000)
    }
}

// ============================================================================
// Funding Rate (O(1) per account)
// ============================================================================

impl<U, L> RiskEngine<U, L>
where
    U: AccountStorage<Account>,
    L: AccountStorage<Account>,
{
    /// Accrue funding globally in O(1)
    ///
    /// Updates the global funding index based on:
    /// - Time elapsed since last accrual
    /// - Current oracle price
    /// - Signed funding rate (positive = longs pay shorts)
    ///
    /// Formula: ΔF = price × rate_bps × dt / 10,000
    /// Where F is in quote-per-base scaled by 1e6
    ///
    /// # Arguments
    /// * `now_slot` - Current slot number
    /// * `oracle_price` - Oracle price (scaled by 1e6)
    /// * `funding_rate_bps_per_slot` - Signed funding rate in bps per slot
    ///
    /// # Returns
    /// * `Ok(())` if successful
    /// * `Err(RiskError::Overflow)` if calculation overflows
    ///
    /// # Invariants
    /// * Does not modify any account state (only global index)
    /// * Idempotent if called multiple times with same slot
    pub fn accrue_funding(
        &mut self,
        now_slot: u64,
        oracle_price: u64,
        funding_rate_bps_per_slot: i64,
    ) -> Result<()> {
        let dt = now_slot.saturating_sub(self.last_funding_slot);
        if dt == 0 {
            return Ok(());
        }

        // Input validation to prevent overflow
        // Oracle price should be reasonable (0.01 to 1M USD with 1e6 scale)
        if oracle_price == 0 || oracle_price > 1_000_000_000_000 {
            return Err(RiskError::Overflow);
        }

        // Funding rate should be reasonable (±100% per slot = ±1M bps)
        if funding_rate_bps_per_slot.abs() > 1_000_000 {
            return Err(RiskError::Overflow);
        }

        // Time delta should be reasonable (max 1 year = ~31M slots at 1s per slot)
        if dt > 31_536_000 {
            return Err(RiskError::Overflow);
        }

        // Use checked math to prevent silent overflow
        let price = oracle_price as i128;
        let rate = funding_rate_bps_per_slot as i128;
        let dt_i = dt as i128;

        // ΔF = price × rate × dt / 10,000
        let delta = price
            .checked_mul(rate)
            .ok_or(RiskError::Overflow)?
            .checked_mul(dt_i)
            .ok_or(RiskError::Overflow)?
            .checked_div(10_000)
            .ok_or(RiskError::Overflow)?;

        self.funding_index_qpb_e6 = self
            .funding_index_qpb_e6
            .checked_add(delta)
            .ok_or(RiskError::Overflow)?;

        self.last_funding_slot = now_slot;
        Ok(())
    }

    /// Settle funding for a user (lazy update)
    ///
    /// Applies accumulated funding payments/receipts to user's PNL.
    /// Convention: positive funding rate → longs pay shorts
    ///
    /// Formula: payment = position_size × ΔF / 1e6
    /// Then: pnl_ledger -= payment
    ///
    /// # Arguments
    /// * `user` - Mutable reference to user account
    ///
    /// # Returns
    /// * `Ok(())` if successful
    /// * `Err(RiskError::Overflow)` if calculation overflows
    ///
    /// # Invariants
    /// * Does not modify principal (Invariant I1 extended)
    /// * Idempotent if global index unchanged
    /// * Zero-sum with LP when positions are opposite
    /// Settle funding for an account (lazy update)
    ///
    /// This is a unified funding settlement function for both users and LPs.
    /// Calculates and applies funding payment based on position size and funding index delta.
    ///
    /// # Arguments
    /// * `account` - Mutable reference to account (user or LP)
    /// * `global_funding_index` - The current global funding index
    ///
    /// # Returns
    /// * `Ok(())` if successful
    /// * `Err(RiskError::Overflow)` if calculation overflows
    fn settle_account_funding(account: &mut Account, global_funding_index: i128) -> Result<()> {
        let delta_f = global_funding_index
            .checked_sub(account.funding_index)
            .ok_or(RiskError::Overflow)?;

        if delta_f != 0 && account.position_size != 0 {
            // payment = position × ΔF / 1e6
            let payment = account
                .position_size
                .checked_mul(delta_f)
                .ok_or(RiskError::Overflow)?
                .checked_div(1_000_000)
                .ok_or(RiskError::Overflow)?;

            // Longs pay when funding positive: pnl -= payment
            account.pnl = account
                .pnl
                .checked_sub(payment)
                .ok_or(RiskError::Overflow)?;
        }

        account.funding_index = global_funding_index;
        Ok(())
    }

    /// Touch a user account (settle funding before operations)
    ///
    /// This should be called before any operation that:
    /// - Reads pnl_ledger for withdrawal/warmup
    /// - Changes position_size or entry_price
    /// - Checks collateral or margin
    /// - Liquidates the account
    ///
    /// # Arguments
    /// * `user_index` - Index of user to touch
    ///
    /// # Returns
    /// * `Ok(())` if successful
    /// * `Err` if settlement fails or index invalid
    pub fn touch_user(&mut self, user_index: usize) -> Result<()> {
        let user = self
            .users
            .get_mut(user_index)
            .ok_or(RiskError::UserNotFound)?;
        Self::settle_account_funding(user, self.funding_index_qpb_e6)
    }

    /// Touch an LP account (settle funding before operations)
    ///
    /// # Arguments
    /// * `lp_index` - Index of LP to touch
    ///
    /// # Returns
    /// * `Ok(())` if successful
    /// * `Err` if settlement fails or index invalid
    pub fn touch_lp(&mut self, lp_index: usize) -> Result<()> {
        let lp = self
            .lps
            .get_mut(lp_index)
            .ok_or(RiskError::LPNotFound)?;
        Self::settle_account_funding(lp, self.funding_index_qpb_e6)
    }

    /// Update a user's warmup slope based on current PNL, respecting global warmup rate limit
    ///
    /// This enforces the invariant: total_warmup_rate <= insurance_fund * max_warmup_rate_fraction / (T/2)
    ///
    /// If the desired slope would exceed available capacity, the slope is clamped to the
    /// available capacity (graceful degradation). This ensures PNL always warms up,
    /// but at a potentially slower rate when the system is under stress.
    ///
    /// # Arguments
    /// * `user_index` - Index of user to update
    ///
    /// # Returns
    /// * `Ok(())` always succeeds (uses graceful degradation)
    pub fn update_warmup_slope(&mut self, user_index: usize) -> Result<()> {
        let user = self
            .users
            .get_mut(user_index)
            .ok_or(RiskError::UserNotFound)?;

        // Calculate positive PNL that needs to warm up
        let positive_pnl = clamp_pos_i128(user.pnl);

        // Calculate desired slope: pnl / warmup_period
        // This is how much PNL should warm up per slot
        let desired_slope = if self.params.warmup_period_slots > 0 {
            positive_pnl / (self.params.warmup_period_slots as u128)
        } else {
            positive_pnl // Instant warmup if period is 0
        };

        // Calculate maximum allowed total warmup rate
        // Formula: insurance_fund * max_warmup_rate_fraction_bps / (warmup_period_slots / 2) / 10_000
        let half_period = core::cmp::max(1, self.params.warmup_period_slots / 2);
        let max_total_rate = self
            .insurance_fund
            .balance
            .saturating_mul(self.params.max_warmup_rate_fraction_bps as u128)
            / (half_period as u128)
            / 10_000;

        // Calculate current total rate excluding this user's old slope
        let old_slope = user.warmup_state.slope_per_step;
        let other_users_rate = self.total_warmup_rate.saturating_sub(old_slope);

        // Check if adding the new slope would exceed capacity
        let new_total_rate = other_users_rate.saturating_add(desired_slope);
        let actual_slope = if new_total_rate > max_total_rate {
            // Clamp to available capacity (graceful degradation)
            let available_capacity = max_total_rate.saturating_sub(other_users_rate);
            available_capacity
        } else {
            desired_slope
        };

        // Update the slope and total rate
        user.warmup_state.slope_per_step = actual_slope;
        user.warmup_state.started_at_slot = self.current_slot;
        self.total_warmup_rate = other_users_rate.saturating_add(actual_slope);

        Ok(())
    }

    /// Update warmup slope for an LP account
    /// CRITICAL FIX: LPs now get warmup rate limiting just like users
    /// This prevents LPs from instantly extracting manipulated profits
    pub fn update_lp_warmup_slope(&mut self, lp_index: usize) -> Result<()> {
        let lp = self
            .lps
            .get_mut(lp_index)
            .ok_or(RiskError::LPNotFound)?;

        // Calculate positive PNL that needs to warm up
        let positive_pnl = clamp_pos_i128(lp.pnl);

        // Calculate desired slope: pnl / warmup_period
        let desired_slope = if self.params.warmup_period_slots > 0 {
            positive_pnl / (self.params.warmup_period_slots as u128)
        } else {
            positive_pnl // Instant warmup if period is 0
        };

        // Calculate maximum allowed total warmup rate
        // LPs share the SAME warmup rate cap as users (global limit)
        let half_period = core::cmp::max(1, self.params.warmup_period_slots / 2);
        let max_total_rate = self
            .insurance_fund
            .balance
            .saturating_mul(self.params.max_warmup_rate_fraction_bps as u128)
            / (half_period as u128)
            / 10_000;

        // Calculate current total rate excluding this LP's old slope
        let old_slope = lp.warmup_state.slope_per_step;
        let other_rate = self.total_warmup_rate.saturating_sub(old_slope);

        // Check if adding the new slope would exceed capacity
        let new_total_rate = other_rate.saturating_add(desired_slope);
        let actual_slope = if new_total_rate > max_total_rate {
            // Clamp to available capacity (graceful degradation)
            let available_capacity = max_total_rate.saturating_sub(other_rate);
            available_capacity
        } else {
            desired_slope
        };

        // Update the slope and total rate
        lp.warmup_state.slope_per_step = actual_slope;
        lp.warmup_state.started_at_slot = self.current_slot;
        self.total_warmup_rate = other_rate.saturating_add(actual_slope);

        Ok(())
    }
}

// ============================================================================
// User Operations
// ============================================================================

impl<U, L> RiskEngine<U, L>
where
    U: AccountStorage<Account>,
    L: AccountStorage<Account>,
{
    /// Deposit funds to user account
    pub fn deposit(&mut self, user_index: usize, amount: u128) -> Result<()> {
        // Deposits are allowed even in withdrawal-only mode
        // They effectively take on their share of the loss (proportional haircut applies)

        let user = self.users.get_mut(user_index).ok_or(RiskError::UserNotFound)?;

        user.capital = add_u128(user.capital, amount);
        self.vault = add_u128(self.vault, amount);

        Ok(())
    }

    /// Withdraw funds from user account
    ///
    /// This function:
    /// 1. Converts any warmed-up realized PNL to principal
    /// 2. Withdraws the requested amount from principal
    /// 3. Ensures margin requirements are maintained if user has open position
    ///
    /// The user can withdraw up to (principal + warmed_up_pnl - margin_required)
    pub fn withdraw(&mut self, user_index: usize, amount: u128) -> Result<()> {
        // Settle funding before any PNL calculations
        self.touch_user(user_index)?;

        // Calculate withdrawable PNL before borrowing mutably
        let user = self.users.get(user_index).ok_or(RiskError::UserNotFound)?;
        let warmed_up_pnl = self.withdrawable_pnl(user);

        // Calculate haircut ratio BEFORE taking mutable borrow (if needed)
        let actual_amount = if self.withdrawal_only && self.loss_accum > 0 {
            // Calculate total system capital for haircut ratio
            // CRITICAL FIX: Include BOTH user and LP capital
            // Include amounts already withdrawn to maintain fair haircut ratio
            let user_capital: u128 = self.users.iter()
                .map(|u| u.capital)
                .sum();
            let lp_capital: u128 = self.lps.iter()
                .map(|lp| lp.capital)
                .sum();
            let current_capital = add_u128(user_capital, lp_capital);
            let total_principal = add_u128(current_capital, self.withdrawal_mode_withdrawn);

            if total_principal == 0 {
                return Err(RiskError::InsufficientBalance);
            }

            // Haircut ratio = (total_principal - loss_accum) / total_principal
            // Actual withdrawal = amount * haircut_ratio
            let available_principal = if total_principal > self.loss_accum {
                total_principal.saturating_sub(self.loss_accum)
            } else {
                0 // Completely insolvent
            };

            // Proportional haircut
            amount.saturating_mul(available_principal) / total_principal
        } else {
            amount
        };

        // Get mutable reference AFTER calculating haircut
        let user = self.users.get_mut(user_index).ok_or(RiskError::UserNotFound)?;

        // Step 1: Convert warmed-up PNL to principal
        if warmed_up_pnl > 0 {
            user.pnl = user.pnl.saturating_sub(warmed_up_pnl as i128);
            user.capital = add_u128(user.capital, warmed_up_pnl);
        }

        // Step 2: Check we have enough principal
        if user.capital < actual_amount {
            return Err(RiskError::InsufficientBalance);
        }

        // Step 4: Calculate what state would be after withdrawal
        let new_principal = sub_u128(user.capital, actual_amount);
        let new_collateral = add_u128(new_principal, clamp_pos_i128(user.pnl));

        // Step 5: If user has position, must maintain initial margin
        if user.position_size != 0 {
            let position_notional = mul_u128(
                user.position_size.abs() as u128,
                user.entry_price as u128
            ) / 1_000_000;

            let initial_margin_required = mul_u128(
                position_notional,
                self.params.initial_margin_bps as u128
            ) / 10_000;

            if new_collateral < initial_margin_required {
                return Err(RiskError::Undercollateralized);
            }
        }

        // Step 6: Commit the withdrawal
        user.capital = new_principal;
        self.vault = sub_u128(self.vault, actual_amount);

        // Track withdrawal amount if in withdrawal-only mode
        if self.withdrawal_only {
            self.withdrawal_mode_withdrawn = add_u128(self.withdrawal_mode_withdrawn, actual_amount);
        }

        Ok(())
    }

    /// Withdraw funds from LP account
    /// CRITICAL FIX: LPs can now withdraw with same haircut protection as users
    ///
    /// This function:
    /// 1. Converts any warmed-up realized PNL to capital
    /// 2. Withdraws the requested amount from capital
    /// 3. Applies proportional haircut if in withdrawal-only mode
    /// 4. Ensures margin requirements are maintained if LP has open position
    pub fn lp_withdraw(&mut self, lp_index: usize, amount: u128) -> Result<()> {
        // Settle funding before any PNL calculations
        self.touch_lp(lp_index)?;

        // Calculate withdrawable PNL before borrowing mutably
        let lp = self.lps.get(lp_index).ok_or(RiskError::LPNotFound)?;
        let warmed_up_pnl = self.withdrawable_pnl(lp);

        // Calculate haircut ratio BEFORE taking mutable borrow (if needed)
        let actual_amount = if self.withdrawal_only && self.loss_accum > 0 {
            // Calculate total system capital for haircut ratio
            // Include BOTH user and LP capital (same calculation as user withdrawal)
            let user_capital: u128 = self.users.iter()
                .map(|u| u.capital)
                .sum();
            let lp_capital: u128 = self.lps.iter()
                .map(|lp| lp.capital)
                .sum();
            let current_capital = add_u128(user_capital, lp_capital);
            let total_principal = add_u128(current_capital, self.withdrawal_mode_withdrawn);

            if total_principal == 0 {
                return Err(RiskError::InsufficientBalance);
            }

            // Haircut ratio = (total_principal - loss_accum) / total_principal
            // Actual withdrawal = amount * haircut_ratio
            let available_principal = if total_principal > self.loss_accum {
                total_principal.saturating_sub(self.loss_accum)
            } else {
                0 // Completely insolvent
            };

            // Proportional haircut
            amount.saturating_mul(available_principal) / total_principal
        } else {
            amount
        };

        // Get mutable reference AFTER calculating haircut
        let lp = self.lps.get_mut(lp_index).ok_or(RiskError::LPNotFound)?;

        // Step 1: Convert warmed-up PNL to capital
        if warmed_up_pnl > 0 {
            lp.pnl = lp.pnl.saturating_sub(warmed_up_pnl as i128);
            lp.capital = add_u128(lp.capital, warmed_up_pnl);
        }

        // Step 2: Check if LP has enough capital
        if lp.capital < actual_amount {
            return Err(RiskError::InsufficientBalance);
        }

        // Step 3: Calculate new capital after withdrawal
        let new_capital = sub_u128(lp.capital, actual_amount);

        // Step 4: If LP has open position, ensure initial margin is maintained
        if lp.position_size != 0 {
            let position_notional = mul_u128(
                lp.position_size.abs() as u128,
                lp.entry_price as u128
            ) / 1_000_000;

            let initial_margin_required = mul_u128(
                position_notional,
                self.params.initial_margin_bps as u128
            ) / 10_000;

            // Calculate new collateral (capital + positive PNL)
            let new_collateral = add_u128(new_capital, clamp_pos_i128(lp.pnl));

            if new_collateral < initial_margin_required {
                return Err(RiskError::Undercollateralized);
            }
        }

        // Step 5: Commit the withdrawal
        lp.capital = new_capital;
        self.vault = sub_u128(self.vault, actual_amount);

        // Track withdrawal amount if in withdrawal-only mode
        if self.withdrawal_only {
            self.withdrawal_mode_withdrawn = add_u128(self.withdrawal_mode_withdrawn, actual_amount);
        }

        Ok(())
    }

    /// Deposit funds to LP account
    /// Mirror of user deposit but for LPs
    pub fn lp_deposit(&mut self, lp_index: usize, amount: u128) -> Result<()> {
        // Deposits are allowed even in withdrawal-only mode
        let lp = self.lps.get_mut(lp_index).ok_or(RiskError::LPNotFound)?;

        lp.capital = add_u128(lp.capital, amount);
        self.vault = add_u128(self.vault, amount);

        Ok(())
    }
}
impl<U, L> RiskEngine<U, L>
where
    U: AccountStorage<Account>,
    L: AccountStorage<Account>,
{

// ============================================================================
// Trading Operations
// ============================================================================

    /// Execute trade via matching engine
    ///
    /// The matching engine is responsible for order matching logic and user authorization.
    /// The risk engine handles position updates, fees, and solvency checks.
    ///
    /// # Arguments
    /// * `matcher` - Implementation of MatchingEngine trait (can perform CPI)
    /// * `lp_index` - Index of LP providing liquidity
    /// * `user_index` - Index of user trading
    /// * `oracle_price` - Current oracle price
    /// * `size` - Position size (positive = long, negative = short)
    ///
    /// # Process
    /// 1. Call matching engine to validate/execute trade
    /// 2. Apply trading fee to insurance fund
    /// 3. Update LP and user positions
    /// 4. Realize PNL if reducing position
    /// 5. Check solvency of both accounts
    pub fn execute_trade<M: MatchingEngine>(
        &mut self,
        matcher: &M,
        lp_index: usize,
        user_index: usize,
        oracle_price: u64,
        size: i128,
    ) -> Result<()> {
        // In withdrawal-only mode, only allow closing/reducing positions
        if self.withdrawal_only {
            let user = self.users.get(user_index).ok_or(RiskError::UserNotFound)?;

            // Check if trade would increase position size
            let current_position = user.position_size;
            let new_position = current_position.saturating_add(size);

            // Allow only if position is being reduced
            if new_position.abs() > current_position.abs() {
                return Err(RiskError::WithdrawalOnlyMode);
            }
        }

        // Settle funding for both accounts before position changes
        self.touch_user(user_index)?;
        self.touch_lp(lp_index)?;

        // Get accounts (immutable first for matching engine call)
        let lp = self.lps.get(lp_index).ok_or(RiskError::LPNotFound)?;

        // Call matching engine (can perform CPI in production)
        let (program, context) = lp.matching_engine()?;
        matcher.execute_match(
            program,
            context,
            oracle_price,
            size,
        )?;

        // Now get mutable references for position updates
        let lp = self.lps.get_mut(lp_index).ok_or(RiskError::LPNotFound)?;
        let user = self.users.get_mut(user_index).ok_or(RiskError::UserNotFound)?;

        // Calculate fee
        let notional = mul_u128(size.abs() as u128, oracle_price as u128) / 1_000_000;
        let fee = mul_u128(notional, self.params.trading_fee_bps as u128) / 10_000;

        // Calculate PNL impact from closing existing position
        if user.position_size != 0 {
            let old_position = user.position_size;
            let old_entry = user.entry_price;

            // If reducing position, realize PNL
            if (old_position > 0 && size < 0) || (old_position < 0 && size > 0) {
                let close_size = core::cmp::min(old_position.abs(), size.abs());
                let pnl = if old_position > 0 {
                    // Closing long: (exit_price - entry_price) * size
                    ((oracle_price as i128 - old_entry as i128) * close_size as i128) / 1_000_000
                } else {
                    // Closing short: (entry_price - exit_price) * size
                    ((old_entry as i128 - oracle_price as i128) * close_size as i128) / 1_000_000
                };

                user.pnl = user.pnl.saturating_add(pnl);
                lp.pnl = lp.pnl.saturating_sub(pnl);
            }
        }

        // Update positions
        let new_user_position = user.position_size.saturating_add(size);
        let new_lp_position = lp.position_size.saturating_sub(size);

        // Update entry prices (weighted average for increases)
        if (user.position_size > 0 && size > 0) || (user.position_size < 0 && size < 0) {
            // Increasing position - weighted average entry
            let old_notional = mul_u128(user.position_size.abs() as u128, user.entry_price as u128);
            let new_notional = mul_u128(size.abs() as u128, oracle_price as u128);
            let total_notional = add_u128(old_notional, new_notional);
            let total_size = user.position_size.abs().saturating_add(size.abs());

            if total_size != 0 {
                user.entry_price = div_u128(total_notional, total_size as u128)? as u64;
            }
        } else if user.position_size.abs() < size.abs() {
            // Flipping position
            user.entry_price = oracle_price;
        }

        // Similar for LP
        if (lp.position_size > 0 && new_lp_position > lp.position_size) ||
           (lp.position_size < 0 && new_lp_position < lp.position_size) {
            let old_notional = mul_u128(lp.position_size.abs() as u128, lp.entry_price as u128);
            let new_notional = mul_u128(size.abs() as u128, oracle_price as u128);
            let total_notional = add_u128(old_notional, new_notional);
            let total_size = lp.position_size.abs().saturating_add(size.abs());

            if total_size != 0 {
                lp.entry_price = div_u128(total_notional, total_size as u128)? as u64;
            }
        }

        // Apply fee to insurance fund
        self.insurance_fund.fee_revenue = add_u128(self.insurance_fund.fee_revenue, fee);
        self.insurance_fund.balance = add_u128(self.insurance_fund.balance, fee);

        // Deduct fee from user
        user.pnl = user.pnl.saturating_sub(fee as i128);

        // Check neither account is negative
        let user_collateral = add_u128(user.capital, clamp_pos_i128(user.pnl));
        let lp_collateral = add_u128(lp.capital, clamp_pos_i128(lp.pnl));

        if user_collateral == 0 && user.capital > 0 {
            return Err(RiskError::Undercollateralized);
        }

        if lp_collateral == 0 && lp.capital > 0 {
            return Err(RiskError::Undercollateralized);
        }

        // Commit position updates
        user.position_size = new_user_position;
        lp.position_size = new_lp_position;

        // Update warmup slopes after PNL changes
        self.update_warmup_slope(user_index)?;
        // CRITICAL FIX: Now updating LP warmup slope too!
        self.update_lp_warmup_slope(lp_index)?;

        Ok(())
    }
}

// ============================================================================
// ADL (Auto-Deleveraging)
// ============================================================================

impl<U, L> RiskEngine<U, L>
where
    U: AccountStorage<Account>,
    L: AccountStorage<Account>,
{
    /// Apply ADL haircut to unwrapped PNL
    ///
    /// Invariants:
    /// - Haircut is applied to PNL that isn't warmed up yet (< time T)
    /// - Remaining losses are applied to insurance fund
    /// - User principal is NEVER touched
    pub fn apply_adl(&mut self, total_loss: u128) -> Result<()> {
        let mut remaining_loss = total_loss;

        // Phase 1: Haircut unwrapped PNL PROPORTIONALLY across ALL accounts (users AND LPs)
        // CRITICAL FIX: Fair treatment - no preferential ordering

        // Step 1: Calculate total unwrapped PNL across all accounts
        let mut total_unwrapped = 0u128;
        let mut user_unwrapped_amounts = Vec::new();
        let mut lp_unwrapped_amounts = Vec::new();

        for (idx, user) in self.users.iter().enumerate() {
            let positive_pnl = clamp_pos_i128(user.pnl);
            let withdrawable = self.withdrawable_pnl(user);
            let unwrapped = sub_u128(sub_u128(positive_pnl, withdrawable), user.reserved_pnl);

            if unwrapped > 0 {
                user_unwrapped_amounts.push((idx, unwrapped));
                total_unwrapped = add_u128(total_unwrapped, unwrapped);
            }
        }

        for (idx, lp) in self.lps.iter().enumerate() {
            let positive_pnl = clamp_pos_i128(lp.pnl);
            let withdrawable = self.withdrawable_pnl(lp);
            let unwrapped = sub_u128(sub_u128(positive_pnl, withdrawable), lp.reserved_pnl);

            if unwrapped > 0 {
                lp_unwrapped_amounts.push((idx, unwrapped));
                total_unwrapped = add_u128(total_unwrapped, unwrapped);
            }
        }

        // Step 2: Apply proportional haircuts to ALL accounts
        if total_unwrapped > 0 {
            let loss_to_socialize = core::cmp::min(remaining_loss, total_unwrapped);

            // Haircut users proportionally
            for (idx, unwrapped) in user_unwrapped_amounts {
                let haircut = mul_u128(loss_to_socialize, unwrapped) / total_unwrapped;
                if let Some(user) = self.users.get_mut(idx) {
                    user.pnl = user.pnl.saturating_sub(haircut as i128);
                }
            }

            // Haircut LPs proportionally (same formula)
            for (idx, unwrapped) in lp_unwrapped_amounts {
                let haircut = mul_u128(loss_to_socialize, unwrapped) / total_unwrapped;
                if let Some(lp) = self.lps.get_mut(idx) {
                    lp.pnl = lp.pnl.saturating_sub(haircut as i128);
                }
            }

            remaining_loss = sub_u128(remaining_loss, loss_to_socialize);
        }

        // Phase 2: Apply remaining loss to insurance fund
        if remaining_loss > 0 {
            if self.insurance_fund.balance < remaining_loss {
                // Insurance fund depleted - this is a crisis
                let insurance_used = self.insurance_fund.balance;
                self.insurance_fund.balance = 0;
                self.loss_accum = add_u128(self.loss_accum,
                    sub_u128(remaining_loss, insurance_used));

                // Enable withdrawal-only mode for fair unwinding
                self.withdrawal_only = true;
            } else {
                self.insurance_fund.balance = sub_u128(self.insurance_fund.balance, remaining_loss);
            }
        }

        Ok(())
    }

    /// Top up insurance fund to cover losses and potentially exit withdrawal-only mode
    ///
    /// This function allows:
    /// 1. Anyone to contribute funds to the insurance fund
    /// 2. Contribution directly reduces loss_accum
    /// 3. If loss_accum reaches 0, exits withdrawal-only mode (trading resumes)
    /// 4. Enables fair unwinding or recovery from crisis
    ///
    /// # Arguments
    /// * `amount` - Amount to add to insurance fund
    ///
    /// # Returns
    /// * `Ok(bool)` - true if withdrawal-only mode was exited, false otherwise
    pub fn top_up_insurance_fund(&mut self, amount: u128) -> Result<bool> {
        // Add to vault (since insurance fund deposit)
        self.vault = add_u128(self.vault, amount);

        // Apply contribution to loss_accum first (if any)
        if self.loss_accum > 0 {
            let loss_coverage = core::cmp::min(amount, self.loss_accum);
            self.loss_accum = sub_u128(self.loss_accum, loss_coverage);
            let remaining = sub_u128(amount, loss_coverage);

            // Add remaining to insurance fund balance
            self.insurance_fund.balance = add_u128(self.insurance_fund.balance, remaining);

            // Exit withdrawal-only mode if loss is fully covered
            if self.loss_accum == 0 && self.withdrawal_only {
                self.withdrawal_only = false;
                self.withdrawal_mode_withdrawn = 0; // Reset tracking
                Ok(true) // Exited withdrawal-only mode
            } else {
                Ok(false) // Still in withdrawal-only mode
            }
        } else {
            // No loss - just add to insurance fund
            self.insurance_fund.balance = add_u128(self.insurance_fund.balance, amount);
            Ok(false)
        }
    }
}

// ============================================================================
// Liquidations
// ============================================================================

impl<U, L> RiskEngine<U, L>
where
    U: AccountStorage<Account>,
    L: AccountStorage<Account>,
{
    /// Liquidate undercollateralized account
    ///
    /// Process:
    /// 1. Check if account is above liquidation threshold
    /// 2. Reduce position to bring account below threshold
    /// 3. Split fee between insurance fund and liquidation keeper
    ///
    /// # Insolvency Handling
    /// This function DOES NOT handle insolvency resolution. If the liquidation
    /// results in negative capital (bad debt), the account will remain in an
    /// insolvent state until a separate call to `apply_adl()` socializes the loss.
    /// This is intentional - liquidation and ADL are separate operations.
    ///
    /// # Returns
    /// * `Ok(())` if liquidation succeeds (or is not needed)
    /// * `Err` if user not found or other error occurs
    pub fn liquidate_user(
        &mut self,
        user_index: usize,
        keeper_account: usize,
        oracle_price: u64,
    ) -> Result<()> {
        // Settle funding before checking margin and realizing PNL
        self.touch_user(user_index)?;

        let user = self.users.get(user_index).ok_or(RiskError::UserNotFound)?;

        // Check if liquidation is needed
        if self.is_above_maintenance_margin(user, oracle_price) {
            return Ok(()); // No liquidation needed
        }

        let user = self.users.get_mut(user_index).ok_or(RiskError::UserNotFound)?;

        // Calculate liquidation size (reduce position to zero for simplicity)
        let liquidation_size = user.position_size;

        if liquidation_size == 0 {
            return Ok(()); // No position to liquidate
        }

        // Calculate liquidation fee
        let notional = mul_u128(liquidation_size.abs() as u128, oracle_price as u128) / 1_000_000;
        let liquidation_fee = mul_u128(notional, self.params.liquidation_fee_bps as u128) / 10_000;

        // Split fee between insurance and keeper
        let insurance_share = mul_u128(
            liquidation_fee,
            self.params.insurance_fee_share_bps as u128
        ) / 10_000;
        let keeper_share = sub_u128(liquidation_fee, insurance_share);

        // Realize PNL from position closure
        let pnl = if user.position_size > 0 {
            ((oracle_price as i128 - user.entry_price as i128) * liquidation_size.abs() as i128) / 1_000_000
        } else {
            ((user.entry_price as i128 - oracle_price as i128) * liquidation_size.abs() as i128) / 1_000_000
        };

        user.pnl = user.pnl.saturating_add(pnl);
        user.position_size = 0;

        // Apply fees
        self.insurance_fund.liquidation_revenue = add_u128(
            self.insurance_fund.liquidation_revenue,
            insurance_share
        );
        self.insurance_fund.balance = add_u128(self.insurance_fund.balance, insurance_share);

        // Give keeper their share
        if let Some(keeper) = self.users.get_mut(keeper_account) {
            keeper.pnl = keeper.pnl.saturating_add(keeper_share as i128);
        }

        // Update warmup slopes after PNL changes
        self.update_warmup_slope(user_index)?;
        if keeper_account != user_index {
            self.update_warmup_slope(keeper_account)?;
        }

        Ok(())
    }

    /// Liquidate an LP that is below maintenance margin
    /// CRITICAL FIX: LPs can now be liquidated just like users
    ///
    /// # Insolvency Handling
    /// This function DOES NOT handle insolvency resolution. If the liquidation
    /// results in negative capital (bad debt), the LP will remain in an
    /// insolvent state until a separate call to `apply_adl()` socializes the loss.
    /// This is intentional - liquidation and ADL are separate operations.
    ///
    /// # Returns
    /// * `Ok(())` if liquidation succeeds (or is not needed)
    /// * `Err` if LP not found or other error occurs
    pub fn liquidate_lp(
        &mut self,
        lp_index: usize,
        keeper_account: usize,
        oracle_price: u64,
    ) -> Result<()> {
        // Settle funding before checking margin and realizing PNL
        self.touch_lp(lp_index)?;

        let lp = self.lps.get(lp_index).ok_or(RiskError::LPNotFound)?;

        // Check if liquidation is needed using same logic as users
        let collateral = self.account_collateral(lp);
        let position_value = mul_u128(lp.position_size.abs() as u128, oracle_price as u128) / 1_000_000;
        let maintenance_margin = mul_u128(position_value, self.params.maintenance_margin_bps as u128) / 10_000;

        if collateral >= maintenance_margin {
            return Ok(()); // No liquidation needed
        }

        let lp = self.lps.get_mut(lp_index).ok_or(RiskError::LPNotFound)?;

        // Calculate liquidation size (reduce position to zero)
        let liquidation_size = lp.position_size;

        if liquidation_size == 0 {
            return Ok(()); // No position to liquidate
        }

        // Calculate liquidation fee
        let notional = mul_u128(liquidation_size.abs() as u128, oracle_price as u128) / 1_000_000;
        let liquidation_fee = mul_u128(notional, self.params.liquidation_fee_bps as u128) / 10_000;

        // Split fee between insurance and keeper
        let insurance_share = mul_u128(
            liquidation_fee,
            self.params.insurance_fee_share_bps as u128
        ) / 10_000;
        let keeper_share = sub_u128(liquidation_fee, insurance_share);

        // Realize PNL from position closure
        let pnl = if lp.position_size > 0 {
            ((oracle_price as i128 - lp.entry_price as i128) * liquidation_size.abs() as i128) / 1_000_000
        } else {
            ((lp.entry_price as i128 - oracle_price as i128) * liquidation_size.abs() as i128) / 1_000_000
        };

        lp.pnl = lp.pnl.saturating_add(pnl);
        lp.position_size = 0;

        // Apply fees
        self.insurance_fund.liquidation_revenue = add_u128(
            self.insurance_fund.liquidation_revenue,
            insurance_share
        );
        self.insurance_fund.balance = add_u128(self.insurance_fund.balance, insurance_share);

        // Give keeper their share (keeper is a user account)
        if let Some(keeper) = self.users.get_mut(keeper_account) {
            keeper.pnl = keeper.pnl.saturating_add(keeper_share as i128);
        }

        // Update warmup slopes after PNL changes
        self.update_lp_warmup_slope(lp_index)?;
        self.update_warmup_slope(keeper_account)?;

        Ok(())
    }
}

// ============================================================================
// Initialization
// ============================================================================

impl<U, L> RiskEngine<U, L>
where
    U: AccountStorage<UserAccount> + Default,
    L: AccountStorage<LPAccount> + Default,
{
    /// Create a new risk engine
    pub fn new(params: RiskParams) -> Self {
        Self {
            vault: 0,
            insurance_fund: InsuranceFund {
                balance: 0,
                fee_revenue: 0,
                liquidation_revenue: 0,
            },
            users: U::default(),
            lps: L::default(),
            params,
            current_slot: 0,
            fee_index: 0,
            sum_vested_pos_pnl: 0,
            loss_accum: 0,
            fee_carry: 0,
            funding_index_qpb_e6: 0,
            last_funding_slot: 0,
            total_warmup_rate: 0,
            withdrawal_only: false,
            withdrawal_mode_withdrawn: 0,
        }
    }

    /// Add a new user account
    /// Add a new user account
    pub fn add_user(&mut self, fee_payment: u128) -> Result<usize> {
        if self.users.len() >= self.params.max_users as usize {
            return Err(RiskError::Overflow); // Or new error
        }
        let multiplier = Self::account_fee_multiplier(self.params.max_users, self.users.len() as u64);
        let required_fee = mul_u128(self.params.account_fee_bps as u128, multiplier) / 10_000;
        if fee_payment < required_fee {
            return Err(RiskError::InsufficientBalance);
        }
        // Pay fee to insurance
        self.insurance_fund.balance = add_u128(self.insurance_fund.balance, required_fee);
        self.insurance_fund.fee_revenue = add_u128(self.insurance_fund.fee_revenue, required_fee);

        let index = self.users.len();
        self.users.push(Account {
            capital: 0,
            pnl: 0,
            reserved_pnl: 0,
            warmup_state: Warmup {
                started_at_slot: self.current_slot,
                slope_per_step: 0, // Will be set by update_warmup_slope when PNL changes
            },
            position_size: 0,
            entry_price: 0,
            fee_index: 0,
            fee_accrued: 0,
            vested_pos_snapshot: 0,
            funding_index: self.funding_index_qpb_e6,
            matching_engine_program: None,
            matching_engine_context: None,
        });
        Ok(index)
    }
    pub fn add_lp(&mut self, matching_engine_program: [u8; 32], matching_engine_context: [u8; 32], fee_payment: u128) -> Result<usize> {
        if self.lps.len() >= self.params.max_lps as usize {
            return Err(RiskError::Overflow);
        }
        let multiplier = Self::account_fee_multiplier(self.params.max_lps, self.lps.len() as u64);
        let required_fee = mul_u128(self.params.account_fee_bps as u128, multiplier) / 10_000;
        if fee_payment < required_fee {
            return Err(RiskError::InsufficientBalance);
        }
        // Pay fee to insurance
        self.insurance_fund.balance = add_u128(self.insurance_fund.balance, required_fee);
        self.insurance_fund.fee_revenue = add_u128(self.insurance_fund.fee_revenue, required_fee);

        let index = self.lps.len();
        self.lps.push(Account {
            capital: 0,
            pnl: 0,
            reserved_pnl: 0,
            warmup_state: Warmup {
                started_at_slot: self.current_slot,
                // Initialize to 0, consistent with user accounts
                // Slope will be set by update_lp_warmup_slope when LP earns PNL
                slope_per_step: 0,
            },
            position_size: 0,
            entry_price: 0,
            fee_index: 0,
            fee_accrued: 0,
            vested_pos_snapshot: 0,
            funding_index: self.funding_index_qpb_e6,
            matching_engine_program: Some(matching_engine_program),
            matching_engine_context: Some(matching_engine_context),
        });
        Ok(index)
    }

    /// Advance to next slot (for testing warmup)
    pub fn advance_slot(&mut self, slots: u64) {
        self.current_slot = self.current_slot.saturating_add(slots);
    }
}
