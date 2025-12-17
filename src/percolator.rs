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

// ============================================================================
// Constants
// ============================================================================

// Use smaller array size for Kani verification to make proofs tractable
// Production uses 4096, but Kani symbolic execution becomes intractable at that scale
#[cfg(kani)]
pub const MAX_ACCOUNTS: usize = 8;  // Small for fast formal verification

#[cfg(not(kani))]
pub const MAX_ACCOUNTS: usize = 4096;

// Ceiling division ensures at least 1 word even when MAX_ACCOUNTS < 64
pub const BITMAP_WORDS: usize = (MAX_ACCOUNTS + 63) / 64;

// ============================================================================
// Core Data Structures
// ============================================================================

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccountKind {
    User = 0,
    LP = 1,
}

/// Unified account - can be user or LP
///
/// LPs are distinguished by having kind = LP and matcher_program/context set.
/// Users have kind = User and matcher arrays zeroed.
///
/// This unification ensures LPs receive the same risk management protections as users:
/// - PNL warmup
/// - ADL (Auto-Deleveraging)
/// - Liquidations
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Account {
    pub kind: AccountKind,

    /// Unique account ID (monotonically increasing, never recycled)
    pub account_id: u64,

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

    // ========================================
    // Warmup (embedded, no separate struct)
    // ========================================

    /// Slot when warmup started
    pub warmup_started_at_slot: u64,

    /// Linear vesting rate per slot
    pub warmup_slope_per_step: u128,

    // ========================================
    // Position (universal)
    // ========================================

    /// Current position size (+ long, - short)
    pub position_size: i128,

    /// Average entry price for position
    pub entry_price: u64,

    // ========================================
    // Funding (universal)
    // ========================================

    /// Funding index snapshot (quote per base, 1e6 scale)
    pub funding_index: i128,

    // ========================================
    // LP-specific (only meaningful for LP kind)
    // ========================================

    /// Matching engine program ID (zero for user accounts)
    pub matcher_program: [u8; 32],

    /// Matching engine context account (zero for user accounts)
    pub matcher_context: [u8; 32],
}

impl Account {
    /// Check if this account is an LP
    pub fn is_lp(&self) -> bool {
        self.kind == AccountKind::LP
    }

    /// Check if this account is a regular user
    pub fn is_user(&self) -> bool {
        self.kind == AccountKind::User
    }
}

/// Helper to create empty account
fn empty_account() -> Account {
    Account {
        kind: AccountKind::User,
        account_id: 0,
        capital: 0,
        pnl: 0,
        reserved_pnl: 0,
        warmup_started_at_slot: 0,
        warmup_slope_per_step: 0,
        position_size: 0,
        entry_price: 0,
        funding_index: 0,
        matcher_program: [0; 32],
        matcher_context: [0; 32],
    }
}

/// Insurance fund state
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InsuranceFund {
    /// Insurance fund balance
    pub balance: u128,

    /// Accumulated fees from trades
    pub fee_revenue: u128,
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

    /// Maximum number of accounts
    pub max_accounts: u64,

    /// Base account creation fee in basis points (e.g., 10000 = 1%)
    /// Actual fee = (account_fee_bps * capacity_multiplier) / 10000
    /// The multiplier increases as the system approaches max capacity
    pub account_fee_bps: u64,

    /// Insurance fund threshold for entering risk-reduction-only mode
    /// If insurance fund balance drops below this, risk-reduction mode activates
    pub risk_reduction_threshold: u128,
}

/// Main risk engine state - fixed slab with bitmap
#[repr(C)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RiskEngine {
    /// Total vault balance (all deposited funds)
    pub vault: u128,

    /// Insurance fund
    pub insurance_fund: InsuranceFund,

    /// Risk parameters
    pub params: RiskParams,

    /// Current slot (for warmup calculations)
    pub current_slot: u64,

    /// Global funding index (quote per 1 base, scaled by 1e6)
    pub funding_index_qpb_e6: i128,

    /// Last slot when funding was accrued
    pub last_funding_slot: u64,

    /// Loss accumulator for socialization
    pub loss_accum: u128,

    /// Risk-reduction-only mode is entered when the system is in deficit. Warmups are frozen so pending PnL cannot become principal. Withdrawals of principal (capital) are allowed (subject to margin). Risk-increasing actions are blocked; only risk-reducing/neutral operations are allowed.
    pub risk_reduction_only: bool,

    /// Total amount withdrawn during risk-reduction-only mode
    /// Used to maintain fair haircut ratio during unwinding
    pub risk_reduction_mode_withdrawn: u128,

    /// Warmup pause flag
    pub warmup_paused: bool,

    /// Slot when warmup was paused
    pub warmup_pause_slot: u64,

    // ========================================
    // Slab Management
    // ========================================

    /// Occupancy bitmap (4096 bits = 64 u64 words)
    pub used: [u64; BITMAP_WORDS],

    /// Number of used accounts (O(1) counter, fixes H2: fee bypass TOCTOU)
    pub num_used_accounts: u16,

    /// Next account ID to assign (monotonically increasing, never recycled)
    pub next_account_id: u64,

    /// Freelist head (u16::MAX = none)
    pub free_head: u16,

    /// Freelist next pointers
    pub next_free: [u16; MAX_ACCOUNTS],

    /// Account slab (4096 accounts)
    pub accounts: [Account; MAX_ACCOUNTS],
}

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

    /// Account not found
    AccountNotFound,

    /// Account is not an LP account
    NotAnLPAccount,

    /// Position size mismatch
    PositionSizeMismatch,

    /// System in withdrawal-only mode (deposits and trading blocked)
    RiskReductionOnlyMode,

    /// Account kind mismatch
    AccountKindMismatch,
}

pub type Result<T> = core::result::Result<T, RiskError>;

/// Operation classification for risk-reduction-only mode gating
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpClass {
    RiskIncrease,
    RiskNeutral,
    RiskReduce,
}

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

/// Saturating absolute value for i128 (handles i128::MIN without overflow)
#[inline]
fn saturating_abs_i128(val: i128) -> i128 {
    if val == i128::MIN {
        i128::MAX
    } else {
        val.abs()
    }
}

// ============================================================================
// Matching Engine Trait
// ============================================================================

/// Result of a successful trade execution from the matching engine
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TradeExecution {
    /// Actual execution price (may differ from oracle/requested price)
    pub price: u64,
    /// Actual executed size (may be partial fill)
    pub size: i128,
}

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
    /// * `lp_account_id` - Unique ID of the LP account (never recycled)
    /// * `oracle_price` - Current oracle price for reference
    /// * `size` - Requested position size (positive = long, negative = short)
    ///
    /// # Returns
    /// * `Ok(TradeExecution)` with actual executed price and size
    /// * `Err(RiskError)` if the trade is rejected
    ///
    /// # Safety
    /// The matching engine MUST verify user authorization before approving trades.
    /// The risk engine will check solvency after the trade executes.
    fn execute_match(
        &self,
        lp_program: &[u8; 32],
        lp_context: &[u8; 32],
        lp_account_id: u64,
        oracle_price: u64,
        size: i128,
    ) -> Result<TradeExecution>;
}

/// No-op matching engine (for testing)
/// Returns the requested price and size as-is
pub struct NoOpMatcher;

impl MatchingEngine for NoOpMatcher {
    fn execute_match(
        &self,
        _lp_program: &[u8; 32],
        _lp_context: &[u8; 32],
        _lp_account_id: u64,
        oracle_price: u64,
        size: i128,
    ) -> Result<TradeExecution> {
        // Return requested price/size unchanged (no actual matching logic)
        Ok(TradeExecution {
            price: oracle_price,
            size,
        })
    }
}

// ============================================================================
// Core Implementation
// ============================================================================

impl RiskEngine {
    /// Create a new risk engine
    pub fn new(params: RiskParams) -> Self {
        let mut engine = Self {
            vault: 0,
            insurance_fund: InsuranceFund {
                balance: 0,
                fee_revenue: 0,
            },
            params,
            current_slot: 0,
            funding_index_qpb_e6: 0,
            last_funding_slot: 0,
            loss_accum: 0,
            risk_reduction_only: false,
            risk_reduction_mode_withdrawn: 0,
            warmup_paused: false,
            warmup_pause_slot: 0,
            used: [0; BITMAP_WORDS],
            num_used_accounts: 0,
            next_account_id: 0,
            free_head: 0,
            next_free: [0; MAX_ACCOUNTS],
            accounts: [empty_account(); MAX_ACCOUNTS],
        };

        // Initialize freelist: 0 -> 1 -> 2 -> ... -> 4095 -> NONE
        for i in 0..MAX_ACCOUNTS - 1 {
            engine.next_free[i] = (i + 1) as u16;
        }
        engine.next_free[MAX_ACCOUNTS - 1] = u16::MAX; // Sentinel

        engine
    }

    // ========================================
    // Bitmap Helpers
    // ========================================

    fn is_used(&self, idx: usize) -> bool {
        let w = idx >> 6;
        let b = idx & 63;
        ((self.used[w] >> b) & 1) == 1
    }

    fn set_used(&mut self, idx: usize) {
        let w = idx >> 6;
        let b = idx & 63;
        self.used[w] |= 1u64 << b;
    }

    fn clear_used(&mut self, idx: usize) {
        let w = idx >> 6;
        let b = idx & 63;
        self.used[w] &= !(1u64 << b);
    }

    fn for_each_used_mut<F: FnMut(usize, &mut Account)>(&mut self, mut f: F) {
        for (block, word) in self.used.iter().copied().enumerate() {
            let mut w = word;
            while w != 0 {
                let bit = w.trailing_zeros() as usize;
                let idx = block * 64 + bit;
                w &= w - 1; // Clear lowest bit
                f(idx, &mut self.accounts[idx]);
            }
        }
    }

    fn for_each_used<F: FnMut(usize, &Account)>(&self, mut f: F) {
        for (block, word) in self.used.iter().copied().enumerate() {
            let mut w = word;
            while w != 0 {
                let bit = w.trailing_zeros() as usize;
                let idx = block * 64 + bit;
                w &= w - 1; // Clear lowest bit
                f(idx, &self.accounts[idx]);
            }
        }
    }

    // ========================================
    // Account Allocation
    // ========================================

    fn alloc_slot(&mut self) -> Result<u16> {
        if self.free_head == u16::MAX {
            return Err(RiskError::Overflow); // Slab full
        }
        let idx = self.free_head;
        self.free_head = self.next_free[idx as usize];
        self.set_used(idx as usize);
        // Increment O(1) counter atomically (fixes H2: TOCTOU fee bypass)
        self.num_used_accounts = self.num_used_accounts.saturating_add(1);
        Ok(idx)
    }

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

    /// Count used accounts
    fn count_used(&self) -> u64 {
        let mut count = 0u64;
        self.for_each_used(|_, _| {
            count += 1;
        });
        count
    }

    // ========================================
    // Risk-Reduction-Only Mode Helpers
    // ========================================

    /// Central gate for operation enforcement in risk-reduction-only mode
    #[inline]
    fn enforce_op(&self, op: OpClass) -> Result<()> {
        if !self.risk_reduction_only {
            return Ok(());
        }
        match op {
            OpClass::RiskIncrease => Err(RiskError::RiskReductionOnlyMode),
            OpClass::RiskNeutral | OpClass::RiskReduce => Ok(()),
        }
    }

    /// Enter risk-reduction-only mode and freeze warmups
    pub fn enter_risk_reduction_only_mode(&mut self) {
        self.risk_reduction_only = true;
        if !self.warmup_paused {
            self.warmup_paused = true;
            self.warmup_pause_slot = self.current_slot;
        }
    }

    /// Exit risk-reduction-only mode if system is safe (loss fully covered AND above threshold)
    pub fn exit_risk_reduction_only_mode_if_safe(&mut self) {
        if self.loss_accum == 0 {
            // Check if insurance fund is back above configured threshold
            if self.insurance_fund.balance >= self.params.risk_reduction_threshold {
                self.risk_reduction_only = false;
                self.risk_reduction_mode_withdrawn = 0;
                self.warmup_paused = false;
            }
        }
    }

    // ========================================
    // Account Management
    // ========================================

    /// Add a new user account
    pub fn add_user(&mut self, fee_payment: u128) -> Result<u16> {
        // Use O(1) counter instead of O(N) count_used() (fixes H2: TOCTOU fee bypass)
        let used_count = self.num_used_accounts as u64;
        if used_count >= self.params.max_accounts {
            return Err(RiskError::Overflow);
        }

        let multiplier = Self::account_fee_multiplier(self.params.max_accounts, used_count);
        let required_fee = mul_u128(self.params.account_fee_bps as u128, multiplier) / 10_000;
        if fee_payment < required_fee {
            return Err(RiskError::InsufficientBalance);
        }

        // Pay fee to insurance (fee tokens are deposited into vault)
        self.vault = add_u128(self.vault, required_fee);
        self.insurance_fund.balance = add_u128(self.insurance_fund.balance, required_fee);
        self.insurance_fund.fee_revenue = add_u128(self.insurance_fund.fee_revenue, required_fee);

        // Allocate slot and assign unique ID
        let idx = self.alloc_slot()?;
        let account_id = self.next_account_id;
        self.next_account_id = self.next_account_id.saturating_add(1);

        // Initialize account
        self.accounts[idx as usize] = Account {
            kind: AccountKind::User,
            account_id,
            capital: 0,
            pnl: 0,
            reserved_pnl: 0,
            warmup_started_at_slot: self.current_slot,
            warmup_slope_per_step: 0,
            position_size: 0,
            entry_price: 0,
            funding_index: self.funding_index_qpb_e6,
            matcher_program: [0; 32],
            matcher_context: [0; 32],
        };

        Ok(idx)
    }

    /// Add a new LP account
    pub fn add_lp(
        &mut self,
        matching_engine_program: [u8; 32],
        matching_engine_context: [u8; 32],
        fee_payment: u128,
    ) -> Result<u16> {
        // Use O(1) counter instead of O(N) count_used() (fixes H2: TOCTOU fee bypass)
        let used_count = self.num_used_accounts as u64;
        if used_count >= self.params.max_accounts {
            return Err(RiskError::Overflow);
        }

        let multiplier = Self::account_fee_multiplier(self.params.max_accounts, used_count);
        let required_fee = mul_u128(self.params.account_fee_bps as u128, multiplier) / 10_000;
        if fee_payment < required_fee {
            return Err(RiskError::InsufficientBalance);
        }

        // Pay fee to insurance (fee tokens are deposited into vault)
        self.vault = add_u128(self.vault, required_fee);
        self.insurance_fund.balance = add_u128(self.insurance_fund.balance, required_fee);
        self.insurance_fund.fee_revenue = add_u128(self.insurance_fund.fee_revenue, required_fee);

        // Allocate slot and assign unique ID
        let idx = self.alloc_slot()?;
        let account_id = self.next_account_id;
        self.next_account_id = self.next_account_id.saturating_add(1);

        // Initialize account
        self.accounts[idx as usize] = Account {
            kind: AccountKind::LP,
            account_id,
            capital: 0,
            pnl: 0,
            reserved_pnl: 0,
            warmup_started_at_slot: self.current_slot,
            warmup_slope_per_step: 0,
            position_size: 0,
            entry_price: 0,
            funding_index: self.funding_index_qpb_e6,
            matcher_program: matching_engine_program,
            matcher_context: matching_engine_context,
        };

        Ok(idx)
    }

    // ========================================
    // Warmup
    // ========================================

    /// Calculate withdrawable PNL for an account after warmup
    pub fn withdrawable_pnl(&self, account: &Account) -> u128 {
        // Only positive PNL can be withdrawn
        let positive_pnl = clamp_pos_i128(account.pnl);

        // Available = positive PNL - reserved
        let available_pnl = sub_u128(positive_pnl, account.reserved_pnl);

        // Apply warmup pause - when paused, warmup cannot progress beyond pause_slot
        let effective_slot = if self.warmup_paused {
            core::cmp::min(self.current_slot, self.warmup_pause_slot)
        } else {
            self.current_slot
        };

        // Calculate elapsed slots
        let elapsed_slots = effective_slot.saturating_sub(account.warmup_started_at_slot);

        // Calculate warmed up cap: slope * elapsed_slots
        let warmed_up_cap = mul_u128(
            account.warmup_slope_per_step,
            elapsed_slots as u128
        );

        // Return minimum of available and warmed up
        core::cmp::min(available_pnl, warmed_up_cap)
    }

    /// Update warmup slope for an account
    /// NOTE: No warmup rate cap (removed for simplicity)
    pub fn update_warmup_slope(&mut self, idx: u16) -> Result<()> {
        if !self.is_used(idx as usize) {
            return Err(RiskError::AccountNotFound);
        }

        let account = &mut self.accounts[idx as usize];

        // Calculate positive PNL that needs to warm up
        let positive_pnl = clamp_pos_i128(account.pnl);

        // Calculate slope: pnl / warmup_period
        let slope = if self.params.warmup_period_slots > 0 {
            positive_pnl / (self.params.warmup_period_slots as u128)
        } else {
            positive_pnl // Instant warmup if period is 0
        };

        // Update slope
        account.warmup_slope_per_step = slope;

        // Don't update started_at_slot if warmup is paused
        if !self.warmup_paused {
            account.warmup_started_at_slot = self.current_slot;
        }

        Ok(())
    }

    // ========================================
    // Funding
    // ========================================

    /// Accrue funding globally in O(1)
    pub fn accrue_funding(
        &mut self,
        now_slot: u64,
        oracle_price: u64,
        funding_rate_bps_per_slot: i64,
    ) -> Result<()> {
        // Funding accrual is risk-neutral (allowed in risk mode)
        self.enforce_op(OpClass::RiskNeutral)?;

        let dt = now_slot.saturating_sub(self.last_funding_slot);
        if dt == 0 {
            return Ok(());
        }

        // Input validation to prevent overflow
        if oracle_price == 0 || oracle_price > 1_000_000_000_000 {
            return Err(RiskError::Overflow);
        }

        // Cap funding rate at 10000 bps (100%) per slot as sanity bound
        // Real-world funding rates should be much smaller (typically < 1 bps/slot)
        if funding_rate_bps_per_slot.abs() > 10_000 {
            return Err(RiskError::Overflow);
        }

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

    /// Settle funding for an account (lazy update)
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

    /// Touch an account (settle funding before operations)
    pub fn touch_account(&mut self, idx: u16) -> Result<()> {
        // Funding settlement is risk-neutral (allowed in risk mode)
        self.enforce_op(OpClass::RiskNeutral)?;

        if !self.is_used(idx as usize) {
            return Err(RiskError::AccountNotFound);
        }

        let account = &mut self.accounts[idx as usize];
        Self::settle_account_funding(account, self.funding_index_qpb_e6)
    }

    // ========================================
    // Deposits and Withdrawals
    // ========================================

    /// Deposit funds to account
    pub fn deposit(&mut self, idx: u16, amount: u128) -> Result<()> {
        // Deposits reduce risk (allowed in risk mode)
        self.enforce_op(OpClass::RiskReduce)?;

        if !self.is_used(idx as usize) {
            return Err(RiskError::AccountNotFound);
        }

        let account = &mut self.accounts[idx as usize];
        account.capital = add_u128(account.capital, amount);
        self.vault = add_u128(self.vault, amount);

        Ok(())
    }

    /// Risk-reduction-only mode is entered when the system is in deficit. Warmups are frozen so pending PNL cannot become principal. Withdrawals of principal (capital) are allowed (subject to margin). Risk-increasing actions are blocked; only risk-reducing/neutral operations are allowed.
    pub fn withdraw(&mut self, idx: u16, amount: u128) -> Result<()> {
        // Withdrawals are neutral in risk mode (allowed)
        self.enforce_op(OpClass::RiskNeutral)?;

        // Settle funding before any PNL calculations
        self.touch_account(idx)?;

        if !self.is_used(idx as usize) {
            return Err(RiskError::AccountNotFound);
        }

        // Calculate withdrawable PNL
        let account = &self.accounts[idx as usize];
        let warmed_up_pnl = self.withdrawable_pnl(account);

        // Get mutable reference
        let account = &mut self.accounts[idx as usize];

        // Step 1: Convert warmed-up PNL to capital
        if warmed_up_pnl > 0 {
            account.pnl = account.pnl.saturating_sub(warmed_up_pnl as i128);
            account.capital = add_u128(account.capital, warmed_up_pnl);
        }

        // Step 2: Check we have enough capital
        if account.capital < amount {
            return Err(RiskError::InsufficientBalance);
        }

        // Step 3: Calculate new state after withdrawal
        let new_capital = sub_u128(account.capital, amount);
        let new_collateral = add_u128(new_capital, clamp_pos_i128(account.pnl));

        // Step 4: If account has position, must maintain initial margin
        if account.position_size != 0 {
            let position_notional = mul_u128(
                saturating_abs_i128(account.position_size) as u128,
                account.entry_price as u128
            ) / 1_000_000;

            let initial_margin_required = mul_u128(
                position_notional,
                self.params.initial_margin_bps as u128
            ) / 10_000;

            if new_collateral < initial_margin_required {
                return Err(RiskError::Undercollateralized);
            }
        }

        // Step 5: Commit the withdrawal
        account.capital = new_capital;
        self.vault = sub_u128(self.vault, amount);

        Ok(())
    }

    // ========================================
    // Trading
    // ========================================

    /// Calculate account's collateral (capital + positive PNL)
    pub fn account_collateral(&self, account: &Account) -> u128 {
        add_u128(account.capital, clamp_pos_i128(account.pnl))
    }

    /// Check if account is above maintenance margin
    pub fn is_above_maintenance_margin(&self, account: &Account, oracle_price: u64) -> bool {
        let collateral = self.account_collateral(account);

        // Calculate position value at current price
        let position_value = mul_u128(
            saturating_abs_i128(account.position_size) as u128,
            oracle_price as u128
        ) / 1_000_000;

        // Maintenance margin requirement
        let margin_required = mul_u128(
            position_value,
            self.params.maintenance_margin_bps as u128
        ) / 10_000;

        collateral > margin_required
    }

    /// Risk-reduction-only mode is entered when the system is in deficit. Warmups are frozen so pending PNL cannot become principal. Withdrawals of principal (capital) are allowed (subject to margin). Risk-increasing actions are blocked; only risk-reducing/neutral operations are allowed.
    pub fn execute_trade<M: MatchingEngine>(
        &mut self,
        matcher: &M,
        lp_idx: u16,
        user_idx: u16,
        oracle_price: u64,
        size: i128,
    ) -> Result<()> {
        // Validate indices
        if !self.is_used(lp_idx as usize) || !self.is_used(user_idx as usize) {
            return Err(RiskError::AccountNotFound);
        }

        // Check if trade increases risk (absolute exposure for either party)
        let old_user_pos = self.accounts[user_idx as usize].position_size;
        let old_lp_pos = self.accounts[lp_idx as usize].position_size;
        let new_user_pos = old_user_pos.saturating_add(size);
        let new_lp_pos = old_lp_pos.saturating_sub(size);

        let user_inc = saturating_abs_i128(new_user_pos) > saturating_abs_i128(old_user_pos);
        let lp_inc = saturating_abs_i128(new_lp_pos) > saturating_abs_i128(old_lp_pos);

        if user_inc || lp_inc {
            self.enforce_op(OpClass::RiskIncrease)?; // Blocked in risk mode
        } else {
            self.enforce_op(OpClass::RiskReduce)?;   // Allowed in risk mode
        }

        // Settle funding for both accounts
        self.touch_account(user_idx)?;
        self.touch_account(lp_idx)?;

        // Validate account kinds
        if self.accounts[lp_idx as usize].kind != AccountKind::LP {
            return Err(RiskError::AccountKindMismatch);
        }
        if self.accounts[user_idx as usize].kind != AccountKind::User {
            return Err(RiskError::AccountKindMismatch);
        }

        // Call matching engine with LP account ID
        let lp = &self.accounts[lp_idx as usize];
        let execution = matcher.execute_match(
            &lp.matcher_program,
            &lp.matcher_context,
            lp.account_id,
            oracle_price,
            size,
        )?;

        // Use executed price and size from matching engine
        let exec_price = execution.price;
        let exec_size = execution.size;

        // Calculate fee based on actual execution
        let notional = mul_u128(saturating_abs_i128(exec_size) as u128, exec_price as u128) / 1_000_000;
        let fee = mul_u128(notional, self.params.trading_fee_bps as u128) / 10_000;

        // Use split_at_mut to access both accounts without copying
        let (user, lp) = if user_idx < lp_idx {
            let (left, right) = self.accounts.split_at_mut(lp_idx as usize);
            (&mut left[user_idx as usize], &mut right[0])
        } else {
            let (left, right) = self.accounts.split_at_mut(user_idx as usize);
            (&mut right[0], &mut left[lp_idx as usize])
        };

        // Calculate PNL impact from closing existing position
        let mut user_pnl_delta = 0i128;
        let mut lp_pnl_delta = 0i128;

        if user.position_size != 0 {
            let old_position = user.position_size;
            let old_entry = user.entry_price;

            // If reducing position, realize PNL at execution price
            if (old_position > 0 && exec_size < 0) || (old_position < 0 && exec_size > 0) {
                let close_size = core::cmp::min(saturating_abs_i128(old_position), saturating_abs_i128(exec_size));
                let price_diff = if old_position > 0 {
                    // Closing long: (exit_price - entry_price)
                    (exec_price as i128).saturating_sub(old_entry as i128)
                } else {
                    // Closing short: (entry_price - exit_price)
                    (old_entry as i128).saturating_sub(exec_price as i128)
                };

                let pnl = price_diff
                    .checked_mul(close_size)
                    .ok_or(RiskError::Overflow)?
                    .checked_div(1_000_000)
                    .ok_or(RiskError::Overflow)?;

                user_pnl_delta = pnl;
                lp_pnl_delta = -pnl;
            }
        }

        // Calculate new positions using executed size
        let new_user_position = user.position_size.saturating_add(exec_size);
        let new_lp_position = lp.position_size.saturating_sub(exec_size);

        // Calculate new entry prices using execution price
        let mut new_user_entry = user.entry_price;
        let mut new_lp_entry = lp.entry_price;

        // Update user entry price
        if (user.position_size > 0 && exec_size > 0) || (user.position_size < 0 && exec_size < 0) {
            // Increasing position - weighted average entry at execution price
            let old_notional = mul_u128(saturating_abs_i128(user.position_size) as u128, user.entry_price as u128);
            let new_notional = mul_u128(saturating_abs_i128(exec_size) as u128, exec_price as u128);
            let total_notional = add_u128(old_notional, new_notional);
            let total_size = saturating_abs_i128(user.position_size).saturating_add(saturating_abs_i128(exec_size));

            if total_size != 0 {
                new_user_entry = div_u128(total_notional, total_size as u128)? as u64;
            }
        } else if saturating_abs_i128(user.position_size) < saturating_abs_i128(exec_size) {
            // Flipping position - new entry at execution price
            new_user_entry = exec_price;
        }

        // Update LP entry price
        if (lp.position_size > 0 && new_lp_position > lp.position_size) ||
           (lp.position_size < 0 && new_lp_position < lp.position_size) {
            let old_notional = mul_u128(saturating_abs_i128(lp.position_size) as u128, lp.entry_price as u128);
            let new_notional = mul_u128(saturating_abs_i128(exec_size) as u128, exec_price as u128);
            let total_notional = add_u128(old_notional, new_notional);
            let total_size = saturating_abs_i128(lp.position_size).saturating_add(saturating_abs_i128(exec_size));

            if total_size != 0 {
                new_lp_entry = div_u128(total_notional, total_size as u128)? as u64;
            }
        }

        // Apply fee to insurance fund
        self.insurance_fund.fee_revenue = add_u128(self.insurance_fund.fee_revenue, fee);
        self.insurance_fund.balance = add_u128(self.insurance_fund.balance, fee);

        // Update user account
        user.pnl = user.pnl.saturating_add(user_pnl_delta);
        user.pnl = user.pnl.saturating_sub(fee as i128);
        user.position_size = new_user_position;
        user.entry_price = new_user_entry;

        // Check user maintenance margin requirement
        if user.position_size != 0 {
            let user_collateral = add_u128(user.capital, clamp_pos_i128(user.pnl));
            let position_value = mul_u128(
                saturating_abs_i128(user.position_size) as u128,
                oracle_price as u128
            ) / 1_000_000;
            let margin_required = mul_u128(
                position_value,
                self.params.maintenance_margin_bps as u128
            ) / 10_000;

            if user_collateral <= margin_required {
                return Err(RiskError::Undercollateralized);
            }
        }

        // Update LP account
        lp.pnl = lp.pnl.saturating_add(lp_pnl_delta);
        lp.position_size = new_lp_position;
        lp.entry_price = new_lp_entry;

        // Check LP maintenance margin requirement
        if lp.position_size != 0 {
            let lp_collateral = add_u128(lp.capital, clamp_pos_i128(lp.pnl));
            let position_value = mul_u128(
                saturating_abs_i128(lp.position_size) as u128,
                oracle_price as u128
            ) / 1_000_000;
            let margin_required = mul_u128(
                position_value,
                self.params.maintenance_margin_bps as u128
            ) / 10_000;

            if lp_collateral <= margin_required {
                return Err(RiskError::Undercollateralized);
            }
        }

        // Update warmup slopes after PNL changes
        self.update_warmup_slope(user_idx)?;
        self.update_warmup_slope(lp_idx)?;

        Ok(())
    }

    // ========================================
    // ADL (Auto-Deleveraging) - Scan-Based
    // ========================================

    /// Calculate unwrapped PNL for an account (inline helper for ADL)
    /// Unwrapped = positive_pnl - withdrawable - reserved
    #[inline]
    fn compute_unwrapped_pnl(&self, account: &Account) -> u128 {
        if account.pnl <= 0 {
            return 0;
        }

        let positive_pnl = account.pnl as u128;
        let available_pnl = positive_pnl.saturating_sub(account.reserved_pnl);

        // Apply warmup pause - when paused, warmup cannot progress beyond pause_slot
        let effective_slot = if self.warmup_paused {
            core::cmp::min(self.current_slot, self.warmup_pause_slot)
        } else {
            self.current_slot
        };

        // Calculate withdrawable inline
        let elapsed_slots = effective_slot.saturating_sub(account.warmup_started_at_slot);
        let warmed_up_cap = mul_u128(account.warmup_slope_per_step, elapsed_slots as u128);
        let withdrawable = core::cmp::min(available_pnl, warmed_up_cap);

        // Unwrapped = positive_pnl - withdrawable - reserved
        positive_pnl
            .saturating_sub(withdrawable)
            .saturating_sub(account.reserved_pnl)
    }

    /// Apply ADL haircut using two-pass bitmap scan (stack-safe, no caching)
    ///
    /// Pass 1: Compute total unwrapped PNL across all accounts
    /// Pass 2: Recompute each account's unwrapped PNL and apply proportional haircut
    ///
    /// Waterfall: unwrapped PNL first, then insurance fund, then loss_accum
    pub fn apply_adl(&mut self, total_loss: u128) -> Result<()> {
        // ADL reduces risk (allowed in risk mode)
        self.enforce_op(OpClass::RiskReduce)?;

        if total_loss == 0 {
            return Ok(());
        }

        // Pass 1: Compute total unwrapped PNL (no caching - deterministic recomputation)
        let mut total_unwrapped = 0u128;

        for (block, word) in self.used.iter().copied().enumerate() {
            let mut w = word;
            while w != 0 {
                let bit = w.trailing_zeros() as usize;
                let idx = block * 64 + bit;
                w &= w - 1;

                let unwrapped = self.compute_unwrapped_pnl(&self.accounts[idx]);
                total_unwrapped = total_unwrapped.saturating_add(unwrapped);
            }
        }

        // Determine how much loss can be socialized via unwrapped PNL
        let loss_to_socialize = core::cmp::min(total_loss, total_unwrapped);

        // Pass 2: Apply proportional haircuts by recomputing unwrapped PNL
        if loss_to_socialize > 0 && total_unwrapped > 0 {
            // Must use manual iteration since we need self for compute_unwrapped_pnl
            for block in 0..BITMAP_WORDS {
                let mut w = self.used[block];
                while w != 0 {
                    let bit = w.trailing_zeros() as usize;
                    let idx = block * 64 + bit;
                    w &= w - 1;

                    // Recompute unwrapped (deterministic - same value as pass 1)
                    let account = &self.accounts[idx];
                    if account.pnl > 0 {
                        let unwrapped = self.compute_unwrapped_pnl(account);

                        if unwrapped > 0 {
                            let haircut = (loss_to_socialize * unwrapped) / total_unwrapped;
                            self.accounts[idx].pnl = self.accounts[idx].pnl.saturating_sub(haircut as i128);
                        }
                    }
                }
            }
        }

        // Handle remaining loss with insurance fund
        let remaining_loss = total_loss.saturating_sub(loss_to_socialize);

        if remaining_loss > 0 {
            if self.insurance_fund.balance < remaining_loss {
                // Insurance fund depleted - crisis mode
                let insurance_used = self.insurance_fund.balance;
                self.insurance_fund.balance = 0;
                self.loss_accum = add_u128(
                    self.loss_accum,
                    remaining_loss.saturating_sub(insurance_used)
                );

                // Enter risk-reduction-only mode (freezes warmup)
                self.enter_risk_reduction_only_mode();
            } else {
                // Deduct from insurance fund
                self.insurance_fund.balance = sub_u128(self.insurance_fund.balance, remaining_loss);

                // Check if we've dropped below configured threshold
                if self.insurance_fund.balance < self.params.risk_reduction_threshold && !self.risk_reduction_only {
                    // Enter risk-reduction-only mode at threshold
                    self.enter_risk_reduction_only_mode();
                }
            }
        }

        Ok(())
    }

    // ========================================
    // Panic Settlement (Atomic Global Settle)
    // ========================================

    /// Atomic global settlement at oracle price
    ///
    /// This is a single-tx emergency instruction that:
    /// 1. Enters risk-reduction-only mode and freezes warmups
    /// 2. Settles all open positions at the given oracle price
    /// 3. Clamps negative PNL to zero and accumulates system loss
    /// 4. Applies ADL to socialize the loss (unwrapped PNL first, then insurance, then loss_accum)
    ///
    /// No funding settlement is performed - this is purely position settlement.
    pub fn panic_settle_all(&mut self, oracle_price: u64) -> Result<()> {
        // Panic settle is a risk-reducing operation
        self.enforce_op(OpClass::RiskReduce)?;

        // Always enter risk-reduction-only mode (freezes warmups)
        self.enter_risk_reduction_only_mode();

        // Accumulate total system loss from negative PNL after settlement
        let mut total_loss = 0u128;
        // Track sum of mark PNL to compensate for integer division rounding
        let mut total_mark_pnl: i128 = 0;

        // Single pass: settle all positions and clamp negative PNL
        for block in 0..BITMAP_WORDS {
            let mut w = self.used[block];
            while w != 0 {
                let bit = w.trailing_zeros() as usize;
                let idx = block * 64 + bit;
                w &= w - 1;

                let account = &mut self.accounts[idx];

                // Skip accounts with no position
                if account.position_size == 0 {
                    continue;
                }

                // Compute mark PNL at oracle price
                let pos = account.position_size;
                let abs_pos = saturating_abs_i128(pos) as u128;

                let diff: i128 = if pos > 0 {
                    // Long: profit when oracle > entry
                    (oracle_price as i128).saturating_sub(account.entry_price as i128)
                } else {
                    // Short: profit when entry > oracle
                    (account.entry_price as i128).saturating_sub(oracle_price as i128)
                };

                // mark_pnl = diff * abs_pos / 1_000_000
                let mark_pnl = diff
                    .checked_mul(abs_pos as i128)
                    .ok_or(RiskError::Overflow)?
                    .checked_div(1_000_000)
                    .ok_or(RiskError::Overflow)?;

                // Track total mark PNL for rounding compensation
                total_mark_pnl = total_mark_pnl.saturating_add(mark_pnl);

                // Apply mark PNL to account
                account.pnl = account.pnl.saturating_add(mark_pnl);

                // Close position
                account.position_size = 0;
                account.entry_price = oracle_price; // Set to oracle for determinism

                // Clamp negative PNL and accumulate system loss
                if account.pnl < 0 {
                    // Convert negative PNL to system loss
                    let loss = (-account.pnl) as u128;
                    total_loss = total_loss.saturating_add(loss);
                    account.pnl = 0;
                }
            }
        }

        // Compensate for integer division rounding slippage to maintain conservation.
        // Due to truncation toward zero, sum(mark_pnl) may not be exactly zero even
        // though positions are zero-sum. This creates a discrepancy:
        // - If total_mark_pnl > 0: accounts claim more profit than they should (phantom profit)
        //   → treat as additional loss to be socialized
        // - If total_mark_pnl < 0: accounts claim less than they should (phantom loss)
        //   → add the excess back to insurance fund
        if total_mark_pnl > 0 {
            // Rounding created phantom profits - add to total loss
            total_loss = total_loss.saturating_add(total_mark_pnl as u128);
        } else if total_mark_pnl < 0 {
            // Rounding created phantom losses - credit to insurance
            self.insurance_fund.balance = add_u128(
                self.insurance_fund.balance,
                (-total_mark_pnl) as u128,
            );
        }

        // Socialize the accumulated loss via ADL waterfall
        if total_loss > 0 {
            self.apply_adl(total_loss)?;
        }

        Ok(())
    }

    /// Top up insurance fund to cover losses
    pub fn top_up_insurance_fund(&mut self, amount: u128) -> Result<bool> {
        // Insurance top-ups reduce risk (allowed in risk mode)
        self.enforce_op(OpClass::RiskReduce)?;

        // Add to vault
        self.vault = add_u128(self.vault, amount);

        // Apply contribution to loss_accum first (if any)
        if self.loss_accum > 0 {
            let loss_coverage = core::cmp::min(amount, self.loss_accum);
            self.loss_accum = sub_u128(self.loss_accum, loss_coverage);
            let remaining = sub_u128(amount, loss_coverage);

            // Add remaining to insurance fund balance
            self.insurance_fund.balance = add_u128(self.insurance_fund.balance, remaining);

            // Exit risk-reduction-only mode if loss is fully covered and above threshold
            let was_in_mode = self.risk_reduction_only;
            self.exit_risk_reduction_only_mode_if_safe();
            if was_in_mode && !self.risk_reduction_only {
                Ok(true) // Exited risk-reduction-only mode
            } else {
                Ok(false) // Still in risk-reduction-only mode
            }
        } else {
            // No loss - just add to insurance fund
            self.insurance_fund.balance = add_u128(self.insurance_fund.balance, amount);

            // Check if we can exit risk-reduction mode (may have been triggered by threshold, not loss)
            let was_in_mode = self.risk_reduction_only;
            self.exit_risk_reduction_only_mode_if_safe();
            if was_in_mode && !self.risk_reduction_only {
                Ok(true) // Exited risk-reduction-only mode
            } else {
                Ok(false)
            }
        }
    }

    // ========================================
    // Utilities
    // ========================================

    /// Check conservation invariant (I2)
    ///
    /// Conservation formula: vault + loss_accum = sum(capital) + sum(pnl) + insurance_fund.balance
    ///
    /// This accounts for:
    /// - Deposits add to both vault and capital
    /// - Withdrawals subtract from both vault and capital
    /// - Trading PNL is zero-sum between counterparties
    /// - Trading fees transfer from user PNL to insurance fund (net zero)
    /// - ADL transfers from user PNL to cover losses (net zero within system)
    /// - loss_accum represents value that was "lost" from the vault (clamped negative PNL
    ///   that couldn't be socialized), so vault + loss_accum = original value
    pub fn check_conservation(&self) -> bool {
        let mut total_capital = 0u128;
        let mut net_pnl: i128 = 0;

        self.for_each_used(|_idx, account| {
            total_capital = add_u128(total_capital, account.capital);
            net_pnl = net_pnl.saturating_add(account.pnl);
        });

        // expected = total_capital + net_pnl + insurance_fund.balance
        // actual = vault + loss_accum (loss_accum is value that "left" the system)
        let base = add_u128(total_capital, self.insurance_fund.balance);

        let expected = if net_pnl >= 0 {
            add_u128(base, net_pnl as u128)
        } else {
            base.saturating_sub((-net_pnl) as u128)
        };

        // vault + loss_accum should equal expected
        let actual = add_u128(self.vault, self.loss_accum);

        actual == expected
    }

    /// Advance to next slot (for testing warmup)
    pub fn advance_slot(&mut self, slots: u64) {
        self.current_slot = self.current_slot.saturating_add(slots);
    }
}
