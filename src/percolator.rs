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

// MAX_ACCOUNTS is feature-configured, not target-configured.
// This ensures x86 and SBF builds use the same sizes for a given feature set.
#[cfg(kani)]
pub const MAX_ACCOUNTS: usize = 8; // Small for fast formal verification

#[cfg(all(feature = "test", not(kani)))]
pub const MAX_ACCOUNTS: usize = 64; // Small for tests

#[cfg(all(not(kani), not(feature = "test")))]
pub const MAX_ACCOUNTS: usize = 4096; // Production

// Derived constants - all use size_of, no hardcoded values
pub const BITMAP_WORDS: usize = (MAX_ACCOUNTS + 63) / 64;
pub const MAX_ROUNDING_SLACK: u128 = MAX_ACCOUNTS as u128;

/// Maximum number of dust accounts to close per crank call.
/// Limits compute usage while still making progress on cleanup.
pub const GC_CLOSE_BUDGET: u32 = 32;

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

    // ========================================
    // Owner & Maintenance Fees (wrapper-related)
    // ========================================
    /// Owner pubkey (32 bytes, signature checks done by wrapper)
    pub owner: [u8; 32],

    /// Fee credits in capital units (can go negative if fees owed)
    pub fee_credits: i128,

    /// Last slot when maintenance fees were settled for this account
    pub last_fee_slot: u64,
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
        owner: [0; 32],
        fee_credits: 0,
        last_fee_slot: 0,
    }
}

/// Insurance fund state
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InsuranceFund {
    /// Insurance fund balance
    pub balance: u128,

    /// Accumulated fees from trades
    pub fee_revenue: u128,
}

/// Outcome from oracle_close_position_core helper
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClosedOutcome {
    /// Absolute position size that was closed
    pub abs_pos: u128,
    /// Mark PnL from closing at oracle price
    pub mark_pnl: i128,
    /// Capital before settlement
    pub cap_before: u128,
    /// Capital after settlement
    pub cap_after: u128,
    /// Whether a position was actually closed
    pub position_was_closed: bool,
}

/// Risk engine parameters
#[repr(C)]
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

    /// Flat account creation fee (absolute amount in capital units)
    pub new_account_fee: u128,

    /// Insurance fund threshold for entering risk-reduction-only mode
    /// If insurance fund balance drops below this, risk-reduction mode activates
    pub risk_reduction_threshold: u128,

    // ========================================
    // Maintenance Fee Parameters
    // ========================================
    /// Maintenance fee per account per slot (in capital units)
    /// Engine is purely slot-native; any per-day conversion is wrapper/UI responsibility
    pub maintenance_fee_per_slot: u128,

    /// Maximum allowed staleness before crank is required (in slots)
    /// Set to u64::MAX to disable crank freshness check
    pub max_crank_staleness_slots: u64,

    /// Liquidation fee in basis points (e.g., 50 = 0.50%)
    /// Paid from liquidated account's capital into insurance fund
    pub liquidation_fee_bps: u64,

    /// Absolute cap on liquidation fee (in capital units)
    /// Prevents whales paying enormous fees
    pub liquidation_fee_cap: u128,

    // ========================================
    // Partial Liquidation Parameters
    // ========================================
    /// Buffer above maintenance margin (in basis points) to target after partial liquidation.
    /// E.g., if maintenance is 500 bps (5%) and buffer is 100 bps (1%), we target 6% margin.
    /// This prevents immediate re-liquidation from small price movements.
    pub liquidation_buffer_bps: u64,

    /// Minimum absolute position size after partial liquidation.
    /// If remaining position would be below this threshold, full liquidation occurs.
    /// Prevents dust positions that are uneconomical to maintain or re-liquidate.
    /// Denominated in base units (same scale as position_size.abs()).
    pub min_liquidation_abs: u128,
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
    // Keeper Crank Tracking
    // ========================================
    /// Last slot when keeper crank was executed
    pub last_crank_slot: u64,

    /// Maximum allowed staleness before crank is required (in slots)
    pub max_crank_staleness_slots: u64,

    // ========================================
    // Open Interest Tracking (O(1))
    // ========================================
    /// Total open interest = sum of abs(position_size) across all accounts
    /// This measures total risk exposure in the system.
    pub total_open_interest: u128,

    // ========================================
    // Warmup Budget Tracking
    // ========================================
    /// Cumulative positive PnL converted to capital (W+)
    pub warmed_pos_total: u128,

    /// Cumulative negative PnL paid from capital (W-)
    pub warmed_neg_total: u128,

    /// Insurance above the floor that has been committed to backing warmed profits (monotone)
    pub warmup_insurance_reserved: u128,

    // ========================================
    // ADL Scratch (production stack-safe)
    // ========================================
    /// Scratch: per-account remainders used by ADL largest-remainder distribution.
    /// Stored in slab to avoid stack allocation in production.
    /// Only meaningful for used accounts; others must be zeroed when not used.
    pub adl_remainder_scratch: [u128; MAX_ACCOUNTS],

    /// Scratch: per-account "eligible" bit for ADL remainder distribution.
    /// 0 = not eligible / already consumed; 1 = eligible.
    pub adl_eligible_scratch: [u8; MAX_ACCOUNTS],

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

/// Outcome of a keeper crank operation
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CrankOutcome {
    /// Whether the crank successfully advanced last_crank_slot
    pub advanced: bool,
    /// Slots forgiven for caller's maintenance (50% discount via time forgiveness)
    pub slots_forgiven: u64,
    /// Whether caller's maintenance fee settle succeeded (false if undercollateralized)
    pub caller_settle_ok: bool,
    /// Whether force_realize_losses was triggered
    pub did_force_realize: bool,
    /// Whether panic_settle_all was triggered
    pub did_panic_settle: bool,
    /// Number of accounts liquidated during this crank
    pub num_liquidations: u32,
    /// Number of dust accounts garbage collected during this crank
    pub num_gc_closed: u32,
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
    if val > 0 {
        val as u128
    } else {
        0
    }
}

#[allow(dead_code)]
#[inline]
fn clamp_neg_i128(val: i128) -> u128 {
    if val < 0 {
        neg_i128_to_u128(val)
    } else {
        0
    }
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

/// Safely convert negative i128 to u128 (handles i128::MIN without overflow)
///
/// For i128::MIN, -i128::MIN would overflow because i128::MAX + 1 cannot be represented.
/// We handle this by returning (i128::MAX as u128) + 1 = 170141183460469231731687303715884105728.
#[inline]
fn neg_i128_to_u128(val: i128) -> u128 {
    debug_assert!(val < 0, "neg_i128_to_u128 called with non-negative value");
    if val == i128::MIN {
        (i128::MAX as u128) + 1
    } else {
        (-val) as u128
    }
}

/// Safely convert u128 to i128 with clamping (handles values > i128::MAX)
///
/// If x > i128::MAX, the cast would wrap to a negative value.
/// We clamp to i128::MAX instead to preserve correctness of margin checks.
#[inline]
fn u128_to_i128_clamped(x: u128) -> i128 {
    if x > i128::MAX as u128 {
        i128::MAX
    } else {
        x as i128
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
    /// Create a new risk engine (stack-allocates the full struct - avoid in BPF!)
    ///
    /// WARNING: This allocates ~6MB on the stack at MAX_ACCOUNTS=4096.
    /// For Solana BPF programs, use `init_in_place` instead.
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
            last_crank_slot: 0,
            max_crank_staleness_slots: params.max_crank_staleness_slots,
            total_open_interest: 0,
            warmed_pos_total: 0,
            warmed_neg_total: 0,
            warmup_insurance_reserved: 0,
            adl_remainder_scratch: [0; MAX_ACCOUNTS],
            adl_eligible_scratch: [0; MAX_ACCOUNTS],
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

    /// Initialize a RiskEngine in place (zero-copy friendly).
    ///
    /// PREREQUISITE: The memory backing `self` MUST be zeroed before calling.
    /// This method only sets non-zero fields to avoid touching the entire ~6MB struct.
    ///
    /// This is the correct way to initialize RiskEngine in Solana BPF programs
    /// where stack space is limited to 4KB.
    pub fn init_in_place(&mut self, params: RiskParams) {
        // Set params (non-zero field)
        self.params = params;
        self.max_crank_staleness_slots = params.max_crank_staleness_slots;

        // Initialize freelist: 0 -> 1 -> 2 -> ... -> MAX_ACCOUNTS-1 -> NONE
        // All other fields are zero which is correct for:
        // - vault, insurance_fund, current_slot, funding_index, etc. = 0
        // - used bitmap = all zeros (no accounts in use)
        // - accounts = all zeros (equivalent to empty_account())
        // - free_head = 0 (first free slot is 0)
        for i in 0..MAX_ACCOUNTS - 1 {
            self.next_free[i] = (i + 1) as u16;
        }
        self.next_free[MAX_ACCOUNTS - 1] = u16::MAX; // Sentinel
    }

    // ========================================
    // Bitmap Helpers
    // ========================================

    pub fn is_used(&self, idx: usize) -> bool {
        if idx >= MAX_ACCOUNTS {
            return false;
        }
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
                if idx >= MAX_ACCOUNTS {
                    continue; // Guard against stray high bits in bitmap
                }
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
                if idx >= MAX_ACCOUNTS {
                    continue; // Guard against stray high bits in bitmap
                }
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

        // Flat fee (no scaling)
        let required_fee = self.params.new_account_fee;
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
            owner: [0; 32],
            fee_credits: 0,
            last_fee_slot: self.current_slot,
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

        // Flat fee (no scaling)
        let required_fee = self.params.new_account_fee;
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
            owner: [0; 32],
            fee_credits: 0,
            last_fee_slot: self.current_slot,
        };

        Ok(idx)
    }

    // ========================================
    // Maintenance Fees
    // ========================================

    /// Settle maintenance fees for an account.
    ///
    /// Returns the fee amount due (for keeper rebate calculation).
    ///
    /// Algorithm:
    /// 1. Compute dt = now_slot - account.last_fee_slot
    /// 2. If dt == 0, return 0 (no-op)
    /// 3. Compute due = fee_per_slot * dt
    /// 4. Deduct from fee_credits; if negative, pay from capital to insurance
    /// 5. If position exists and below maintenance after fee, return Err
    pub fn settle_maintenance_fee(
        &mut self,
        idx: u16,
        now_slot: u64,
        oracle_price: u64,
    ) -> Result<u128> {
        if idx as usize >= MAX_ACCOUNTS || !self.is_used(idx as usize) {
            return Err(RiskError::Unauthorized);
        }

        let account = &mut self.accounts[idx as usize];

        // Calculate elapsed time
        let dt = now_slot.saturating_sub(account.last_fee_slot);
        if dt == 0 {
            return Ok(0);
        }

        // Calculate fee due (engine is purely slot-native)
        let due = self.params.maintenance_fee_per_slot.saturating_mul(dt as u128);

        // Update last_fee_slot
        account.last_fee_slot = now_slot;

        // Deduct from fee_credits
        account.fee_credits = account.fee_credits.saturating_sub(due as i128);

        // If fee_credits is negative, pay from capital
        if account.fee_credits < 0 {
            let owed = neg_i128_to_u128(account.fee_credits);
            let pay = core::cmp::min(owed, account.capital);

            account.capital = account.capital.saturating_sub(pay);
            self.insurance_fund.balance = add_u128(self.insurance_fund.balance, pay);
            self.insurance_fund.fee_revenue = add_u128(self.insurance_fund.fee_revenue, pay);

            // Credit back what was paid
            account.fee_credits = account.fee_credits.saturating_add(pay as i128);
        }

        // Check maintenance margin if account has a position
        if account.position_size != 0 {
            // Re-borrow immutably for margin check
            let account_ref = &self.accounts[idx as usize];
            if !self.is_above_maintenance_margin(account_ref, oracle_price) {
                return Err(RiskError::Undercollateralized);
            }
        }

        Ok(due) // Return fee due for keeper rebate calculation
    }

    /// Set owner pubkey for an account
    pub fn set_owner(&mut self, idx: u16, owner: [u8; 32]) -> Result<()> {
        if idx as usize >= MAX_ACCOUNTS || !self.is_used(idx as usize) {
            return Err(RiskError::Unauthorized);
        }
        self.accounts[idx as usize].owner = owner;
        Ok(())
    }

    /// Add fee credits to an account (e.g., user deposits fee credits)
    pub fn add_fee_credits(&mut self, idx: u16, amount: u128) -> Result<()> {
        if idx as usize >= MAX_ACCOUNTS || !self.is_used(idx as usize) {
            return Err(RiskError::Unauthorized);
        }
        self.accounts[idx as usize].fee_credits =
            self.accounts[idx as usize].fee_credits.saturating_add(amount as i128);
        Ok(())
    }

    /// Set the risk reduction threshold (admin function).
    /// This controls when risk-reduction-only mode is triggered.
    #[inline]
    pub fn set_risk_reduction_threshold(&mut self, new_threshold: u128) {
        self.params.risk_reduction_threshold = new_threshold;
    }

    /// Get the current risk reduction threshold.
    #[inline]
    pub fn risk_reduction_threshold(&self) -> u128 {
        self.params.risk_reduction_threshold
    }

    /// Close an account and return its capital to the caller.
    ///
    /// Requirements:
    /// - Account must exist
    /// - Position must be zero (no open positions)
    /// - fee_credits >= 0 (no outstanding fees owed)
    /// - pnl must be 0 after settlement (any unwarmed positive pnl is forfeited)
    ///
    /// Returns the amount withdrawn (capital only - pnl must go through warmup first).
    pub fn close_account(
        &mut self,
        idx: u16,
        now_slot: u64,
        oracle_price: u64,
    ) -> Result<u128> {
        if idx as usize >= MAX_ACCOUNTS || !self.is_used(idx as usize) {
            return Err(RiskError::AccountNotFound);
        }

        // Full settlement: funding + maintenance fees + warmup
        // This converts warmed pnl to capital and realizes negative pnl
        self.touch_account_full(idx, now_slot, oracle_price)?;

        let account = &self.accounts[idx as usize];

        // Position must be zero
        if account.position_size != 0 {
            return Err(RiskError::Undercollateralized); // Has open position
        }

        // Check no outstanding fees owed
        if account.fee_credits < 0 {
            return Err(RiskError::InsufficientBalance); // Owes fees
        }

        // After full settlement, negative pnl has been realized from capital.
        // Positive unwarmed pnl remains in pnl field but is NOT withdrawable.
        // Users must wait for warmup to convert pnl to capital before closing.
        // If there's still positive pnl, it means warmup hasn't completed - user forfeits it.
        // (This is the security property: can't bypass warmup via close_account)

        // Return capital only (warmed profits are already in capital, unwarmed are forfeited)
        let capital = account.capital;

        // Deduct from vault
        if capital > self.vault {
            return Err(RiskError::InsufficientBalance);
        }
        self.vault = self.vault.saturating_sub(capital);

        // Free the slot
        self.free_slot(idx);

        Ok(capital)
    }

    /// Free an account slot (internal helper).
    /// Clears the account, bitmap, and returns slot to freelist.
    /// Caller must ensure the account is safe to free (no capital, no positive pnl, etc).
    fn free_slot(&mut self, idx: u16) {
        self.accounts[idx as usize] = empty_account();
        self.clear_used(idx as usize);
        self.next_free[idx as usize] = self.free_head;
        self.free_head = idx;
        self.num_used_accounts = self.num_used_accounts.saturating_sub(1);
    }

    /// Garbage collect dust accounts.
    ///
    /// A "dust account" is a slot that can never pay out anything:
    /// - position_size == 0
    /// - capital == 0
    /// - reserved_pnl == 0
    /// - pnl <= 0
    ///
    /// Any remaining negative PnL is socialized via ADL waterfall before freeing.
    /// No token transfers occur - this is purely internal bookkeeping cleanup.
    ///
    /// Called at end of keeper_crank after liquidation/settlement has already run.
    ///
    /// Returns the number of accounts closed.
    pub fn garbage_collect_dust(&mut self) -> u32 {
        // Two separate lists: zero-pnl (always freeable) vs negative-pnl (needs ADL)
        let mut to_free_zero: [u16; GC_CLOSE_BUDGET as usize] = [0; GC_CLOSE_BUDGET as usize];
        let mut to_free_neg: [u16; GC_CLOSE_BUDGET as usize] = [0; GC_CLOSE_BUDGET as usize];
        let mut num_zero = 0usize;
        let mut num_neg = 0usize;
        let mut total_unpaid: u128 = 0;

        // Pass 1: Collect dust candidates (up to budget)
        let total_budget = GC_CLOSE_BUDGET as usize;
        'outer: for block in 0..BITMAP_WORDS {
            let mut w = self.used[block];
            while w != 0 {
                let bit = w.trailing_zeros() as usize;
                let idx = block * 64 + bit;
                w &= w - 1;

                if idx >= MAX_ACCOUNTS {
                    continue;
                }

                // Budget check (combined count)
                if num_zero + num_neg >= total_budget {
                    break 'outer;
                }

                let account = &self.accounts[idx];

                // Dust predicate: must have zero position, capital, reserved, and non-positive pnl
                if account.position_size != 0 {
                    continue;
                }
                if account.capital != 0 {
                    continue;
                }
                if account.reserved_pnl != 0 {
                    continue;
                }
                if account.pnl > 0 {
                    continue;
                }
                // Funding must be settled to avoid unsettled-value footgun
                if account.funding_index != self.funding_index_qpb_e6 {
                    continue;
                }

                if account.pnl < 0 {
                    // Track negative pnl for batched ADL
                    let add = neg_i128_to_u128(account.pnl);
                    let new_total = total_unpaid.saturating_add(add);
                    // Saturation guard: stop collecting negative-pnl if saturated
                    if new_total == total_unpaid && add > 0 {
                        break 'outer;
                    }
                    total_unpaid = new_total;
                    to_free_neg[num_neg] = idx as u16;
                    num_neg += 1;
                } else {
                    // pnl == 0: always freeable
                    to_free_zero[num_zero] = idx as u16;
                    num_zero += 1;
                }
            }
        }

        // Pass 2: Try batched ADL for negative-pnl accounts
        let adl_ok = if total_unpaid > 0 {
            self.apply_adl(total_unpaid).is_ok()
        } else {
            true
        };

        let mut freed = 0u32;

        // Pass 3a: Always free zero-pnl accounts
        for i in 0..num_zero {
            self.free_slot(to_free_zero[i]);
            freed += 1;
        }

        // Pass 3b: Only free negative-pnl accounts if ADL succeeded
        if adl_ok {
            for i in 0..num_neg {
                self.accounts[to_free_neg[i] as usize].pnl = 0;
                self.free_slot(to_free_neg[i]);
                freed += 1;
            }
        }

        freed
    }

    // ========================================
    // Keeper Crank
    // ========================================

    /// Check if a fresh crank is required before state-changing operations.
    /// Returns Err if the crank is stale (too old).
    pub fn require_fresh_crank(&self, now_slot: u64) -> Result<()> {
        if now_slot.saturating_sub(self.last_crank_slot) > self.max_crank_staleness_slots {
            return Err(RiskError::Unauthorized); // NeedsCrank
        }
        Ok(())
    }

    /// Keeper crank entrypoint - advances global state and performs maintenance.
    ///
    /// Returns CrankOutcome with flags indicating what happened.
    ///
    /// Behavior:
    /// 1. Accrue funding
    /// 2. If now_slot <= last_crank_slot, return early (no-op)
    /// 3. Else advance last_crank_slot
    /// 4. Settle maintenance fees for caller
    /// 5. Evaluate heavy actions (force_realize, panic_settle)
    ///
    /// This is the single permissionless "do-the-right-thing" entrypoint.
    /// - Always attempts caller's maintenance settle with 50% discount (best-effort)
    /// - Only advances last_crank_slot when now_slot > last_crank_slot
    /// - Heavy actions run independent of caller settle success
    pub fn keeper_crank(
        &mut self,
        caller_idx: u16,
        now_slot: u64,
        oracle_price: u64,
        funding_rate_bps_per_slot: i64,
        allow_panic: bool,
    ) -> Result<CrankOutcome> {
        // Update current_slot so warmup/bookkeeping progresses consistently
        self.current_slot = now_slot;

        // Accrue funding first (always)
        let _ = self.accrue_funding(now_slot, oracle_price, funding_rate_bps_per_slot);

        // Check if we're advancing the global crank slot
        let advanced = now_slot > self.last_crank_slot;
        if advanced {
            self.last_crank_slot = now_slot;
        }

        // Always attempt caller's maintenance settle with 50% discount (best-effort)
        // This lets users "self-crank" even if someone already cranked this slot
        let (slots_forgiven, caller_settle_ok) = if (caller_idx as usize) < MAX_ACCOUNTS
            && self.is_used(caller_idx as usize)
        {
            // Pre-advance last_fee_slot by dt/2 to forgive half the elapsed time
            // This makes settle_maintenance_fee charge only half the slots
            let last_fee = self.accounts[caller_idx as usize].last_fee_slot;
            let dt = now_slot.saturating_sub(last_fee);
            let forgive = dt / 2;

            if forgive > 0 {
                self.accounts[caller_idx as usize].last_fee_slot =
                    last_fee.saturating_add(forgive);
            }

            // Now settle - will only charge for remaining half of dt
            // Best-effort: don't fail keeper_crank if caller is undercollateralized
            let settle_result = self.settle_maintenance_fee(caller_idx, now_slot, oracle_price);

            (forgive, settle_result.is_ok())
        } else {
            (0, true) // No caller to settle, considered ok
        };

        // Proactive liquidation scan: liquidate any accounts below maintenance margin
        // This runs on EVERY crank call (not just when advanced) to ensure timely liquidations
        let num_liquidations = self.scan_and_liquidate_all(now_slot, oracle_price);

        // Heavy actions run independent of caller settle success
        let mut did_force_realize = false;
        let mut did_panic_settle = false;

        // Only run heavy actions when we actually advanced the crank
        if advanced {
            if self.insurance_fund.balance <= self.params.risk_reduction_threshold {
                // Insurance at or below floor - force realize losses
                if self.force_realize_losses(oracle_price).is_ok() {
                    did_force_realize = true;
                }
            } else if (self.loss_accum > 0 || self.risk_reduction_only)
                && allow_panic
                && self.total_open_interest > 0
            {
                // System in stress with open positions - panic settle
                if self.panic_settle_all(oracle_price).is_ok() {
                    did_panic_settle = true;
                }
            }
        }

        // Garbage collect dust accounts (runs every crank)
        let num_gc_closed = self.garbage_collect_dust();

        Ok(CrankOutcome {
            advanced,
            slots_forgiven,
            caller_settle_ok,
            did_force_realize,
            did_panic_settle,
            num_liquidations,
            num_gc_closed,
        })
    }

    // ========================================
    // Liquidation
    // ========================================

    /// Compute mark PnL for a position at oracle price (pure helper, no side effects).
    /// Returns the PnL from closing the position at oracle price.
    /// - Longs: profit when oracle > entry
    /// - Shorts: profit when entry > oracle
    pub fn mark_pnl_for_position(pos: i128, entry: u64, oracle: u64) -> Result<i128> {
        if pos == 0 {
            return Ok(0);
        }

        let abs_pos = saturating_abs_i128(pos) as u128;

        let diff: i128 = if pos > 0 {
            // Long: profit when oracle > entry
            (oracle as i128).saturating_sub(entry as i128)
        } else {
            // Short: profit when entry > oracle
            (entry as i128).saturating_sub(oracle as i128)
        };

        // mark_pnl = diff * abs_pos / 1_000_000
        diff.checked_mul(abs_pos as i128)
            .ok_or(RiskError::Overflow)?
            .checked_div(1_000_000)
            .ok_or(RiskError::Overflow)
    }

    /// Compute how much position to close for liquidation (closed-form, single-pass).
    ///
    /// Returns (close_abs, is_full_close) where:
    /// - close_abs = absolute position size to close
    /// - is_full_close = true if this is a full position close (including dust kill-switch)
    ///
    /// ## Algorithm:
    /// 1. Compute target_bps = maintenance_margin_bps + liquidation_buffer_bps
    /// 2. Compute max safe remaining position: abs_pos_safe_max = floor(E * 10_000 * 1_000_000 / (P * target_bps))
    /// 3. close_abs = abs_pos - abs_pos_safe_max
    /// 4. If remaining position < min_liquidation_abs, do full close (dust kill-switch)
    ///
    /// This is deterministic, requires no iteration, and guarantees single-pass liquidation.
    pub fn compute_liquidation_close_amount(
        &self,
        account: &Account,
        oracle_price: u64,
    ) -> (u128, bool) {
        let abs_pos = saturating_abs_i128(account.position_size) as u128;
        if abs_pos == 0 {
            return (0, false);
        }

        // Account equity (may be 0 if underwater)
        let equity = self.account_equity(account);

        // Target margin = maintenance + buffer (in basis points)
        let target_bps = self.params.maintenance_margin_bps
            .saturating_add(self.params.liquidation_buffer_bps);

        // Maximum safe remaining position (floor-safe calculation)
        // abs_pos_safe_max = floor(equity * 10_000 * 1_000_000 / (oracle_price * target_bps))
        // Rearranged to avoid intermediate overflow:
        // abs_pos_safe_max = floor(equity * 10_000_000_000 / (oracle_price * target_bps))
        let numerator = mul_u128(equity, 10_000_000_000);
        let denominator = mul_u128(oracle_price as u128, target_bps as u128);

        let mut abs_pos_safe_max = if denominator == 0 {
            0 // Edge case: full liquidation if no denominator
        } else {
            numerator / denominator
        };

        // Clamp to current position (can't have safe max > actual position)
        abs_pos_safe_max = core::cmp::min(abs_pos_safe_max, abs_pos);

        // Conservative rounding guard: subtract 1 unit to ensure we close slightly more
        // than mathematically required. This guarantees post-liquidation account is
        // strictly on the safe side of the inequality despite integer truncation.
        if abs_pos_safe_max > 0 {
            abs_pos_safe_max -= 1;
        }

        // Required close amount
        let close_abs = abs_pos.saturating_sub(abs_pos_safe_max);

        // Dust kill-switch: if remaining position would be below min, do full close
        let remaining = abs_pos.saturating_sub(close_abs);
        if remaining < self.params.min_liquidation_abs {
            return (abs_pos, true); // Full close
        }

        (close_abs, close_abs == abs_pos)
    }

    /// Core helper for closing a SLICE of a position at oracle price (partial liquidation).
    ///
    /// Similar to oracle_close_position_core but:
    /// - Only closes `close_abs` units of position (not the entire position)
    /// - Computes proportional mark_pnl for the closed slice
    /// - Entry price remains unchanged (correct for same-direction partial reduction)
    ///
    /// ## PnL Routing (same invariant as full close):
    /// - mark_pnl > 0 (profit) → funded via apply_adl() waterfall
    /// - mark_pnl <= 0 (loss) → realized via settle_warmup_to_capital (capital path)
    /// - Residual negative PnL (capital exhausted) → routed through ADL, PnL clamped to 0
    ///
    /// ASSUMES: Caller has already called touch_account_full() on this account.
    fn oracle_close_position_slice_core(
        &mut self,
        idx: u16,
        oracle_price: u64,
        close_abs: u128,
    ) -> Result<ClosedOutcome> {
        // NOTE: Caller must have already called touch_account_full()
        // to settle funding, maintenance, and warmup.

        let pos = self.accounts[idx as usize].position_size;
        let current_abs_pos = saturating_abs_i128(pos) as u128;

        // Validate: can't close more than we have
        if close_abs == 0 || current_abs_pos == 0 {
            return Ok(ClosedOutcome {
                abs_pos: 0,
                mark_pnl: 0,
                cap_before: self.accounts[idx as usize].capital,
                cap_after: self.accounts[idx as usize].capital,
                position_was_closed: false,
            });
        }

        // If close_abs >= current position, delegate to full close
        if close_abs >= current_abs_pos {
            return self.oracle_close_position_core(idx, oracle_price);
        }

        // Partial close: close_abs < current_abs_pos
        let entry = self.accounts[idx as usize].entry_price;
        let cap_before = self.accounts[idx as usize].capital;

        // Compute proportional mark PnL for the closed slice
        // mark_pnl_slice = (close_abs / abs_pos) * full_mark_pnl
        // But we compute directly: mark_pnl = diff * close_abs / 1_000_000
        let diff: i128 = if pos > 0 {
            (oracle_price as i128).saturating_sub(entry as i128)
        } else {
            (entry as i128).saturating_sub(oracle_price as i128)
        };

        let mark_pnl = diff
            .checked_mul(close_abs as i128)
            .ok_or(RiskError::Overflow)?
            .checked_div(1_000_000)
            .ok_or(RiskError::Overflow)?;

        // Apply mark PnL to account
        self.accounts[idx as usize].pnl = self.accounts[idx as usize].pnl.saturating_add(mark_pnl);

        // Update position: reduce by close_abs (maintain sign)
        let new_abs_pos = current_abs_pos.saturating_sub(close_abs);
        self.accounts[idx as usize].position_size = if pos > 0 {
            new_abs_pos as i128
        } else {
            -(new_abs_pos as i128)
        };

        // Entry price remains unchanged for remaining position
        // (partial close at oracle price doesn't change the entry of what remains)

        // Update OI
        self.total_open_interest = self.total_open_interest.saturating_sub(close_abs);

        // Route positive mark_pnl through ADL (excluding this account - it shouldn't fund its own profit)
        if mark_pnl > 0 {
            self.apply_adl_excluding(mark_pnl as u128, idx as usize)?;
        }

        // Settle warmup
        self.settle_warmup_to_capital(idx)?;

        // Handle residual negative PnL
        let residual_pnl = self.accounts[idx as usize].pnl;
        if residual_pnl < 0 {
            let unpaid = neg_i128_to_u128(residual_pnl);
            self.apply_adl(unpaid)?;
            self.accounts[idx as usize].pnl = 0;
        }

        let cap_after = self.accounts[idx as usize].capital;

        Ok(ClosedOutcome {
            abs_pos: close_abs,
            mark_pnl,
            cap_before,
            cap_after,
            position_was_closed: true,
        })
    }

    /// Core helper for oracle-price position close.
    ///
    /// This is the ONLY place that applies mark PnL + ADL routing + settlement
    /// for forced-close flows (liquidation, panic_settle, force_realize).
    ///
    /// ## PnL Routing Invariant:
    /// - mark_pnl > 0 (profit) → funded via apply_adl() waterfall
    /// - mark_pnl <= 0 (loss) → realized via settle_warmup_to_capital (capital path)
    /// - Residual negative PnL (capital exhausted) → routed through ADL, PnL clamped to 0
    ///
    /// No other path creates or destroys value.
    ///
    /// ASSUMES: Caller has already called touch_account_full() on this account.
    fn oracle_close_position_core(
        &mut self,
        idx: u16,
        oracle_price: u64,
    ) -> Result<ClosedOutcome> {
        // NOTE: Caller must have already called touch_account_full()
        // to settle funding, maintenance, and warmup.

        // Check if there's a position to close
        if self.accounts[idx as usize].position_size == 0 {
            return Ok(ClosedOutcome {
                abs_pos: 0,
                mark_pnl: 0,
                cap_before: self.accounts[idx as usize].capital,
                cap_after: self.accounts[idx as usize].capital,
                position_was_closed: false,
            });
        }

        // Snapshot position details and capital
        let pos = self.accounts[idx as usize].position_size;
        let abs_pos = saturating_abs_i128(pos) as u128;
        let entry = self.accounts[idx as usize].entry_price;
        let cap_before = self.accounts[idx as usize].capital;

        // Compute mark PnL at oracle price
        let mark_pnl = Self::mark_pnl_for_position(pos, entry, oracle_price)?;

        // Apply mark PnL to account
        self.accounts[idx as usize].pnl = self.accounts[idx as usize].pnl.saturating_add(mark_pnl);

        // Close position
        self.accounts[idx as usize].position_size = 0;
        self.accounts[idx as usize].entry_price = oracle_price; // Determinism

        // Update OI (remove this account's contribution)
        self.total_open_interest = self.total_open_interest.saturating_sub(abs_pos);

        // Route positive mark_pnl through ADL (excluding this account - it shouldn't fund its own profit)
        if mark_pnl > 0 {
            self.apply_adl_excluding(mark_pnl as u128, idx as usize)?;
        }
        // mark_pnl <= 0: losses realized from capital via settlement below

        // Settle warmup (realizes negative PnL from capital immediately, budgets positive)
        self.settle_warmup_to_capital(idx)?;

        // Handle residual negative PnL (capital exhausted)
        // This unpaid loss must be socialized via ADL waterfall, then clamp PnL to 0
        let residual_pnl = self.accounts[idx as usize].pnl;
        if residual_pnl < 0 {
            let unpaid = neg_i128_to_u128(residual_pnl);
            self.apply_adl(unpaid)?;
            self.accounts[idx as usize].pnl = 0;
        }

        // Snapshot capital after settlement
        let cap_after = self.accounts[idx as usize].capital;

        Ok(ClosedOutcome {
            abs_pos,
            mark_pnl,
            cap_before,
            cap_after,
            position_was_closed: true,
        })
    }

    /// Liquidate a single account at oracle price if below maintenance margin.
    ///
    /// Returns Ok(true) if liquidation occurred, Ok(false) if not needed/possible.
    /// This is an oracle-price force-close that does NOT require an LP/AMM.
    ///
    /// ## Partial Liquidation:
    /// Computes the minimum amount to close to bring the account to safety (above
    /// maintenance margin + buffer). If remaining position would be below
    /// min_liquidation_abs, full close occurs instead (dust kill-switch).
    ///
    /// Uses oracle_close_position_core (full) or oracle_close_position_slice_core (partial)
    /// for PnL routing, then charges liquidation fee on the closed amount.
    pub fn liquidate_at_oracle(
        &mut self,
        idx: u16,
        now_slot: u64,
        oracle_price: u64,
    ) -> Result<bool> {
        // Validate index
        if (idx as usize) >= MAX_ACCOUNTS || !self.is_used(idx as usize) {
            return Ok(false);
        }

        // Settle and check maintenance margin
        self.touch_account_full(idx, now_slot, oracle_price)?;

        let account = &self.accounts[idx as usize];
        if account.position_size == 0 {
            return Ok(false);
        }
        if self.is_above_maintenance_margin(account, oracle_price) {
            return Ok(false);
        }

        // Compute how much to close (closed-form, single-pass)
        let (close_abs, is_full_close) = self.compute_liquidation_close_amount(account, oracle_price);

        if close_abs == 0 {
            return Ok(false);
        }

        // Close position via appropriate helper
        // If partial close overflows, fall back to full close (overflow → full close safety)
        let mut outcome = if is_full_close {
            self.oracle_close_position_core(idx, oracle_price)?
        } else {
            match self.oracle_close_position_slice_core(idx, oracle_price, close_abs) {
                Ok(o) => o,
                Err(RiskError::Overflow) => {
                    // Overflow in partial close arithmetic → force full close
                    self.oracle_close_position_core(idx, oracle_price)?
                }
                Err(e) => return Err(e),
            }
        };

        if !outcome.position_was_closed {
            return Ok(false);
        }

        // Post-liquidation safety check: if position remains and still below target,
        // fall back to full close. This handles rare cases where mark_pnl realization
        // during partial close reduces equity enough to miss the target.
        let remaining_pos = self.accounts[idx as usize].position_size;
        if remaining_pos != 0 {
            let target_bps = self.params.maintenance_margin_bps
                .saturating_add(self.params.liquidation_buffer_bps);
            if !self.is_above_margin_bps(&self.accounts[idx as usize], oracle_price, target_bps) {
                // Fallback: close remaining position entirely
                let fallback_outcome = self.oracle_close_position_core(idx, oracle_price)?;
                if fallback_outcome.position_was_closed {
                    // Accumulate closed amount for fee calculation.
                    // Note: mark_pnl is not accumulated because only abs_pos is used
                    // downstream (fee calc). PnL routing already happened in each core call.
                    outcome.abs_pos = outcome.abs_pos.saturating_add(fallback_outcome.abs_pos);
                }
            }
        }

        // Compute and apply liquidation fee on total closed amount (this IS fee revenue)
        let notional = mul_u128(outcome.abs_pos, oracle_price as u128) / 1_000_000;
        let fee_raw = mul_u128(notional, self.params.liquidation_fee_bps as u128) / 10_000;
        let fee = core::cmp::min(fee_raw, self.params.liquidation_fee_cap);

        // Pay fee from account capital (capped by available capital)
        let account_capital = self.accounts[idx as usize].capital;
        let pay = core::cmp::min(fee, account_capital);

        self.accounts[idx as usize].capital = account_capital.saturating_sub(pay);
        self.insurance_fund.balance = self.insurance_fund.balance.saturating_add(pay);
        self.insurance_fund.fee_revenue = self.insurance_fund.fee_revenue.saturating_add(pay);

        // Recompute warmup reserved after insurance changes
        self.recompute_warmup_insurance_reserved();

        Ok(true)
    }

    /// Scan all used accounts and liquidate any that are below maintenance margin.
    /// Returns the number of accounts liquidated.
    /// Best-effort: errors on individual accounts are ignored (only operational errors
    /// like Overflow are swallowed, not internal invariant violations which would panic).
    fn scan_and_liquidate_all(&mut self, now_slot: u64, oracle_price: u64) -> u32 {
        let mut count = 0u32;

        for block in 0..BITMAP_WORDS {
            let mut w = self.used[block];
            while w != 0 {
                let bit = w.trailing_zeros() as usize;
                let idx = block * 64 + bit;
                w &= w - 1;
                if idx >= MAX_ACCOUNTS {
                    continue; // Guard against stray high bits in bitmap
                }

                // Best-effort: ignore errors, just count successes
                if let Ok(true) = self.liquidate_at_oracle(idx as u16, now_slot, oracle_price) {
                    count += 1;
                }
            }
        }

        count
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
        let warmed_up_cap = mul_u128(account.warmup_slope_per_step, elapsed_slots as u128);

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
        // Ensure slope >= 1 when positive_pnl > 0 to prevent "zero forever" bug
        let slope = if self.params.warmup_period_slots > 0 {
            let base = positive_pnl / (self.params.warmup_period_slots as u128);
            if positive_pnl > 0 {
                core::cmp::max(1, base)
            } else {
                0
            }
        } else {
            positive_pnl // Instant warmup if period is 0
        };

        // Verify slope >= 1 when positive PnL exists
        #[cfg(any(test, kani))]
        debug_assert!(
            slope >= 1 || positive_pnl == 0,
            "Warmup slope bug: slope {} with positive_pnl {}",
            slope,
            positive_pnl
        );

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
            // Round UP for positive payments (account pays), truncate for negative (account receives)
            // This ensures vault always has at least what's owed (one-sided conservation slack).
            let raw = account
                .position_size
                .checked_mul(delta_f)
                .ok_or(RiskError::Overflow)?;

            let payment = if raw > 0 {
                // Account is paying: round UP to ensure vault gets at least theoretical amount
                raw.checked_add(999_999)
                    .ok_or(RiskError::Overflow)?
                    .checked_div(1_000_000)
                    .ok_or(RiskError::Overflow)?
            } else {
                // Account is receiving: truncate towards zero to give at most theoretical amount
                raw.checked_div(1_000_000).ok_or(RiskError::Overflow)?
            };

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

    /// Full account touch: funding + maintenance fees + warmup settlement.
    /// This is the standard "lazy settlement" path called on every user operation.
    /// Triggers liquidation check if fees push account below maintenance margin.
    pub fn touch_account_full(
        &mut self,
        idx: u16,
        now_slot: u64,
        oracle_price: u64,
    ) -> Result<()> {
        // 1. Settle funding
        self.touch_account(idx)?;

        // 2. Settle maintenance fees (may trigger undercollateralized error)
        self.settle_maintenance_fee(idx, now_slot, oracle_price)?;

        // 3. Settle warmup (convert warmed PnL to capital, realize losses)
        self.settle_warmup_to_capital(idx)?;

        Ok(())
    }

    /// Settle funding for all accounts (ensures funding is zero-sum for conservation checks)
    #[cfg(any(test, feature = "fuzz"))]
    pub fn settle_all_funding(&mut self) -> Result<()> {
        let global_index = self.funding_index_qpb_e6;
        for block in 0..BITMAP_WORDS {
            let mut w = self.used[block];
            while w != 0 {
                let bit = w.trailing_zeros() as usize;
                let idx = block * 64 + bit;
                w &= w - 1;
                if idx >= MAX_ACCOUNTS {
                    continue; // Guard against stray high bits in bitmap
                }

                Self::settle_account_funding(&mut self.accounts[idx], global_index)?;
            }
        }
        Ok(())
    }

    /// Settle funding for all accounts (Kani-specific helper for fast conservation proofs)
    ///
    /// This allows harnesses to settle funding before using the fast conservation check
    /// (conservation_fast_no_funding) which assumes funding is already settled.
    #[cfg(kani)]
    pub fn settle_all_funding_for_kani(&mut self) -> Result<()> {
        let global_index = self.funding_index_qpb_e6;
        for block in 0..BITMAP_WORDS {
            let mut w = self.used[block];
            while w != 0 {
                let bit = w.trailing_zeros() as usize;
                let idx = block * 64 + bit;
                w &= w - 1;
                if idx >= MAX_ACCOUNTS {
                    continue; // Guard against stray high bits in bitmap
                }

                Self::settle_account_funding(&mut self.accounts[idx], global_index)?;
            }
        }
        Ok(())
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

        self.accounts[idx as usize].capital = add_u128(self.accounts[idx as usize].capital, amount);
        self.vault = add_u128(self.vault, amount);

        // Settle warmup after deposit (allows losses to be paid promptly if underwater)
        self.settle_warmup_to_capital(idx)?;

        Ok(())
    }

    /// Withdraw capital from an account.
    /// Relies on Solana transaction atomicity: if this returns Err, the entire TX aborts.
    pub fn withdraw(
        &mut self,
        idx: u16,
        amount: u128,
        now_slot: u64,
        oracle_price: u64,
    ) -> Result<()> {
        // Require fresh crank before state-changing operations
        self.require_fresh_crank(now_slot)?;

        // Withdrawals are neutral in risk mode (allowed)
        self.enforce_op(OpClass::RiskNeutral)?;

        // Validate account exists
        if !self.is_used(idx as usize) {
            return Err(RiskError::AccountNotFound);
        }

        // Full settlement: funding + maintenance fees + warmup
        self.touch_account_full(idx, now_slot, oracle_price)?;

        let account = &self.accounts[idx as usize];

        // Check we have enough capital
        if account.capital < amount {
            return Err(RiskError::InsufficientBalance);
        }

        // Calculate new state after withdrawal
        // FIX B: Use equity (includes negative PnL) for margin checks
        let new_capital = sub_u128(account.capital, amount);
        let cap_i = u128_to_i128_clamped(new_capital);
        let new_eq_i = cap_i.saturating_add(account.pnl);
        let new_equity = if new_eq_i > 0 { new_eq_i as u128 } else { 0 };

        // If account has position, must maintain initial margin
        if account.position_size != 0 {
            let position_notional = mul_u128(
                saturating_abs_i128(account.position_size) as u128,
                account.entry_price as u128,
            ) / 1_000_000;

            let initial_margin_required =
                mul_u128(position_notional, self.params.initial_margin_bps as u128) / 10_000;

            if new_equity < initial_margin_required {
                return Err(RiskError::Undercollateralized);
            }
        }

        // Commit the withdrawal
        self.accounts[idx as usize].capital = new_capital;
        self.vault = sub_u128(self.vault, amount);

        // Regression assert: after settle + withdraw, negative PnL should have been settled
        #[cfg(any(test, kani))]
        debug_assert!(
            self.accounts[idx as usize].pnl >= 0 || self.accounts[idx as usize].capital == 0,
            "Withdraw: negative PnL must settle immediately"
        );

        Ok(())
    }

    // ========================================
    // Trading
    // ========================================

    /// Calculate account's collateral (capital + positive PNL)
    /// NOTE: This is the OLD collateral definition. For margin checks, use account_equity instead.
    pub fn account_collateral(&self, account: &Account) -> u128 {
        add_u128(account.capital, clamp_pos_i128(account.pnl))
    }

    /// Calculate account's equity for margin checks: max(0, capital + pnl)
    /// FIX B: This includes negative PnL in margin calculations.
    #[inline]
    pub fn account_equity(&self, account: &Account) -> u128 {
        let cap_i = u128_to_i128_clamped(account.capital);
        let eq_i = cap_i.saturating_add(account.pnl);
        if eq_i > 0 { eq_i as u128 } else { 0 }
    }

    /// Check if account is above maintenance margin
    /// FIX B: Uses equity (includes negative PnL) instead of collateral
    pub fn is_above_maintenance_margin(&self, account: &Account, oracle_price: u64) -> bool {
        self.is_above_margin_bps(account, oracle_price, self.params.maintenance_margin_bps)
    }

    /// Check if account is above a given margin threshold (in basis points).
    /// Used for both maintenance margin check and post-liquidation target margin check.
    pub fn is_above_margin_bps(&self, account: &Account, oracle_price: u64, bps: u64) -> bool {
        let equity = self.account_equity(account);

        // Calculate position value at current price
        let position_value = mul_u128(
            saturating_abs_i128(account.position_size) as u128,
            oracle_price as u128,
        ) / 1_000_000;

        // Margin requirement at given bps
        let margin_required = mul_u128(position_value, bps as u128) / 10_000;

        equity > margin_required
    }

    /// Risk-reduction-only mode is entered when the system is in deficit. Warmups are frozen so pending PNL cannot become principal. Withdrawals of principal (capital) are allowed (subject to margin). Risk-increasing actions are blocked; only risk-reducing/neutral operations are allowed.
    /// Execute a trade between LP and user.
    /// Relies on Solana transaction atomicity: if this returns Err, the entire TX aborts.
    pub fn execute_trade<M: MatchingEngine>(
        &mut self,
        matcher: &M,
        lp_idx: u16,
        user_idx: u16,
        now_slot: u64,
        oracle_price: u64,
        size: i128,
    ) -> Result<()> {
        // Require fresh crank before state-changing operations
        self.require_fresh_crank(now_slot)?;

        // Validate indices
        if !self.is_used(lp_idx as usize) || !self.is_used(user_idx as usize) {
            return Err(RiskError::AccountNotFound);
        }

        // Validate account kinds
        if self.accounts[lp_idx as usize].kind != AccountKind::LP {
            return Err(RiskError::AccountKindMismatch);
        }
        if self.accounts[user_idx as usize].kind != AccountKind::User {
            return Err(RiskError::AccountKindMismatch);
        }

        // Check if trade increases risk (absolute exposure for either party)
        let old_user_pos = self.accounts[user_idx as usize].position_size;
        let old_lp_pos = self.accounts[lp_idx as usize].position_size;
        let new_user_pos = old_user_pos.saturating_add(size);
        let new_lp_pos = old_lp_pos.saturating_sub(size);

        let user_inc = saturating_abs_i128(new_user_pos) > saturating_abs_i128(old_user_pos);
        let lp_inc = saturating_abs_i128(new_lp_pos) > saturating_abs_i128(old_lp_pos);

        if user_inc || lp_inc {
            self.enforce_op(OpClass::RiskIncrease)?;
        } else {
            self.enforce_op(OpClass::RiskReduce)?;
        }

        // Call matching engine
        let lp = &self.accounts[lp_idx as usize];
        let execution = matcher.execute_match(
            &lp.matcher_program,
            &lp.matcher_context,
            lp.account_id,
            oracle_price,
            size,
        )?;

        // Settle funding and maintenance fees for both accounts (propagate errors)
        // Note: warmup is settled at the END after trade PnL is generated
        self.touch_account(user_idx)?;
        self.touch_account(lp_idx)?;
        self.settle_maintenance_fee(user_idx, now_slot, oracle_price)?;
        self.settle_maintenance_fee(lp_idx, now_slot, oracle_price)?;

        let exec_price = execution.price;
        let exec_size = execution.size;

        // Calculate fee
        let notional =
            mul_u128(saturating_abs_i128(exec_size) as u128, exec_price as u128) / 1_000_000;
        let fee = mul_u128(notional, self.params.trading_fee_bps as u128) / 10_000;

        // Access both accounts
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

            if (old_position > 0 && exec_size < 0) || (old_position < 0 && exec_size > 0) {
                let close_size = core::cmp::min(
                    saturating_abs_i128(old_position),
                    saturating_abs_i128(exec_size),
                );
                let price_diff = if old_position > 0 {
                    (exec_price as i128).saturating_sub(old_entry as i128)
                } else {
                    (old_entry as i128).saturating_sub(exec_price as i128)
                };

                // Use saturating arithmetic (no overflow errors needed with Solana atomicity)
                let pnl = price_diff
                    .saturating_mul(close_size)
                    .saturating_div(1_000_000);
                user_pnl_delta = pnl;
                lp_pnl_delta = -pnl;
            }
        }

        // Calculate new positions
        let new_user_position = user.position_size.saturating_add(exec_size);
        let new_lp_position = lp.position_size.saturating_sub(exec_size);

        // Calculate new entry prices
        let mut new_user_entry = user.entry_price;
        let mut new_lp_entry = lp.entry_price;

        // Update user entry price
        if (user.position_size > 0 && exec_size > 0) || (user.position_size < 0 && exec_size < 0) {
            let old_notional = mul_u128(
                saturating_abs_i128(user.position_size) as u128,
                user.entry_price as u128,
            );
            let new_notional = mul_u128(saturating_abs_i128(exec_size) as u128, exec_price as u128);
            let total_notional = add_u128(old_notional, new_notional);
            let total_size = saturating_abs_i128(user.position_size)
                .saturating_add(saturating_abs_i128(exec_size));
            if total_size != 0 {
                new_user_entry = div_u128(total_notional, total_size as u128)? as u64;
            }
        } else if saturating_abs_i128(user.position_size) < saturating_abs_i128(exec_size) {
            new_user_entry = exec_price;
        }

        // Update LP entry price
        if lp.position_size == 0 {
            new_lp_entry = exec_price;
        } else if (lp.position_size > 0 && new_lp_position > lp.position_size)
            || (lp.position_size < 0 && new_lp_position < lp.position_size)
        {
            let old_notional = mul_u128(
                saturating_abs_i128(lp.position_size) as u128,
                lp.entry_price as u128,
            );
            let new_notional = mul_u128(saturating_abs_i128(exec_size) as u128, exec_price as u128);
            let total_notional = add_u128(old_notional, new_notional);
            let total_size = saturating_abs_i128(lp.position_size)
                .saturating_add(saturating_abs_i128(exec_size));
            if total_size != 0 {
                new_lp_entry = div_u128(total_notional, total_size as u128)? as u64;
            }
        } else if saturating_abs_i128(lp.position_size) < saturating_abs_i128(new_lp_position)
            && ((lp.position_size > 0 && new_lp_position < 0)
                || (lp.position_size < 0 && new_lp_position > 0))
        {
            new_lp_entry = exec_price;
        }

        // Compute final PNL values
        let new_user_pnl = user
            .pnl
            .saturating_add(user_pnl_delta)
            .saturating_sub(fee as i128);
        let new_lp_pnl = lp.pnl.saturating_add(lp_pnl_delta);

        // Check user maintenance margin
        // FIX B: Use equity (includes negative PnL) for margin checks
        if new_user_position != 0 {
            let user_cap_i = u128_to_i128_clamped(user.capital);
            let user_eq_i = user_cap_i.saturating_add(new_user_pnl);
            let user_equity = if user_eq_i > 0 { user_eq_i as u128 } else { 0 };
            let position_value = mul_u128(
                saturating_abs_i128(new_user_position) as u128,
                oracle_price as u128,
            ) / 1_000_000;
            let margin_required =
                mul_u128(position_value, self.params.maintenance_margin_bps as u128) / 10_000;
            if user_equity <= margin_required {
                return Err(RiskError::Undercollateralized);
            }
        }

        // Check LP maintenance margin
        // FIX B: Use equity (includes negative PnL) for margin checks
        if new_lp_position != 0 {
            let lp_cap_i = u128_to_i128_clamped(lp.capital);
            let lp_eq_i = lp_cap_i.saturating_add(new_lp_pnl);
            let lp_equity = if lp_eq_i > 0 { lp_eq_i as u128 } else { 0 };
            let position_value = mul_u128(
                saturating_abs_i128(new_lp_position) as u128,
                oracle_price as u128,
            ) / 1_000_000;
            let margin_required =
                mul_u128(position_value, self.params.maintenance_margin_bps as u128) / 10_000;
            if lp_equity <= margin_required {
                return Err(RiskError::Undercollateralized);
            }
        }

        // Commit all state changes
        self.insurance_fund.fee_revenue = add_u128(self.insurance_fund.fee_revenue, fee);
        self.insurance_fund.balance = add_u128(self.insurance_fund.balance, fee);

        // Credit fee to user's fee_credits (active traders earn credits that offset maintenance)
        user.fee_credits = user.fee_credits.saturating_add(fee as i128);

        user.pnl = new_user_pnl;
        user.position_size = new_user_position;
        user.entry_price = new_user_entry;

        lp.pnl = new_lp_pnl;
        lp.position_size = new_lp_position;
        lp.entry_price = new_lp_entry;

        // Update total open interest tracking (O(1))
        // OI = sum of abs(position_size) across all accounts
        let old_oi = saturating_abs_i128(old_user_pos) as u128
            + saturating_abs_i128(old_lp_pos) as u128;
        let new_oi = saturating_abs_i128(new_user_position) as u128
            + saturating_abs_i128(new_lp_position) as u128;
        if new_oi > old_oi {
            self.total_open_interest = self.total_open_interest.saturating_add(new_oi - old_oi);
        } else {
            self.total_open_interest = self.total_open_interest.saturating_sub(old_oi - new_oi);
        }

        // Update warmup slopes after PNL changes
        self.update_warmup_slope(user_idx)?;
        self.update_warmup_slope(lp_idx)?;

        // Settle warmup for both accounts (at the very end of trade)
        self.settle_warmup_to_capital(user_idx)?;
        self.settle_warmup_to_capital(lp_idx)?;

        Ok(())
    }

    // ========================================
    // ADL (Auto-Deleveraging) - Scan-Based
    // ========================================

    /// Calculate withdrawable PnL for an account (inline helper)
    /// withdrawable = min(available_pnl, warmed_up_cap)
    #[inline]
    fn compute_withdrawable_pnl(&self, account: &Account) -> u128 {
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

        // Calculate warmed capacity
        let elapsed_slots = effective_slot.saturating_sub(account.warmup_started_at_slot);
        let warmed_up_cap = mul_u128(account.warmup_slope_per_step, elapsed_slots as u128);

        core::cmp::min(available_pnl, warmed_up_cap)
    }

    /// Calculate unwrapped PNL for an account (inline helper for ADL)
    /// unwrapped = max(0, positive_pnl - reserved_pnl - withdrawable_pnl)
    /// This is PnL that hasn't yet warmed and isn't reserved - subject to ADL haircuts
    #[inline]
    fn compute_unwrapped_pnl(&self, account: &Account) -> u128 {
        if account.pnl <= 0 {
            return 0;
        }

        let positive_pnl = account.pnl as u128;
        let reserved = account.reserved_pnl;
        let withdrawable = self.compute_withdrawable_pnl(account);

        // unwrapped = positive_pnl - reserved - withdrawable (all saturating)
        positive_pnl
            .saturating_sub(reserved)
            .saturating_sub(withdrawable)
    }

    /// Returns insurance balance above the floor (raw spendable, before reservations)
    #[inline]
    pub fn insurance_spendable_raw(&self) -> u128 {
        let floor = self.params.risk_reduction_threshold;
        if self.insurance_fund.balance > floor {
            self.insurance_fund.balance - floor
        } else {
            0
        }
    }

    /// Returns insurance spendable for ADL and warmup budget (raw - reserved)
    #[inline]
    pub fn insurance_spendable_unreserved(&self) -> u128 {
        self.insurance_spendable_raw()
            .saturating_sub(self.warmup_insurance_reserved)
    }

    /// Returns remaining warmup budget for converting positive PnL to capital
    /// Budget = max(0, warmed_neg_total + unreserved_spendable_insurance - warmed_pos_total)
    #[inline]
    pub fn warmup_budget_remaining(&self) -> u128 {
        let rhs = self
            .warmed_neg_total
            .saturating_add(self.insurance_spendable_unreserved());
        rhs.saturating_sub(self.warmed_pos_total)
    }

    /// Recompute warmup_insurance_reserved from current W+, W-, and insurance.
    /// Must be called after any operation that changes insurance or W+/W-.
    /// Formula: reserved = min(max(W+ - W-, 0), raw_spendable)
    #[inline]
    pub fn recompute_warmup_insurance_reserved(&mut self) {
        let raw = self.insurance_spendable_raw();
        let needed = self.warmed_pos_total.saturating_sub(self.warmed_neg_total);
        self.warmup_insurance_reserved = core::cmp::min(needed, raw);
    }

    /// Settle warmup: convert PnL to capital with global budget constraint
    ///
    /// This function settles matured PnL into capital:
    /// - Negative PnL: reduces capital (losses paid from principal)
    /// - Positive PnL: increases capital (profits become principal, clamped by budget)
    ///
    /// The warmup budget invariant ensures:
    ///   warmed_pos_total <= warmed_neg_total + insurance_spendable_unreserved()
    pub fn settle_warmup_to_capital(&mut self, idx: u16) -> Result<()> {
        if !self.is_used(idx as usize) {
            return Err(RiskError::AccountNotFound);
        }

        // 3.1 Compute per-account warmup capacity with pause semantics
        let effective_slot = if self.warmup_paused {
            core::cmp::min(self.current_slot, self.warmup_pause_slot)
        } else {
            self.current_slot
        };

        let started_at = self.accounts[idx as usize].warmup_started_at_slot;
        let elapsed = effective_slot.saturating_sub(started_at);
        let slope = self.accounts[idx as usize].warmup_slope_per_step;
        let cap = mul_u128(slope, elapsed as u128);

        // 3.2 Settle losses IMMEDIATELY (negative PnL → reduce capital)
        // FIX A: Negative PnL is not time-gated by warmup slope - it settles fully and immediately.
        // pay = min(capital, -pnl)
        let pnl = self.accounts[idx as usize].pnl;
        if pnl < 0 {
            let need = neg_i128_to_u128(pnl);
            let capital = self.accounts[idx as usize].capital;
            let pay = core::cmp::min(need, capital);

            if pay > 0 {
                self.accounts[idx as usize].pnl =
                    self.accounts[idx as usize].pnl.saturating_add(pay as i128);
                self.accounts[idx as usize].capital = sub_u128(capital, pay);
                self.warmed_neg_total = add_u128(self.warmed_neg_total, pay);
            }

            // After immediate settlement: pnl < 0 only if capital was exhausted
            #[cfg(any(test, kani))]
            debug_assert!(
                self.accounts[idx as usize].pnl >= 0 || self.accounts[idx as usize].capital == 0,
                "Negative PnL must settle immediately: pnl < 0 implies capital == 0"
            );
        }

        // 3.3 Budget from losses (currently unused but documents the design)
        let _losses_budget = self.warmed_neg_total.saturating_sub(self.warmed_pos_total);

        // 3.4 Settle gains with budget clamp (positive PnL → increase capital)
        let pnl = self.accounts[idx as usize].pnl;
        if pnl > 0 && cap > 0 {
            let positive_pnl = pnl as u128;
            let reserved = self.accounts[idx as usize].reserved_pnl;
            let avail = positive_pnl.saturating_sub(reserved);

            if avail > 0 {
                let budget = self.warmup_budget_remaining();
                let x = core::cmp::min(cap, core::cmp::min(avail, budget));

                if x > 0 {
                    self.accounts[idx as usize].pnl =
                        self.accounts[idx as usize].pnl.saturating_sub(x as i128);
                    self.accounts[idx as usize].capital =
                        add_u128(self.accounts[idx as usize].capital, x);
                    self.warmed_pos_total = add_u128(self.warmed_pos_total, x);
                }
            }
        }

        // 3.5 Always advance start marker to prevent double-settling the same matured amount.
        // This is safe even when paused: effective_slot==warmup_pause_slot, so further elapsed==0.
        self.accounts[idx as usize].warmup_started_at_slot = effective_slot;

        // 3.6 Recompute reserved (W+ or W- may have changed)
        self.recompute_warmup_insurance_reserved();

        // 3.7 Hard invariant assert in debug/kani
        // W+ ≤ W- + raw_spendable (reserved insurance backs warmed profits)
        // Reserved equality: reserved == min(max(W+ - W-, 0), raw)
        // Also: insurance >= floor + reserved (reserved portion protected)
        #[cfg(any(test, kani))]
        {
            let raw = self.insurance_spendable_raw();
            let floor = self.params.risk_reduction_threshold;
            let needed = self.warmed_pos_total.saturating_sub(self.warmed_neg_total);
            let expect_reserved = core::cmp::min(needed, raw);
            debug_assert!(
                self.warmed_pos_total <= self.warmed_neg_total.saturating_add(raw),
                "Warmup budget invariant violated: W+ > W- + raw"
            );
            debug_assert!(
                self.warmup_insurance_reserved == expect_reserved,
                "Reserved equality invariant violated: {} != {}",
                self.warmup_insurance_reserved,
                expect_reserved
            );
            debug_assert!(
                self.insurance_fund.balance >= floor.saturating_add(self.warmup_insurance_reserved),
                "Insurance fell below floor+reserved"
            );
        }

        Ok(())
    }

    /// Apply ADL haircut using two-pass bitmap scan (stack-safe, no caching)
    ///
    /// Pass 1: Compute total unwrapped PNL across all accounts
    /// Pass 2: Recompute each account's unwrapped PNL and apply proportional haircut
    ///
    /// Waterfall: unwrapped PNL first, then insurance fund, then loss_accum
    ///
    /// Uses largest-remainder method for exact haircut distribution:
    /// 1. Compute haircut = (loss * unwrapped) / total for each account
    /// 2. Track remainder = (loss * unwrapped) % total for each account
    /// 3. Distribute leftover units to accounts with largest remainder (ties: lowest idx)
    pub fn apply_adl(&mut self, total_loss: u128) -> Result<()> {
        self.apply_adl_impl(total_loss, None)
    }

    /// ADL variant that excludes a specific account from being haircutted.
    ///
    /// Used when funding liquidation profit (mark_pnl > 0) - the liquidated account
    /// should not fund its own profit via ADL. This ensures profits are backed by
    /// other accounts' unwrapped PnL, insurance, or loss_accum.
    pub fn apply_adl_excluding(&mut self, total_loss: u128, exclude_idx: usize) -> Result<()> {
        self.apply_adl_impl(total_loss, Some(exclude_idx))
    }

    /// Core ADL implementation with optional account exclusion.
    ///
    /// When `exclude` is Some(idx), that account is skipped during haircutting.
    /// This prevents liquidated winners from funding their own profit.
    fn apply_adl_impl(&mut self, total_loss: u128, exclude: Option<usize>) -> Result<()> {
        // ADL reduces risk (allowed in risk mode)
        self.enforce_op(OpClass::RiskReduce)?;

        if total_loss == 0 {
            return Ok(());
        }

        // Inline helper - simpler for Kani than a closure
        #[inline]
        fn is_excluded(exclude: Option<usize>, idx: usize) -> bool {
            match exclude {
                Some(ex) => ex == idx,
                None => false,
            }
        }

        // Pass 1: Compute total unwrapped PNL (excluding specified account if any)
        let mut total_unwrapped = 0u128;

        for block in 0..BITMAP_WORDS {
            let mut w = self.used[block];
            while w != 0 {
                let bit = w.trailing_zeros() as usize;
                let idx = block * 64 + bit;
                w &= w - 1;
                if idx >= MAX_ACCOUNTS {
                    continue; // Guard against stray high bits in bitmap
                }
                if is_excluded(exclude, idx) {
                    continue;
                }

                let unwrapped = self.compute_unwrapped_pnl(&self.accounts[idx]);
                total_unwrapped = total_unwrapped.saturating_add(unwrapped);
            }
        }

        // Determine how much loss can be socialized via unwrapped PNL
        let loss_to_socialize = core::cmp::min(total_loss, total_unwrapped);

        // Track total applied for conservation
        let mut applied_from_pnl: u128 = 0;

        if loss_to_socialize > 0 && total_unwrapped > 0 {
            // Step 1: Zero scratch arrays for used accounts only (via bitmap)
            for block in 0..BITMAP_WORDS {
                let mut w = self.used[block];
                while w != 0 {
                    let bit = w.trailing_zeros() as usize;
                    let idx = block * 64 + bit;
                    w &= w - 1;
                    if idx >= MAX_ACCOUNTS {
                        continue;
                    }
                    if is_excluded(exclude, idx) {
                        continue;
                    }

                    self.adl_remainder_scratch[idx] = 0;
                    self.adl_eligible_scratch[idx] = 0;
                }
            }

            // Step 2: Compute floor haircuts, store remainders, mark eligible
            for block in 0..BITMAP_WORDS {
                let mut w = self.used[block];
                while w != 0 {
                    let bit = w.trailing_zeros() as usize;
                    let idx = block * 64 + bit;
                    w &= w - 1;
                    if idx >= MAX_ACCOUNTS {
                        continue;
                    }
                    if is_excluded(exclude, idx) {
                        continue;
                    }

                    let account = &self.accounts[idx];
                    if account.pnl > 0 {
                        let unwrapped = self.compute_unwrapped_pnl(account);

                        if unwrapped > 0 {
                            let numer = loss_to_socialize.saturating_mul(unwrapped);
                            let haircut = numer / total_unwrapped;
                            let rem = numer % total_unwrapped;

                            self.accounts[idx].pnl =
                                self.accounts[idx].pnl.saturating_sub(haircut as i128);
                            applied_from_pnl += haircut;

                            // Store remainder and mark eligible if rem > 0
                            self.adl_remainder_scratch[idx] = rem;
                            self.adl_eligible_scratch[idx] = if rem > 0 { 1 } else { 0 };
                        }
                    }
                }
            }

            // Step 3: Distribute leftover using largest-remainder method
            // Each account can receive at most +1 leftover (correct for largest-remainder)
            // Tighter bound: min(leftover, MAX_ACCOUNTS) reduces Kani unwind work
            let mut leftover = loss_to_socialize - applied_from_pnl;
            let max_iters = core::cmp::min(leftover as usize, MAX_ACCOUNTS);

            for _ in 0..max_iters {
                if leftover == 0 {
                    break;
                }

                // Find account with max remainder; ties: lowest idx wins
                let mut best_idx: Option<usize> = None;
                let mut best_rem: u128 = 0;

                for block in 0..BITMAP_WORDS {
                    let mut w = self.used[block];
                    while w != 0 {
                        let bit = w.trailing_zeros() as usize;
                        let idx = block * 64 + bit;
                        w &= w - 1;
                        if idx >= MAX_ACCOUNTS {
                            continue;
                        }
                        if is_excluded(exclude, idx) {
                            continue;
                        }

                        // Only consider eligible accounts (don't gate on pnl > 0)
                        if self.adl_eligible_scratch[idx] == 1 {
                            let rem = self.adl_remainder_scratch[idx];
                            // Skip zero remainders for robust selection
                            if rem == 0 {
                                continue;
                            }
                            // Prefer larger remainder; if equal, prefer smaller idx (ties)
                            if rem > best_rem || (rem == best_rem && best_idx.map_or(true, |b| idx < b)) {
                                best_rem = rem;
                                best_idx = Some(idx);
                            }
                        }
                    }
                }

                match best_idx {
                    Some(idx) => {
                        self.accounts[idx].pnl = self.accounts[idx].pnl.saturating_sub(1);
                        applied_from_pnl += 1;
                        self.adl_eligible_scratch[idx] = 0;
                        leftover -= 1;
                    }
                    None => {
                        break;
                    }
                }
            }

            // Hygiene: verify all eligible bits consumed after distribution
            #[cfg(any(test, kani))]
            {
                for block in 0..BITMAP_WORDS {
                    let mut w = self.used[block];
                    while w != 0 {
                        let bit = w.trailing_zeros() as usize;
                        let idx = block * 64 + bit;
                        w &= w - 1;
                        if idx >= MAX_ACCOUNTS {
                            continue;
                        }
                        if is_excluded(exclude, idx) {
                            continue;
                        }
                        debug_assert!(
                            self.adl_eligible_scratch[idx] == 0,
                            "Eligible bit not consumed for account {}",
                            idx
                        );
                    }
                }
            }
        }

        // Verify exact socialization in test/kani builds
        #[cfg(any(test, kani))]
        debug_assert!(
            applied_from_pnl == loss_to_socialize,
            "ADL rounding bug: applied {} != socialized {}",
            applied_from_pnl,
            loss_to_socialize
        );

        // Handle remaining loss with insurance fund (respecting floor)
        let remaining_loss = total_loss.saturating_sub(applied_from_pnl);

        if remaining_loss > 0 {
            // Insurance can only spend unreserved amount above the floor
            let spendable = self.insurance_spendable_unreserved();
            let spend = core::cmp::min(remaining_loss, spendable);

            // Deduct from insurance fund
            self.insurance_fund.balance = sub_u128(self.insurance_fund.balance, spend);

            // Any remaining loss goes to loss_accum
            let uncovered = remaining_loss.saturating_sub(spend);
            if uncovered > 0 {
                self.loss_accum = add_u128(self.loss_accum, uncovered);
            }

            // Enter risk-reduction-only mode if we've hit the floor or have uncovered losses
            if uncovered > 0 || self.insurance_fund.balance <= self.params.risk_reduction_threshold
            {
                self.enter_risk_reduction_only_mode();
            }
        }

        // Recompute reserved since insurance may have changed
        self.recompute_warmup_insurance_reserved();

        // Assert reserved equality invariant in test/kani
        #[cfg(any(test, kani))]
        {
            let raw = self.insurance_spendable_raw();
            let needed = self.warmed_pos_total.saturating_sub(self.warmed_neg_total);
            let expect_reserved = core::cmp::min(needed, raw);
            debug_assert!(
                self.warmup_insurance_reserved == expect_reserved,
                "Reserved invariant violated in apply_adl"
            );
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
    /// Unlike single-account liquidation, global settlement requires multi-phase
    /// processing so ADL can see the full picture of positive PnL before haircutting.
    pub fn panic_settle_all(&mut self, oracle_price: u64) -> Result<()> {
        // Panic settle is a risk-reducing operation
        self.enforce_op(OpClass::RiskReduce)?;

        // Always enter risk-reduction-only mode (freezes warmups)
        self.enter_risk_reduction_only_mode();

        // Accumulate total system loss from negative PNL after settlement
        let mut total_loss = 0u128;
        // Track sum of mark PNL to compensate for integer division rounding
        let mut total_mark_pnl: i128 = 0;

        // Phase 1: settle funding, apply mark PnL, close positions, clamp negative PnL
        let global_funding_index = self.funding_index_qpb_e6;
        for block in 0..BITMAP_WORDS {
            let mut w = self.used[block];
            while w != 0 {
                let bit = w.trailing_zeros() as usize;
                let idx = block * 64 + bit;
                w &= w - 1;

                if idx >= MAX_ACCOUNTS {
                    continue; // Guard against stray high bits in bitmap
                }

                let account = &mut self.accounts[idx];

                // Settle funding first (required for correct PNL accounting)
                Self::settle_account_funding(account, global_funding_index)?;

                // Skip accounts with no position
                if account.position_size == 0 {
                    continue;
                }

                // Compute mark PNL at oracle price
                let pos = account.position_size;
                let abs_pos = saturating_abs_i128(pos) as u128;
                let mark_pnl = Self::mark_pnl_for_position(pos, account.entry_price, oracle_price)?;

                // Track total mark PNL for rounding compensation
                total_mark_pnl = total_mark_pnl.saturating_add(mark_pnl);

                // Apply mark PNL to account
                account.pnl = account.pnl.saturating_add(mark_pnl);

                // Close position
                account.position_size = 0;
                account.entry_price = oracle_price;

                // Update OI
                self.total_open_interest = self.total_open_interest.saturating_sub(abs_pos);

                // Clamp negative PNL and accumulate system loss
                if account.pnl < 0 {
                    let loss = neg_i128_to_u128(account.pnl);
                    total_loss = total_loss.saturating_add(loss);
                    account.pnl = 0;
                }
            }
        }

        // Compensate for non-zero-sum mark PNL from rounding.
        // If positive: treat as additional loss to socialize via ADL
        // If negative: absorbed by bounded conservation slack (don't mint to insurance)
        if total_mark_pnl > 0 {
            total_loss = total_loss.saturating_add(total_mark_pnl as u128);
        }

        // Phase 2: Socialize accumulated loss via ADL waterfall
        // All accounts now have their mark_pnl applied, so ADL can haircut properly
        if total_loss > 0 {
            self.apply_adl(total_loss)?;
        }

        // Phase 3: Settle warmup for all accounts (after ADL haircuts)
        for block in 0..BITMAP_WORDS {
            let mut w = self.used[block];
            while w != 0 {
                let bit = w.trailing_zeros() as usize;
                let idx = block * 64 + bit;
                w &= w - 1;

                if idx >= MAX_ACCOUNTS {
                    continue; // Guard against stray high bits in bitmap
                }

                self.settle_warmup_to_capital(idx as u16)?;
            }
        }

        // Recompute reserved after all operations
        self.recompute_warmup_insurance_reserved();

        // Assert reserved equality invariant in test/kani
        #[cfg(any(test, kani))]
        {
            let raw = self.insurance_spendable_raw();
            let needed = self.warmed_pos_total.saturating_sub(self.warmed_neg_total);
            let expect_reserved = core::cmp::min(needed, raw);
            debug_assert!(
                self.warmup_insurance_reserved == expect_reserved,
                "Reserved invariant violated in panic_settle_all"
            );
        }

        Ok(())
    }

    /// Force realize losses to unstick the exchange at insurance floor
    ///
    /// When insurance is at/below the threshold, the exchange can get "stuck"
    /// because positive PnL cannot warm (no budget). This instruction forces
    /// loss realization which increases warmed_neg_total, creating budget for
    /// positive PnL to warm and withdrawals to proceed.
    ///
    /// This instruction:
    /// 1. Requires insurance_fund.balance <= risk_reduction_threshold
    /// 2. Enters risk_reduction_only mode and freezes warmup
    /// 3. Scans all accounts with positions and realizes mark PnL at oracle_price
    /// 4. For losers: pays losses from capital, incrementing warmed_neg_total
    /// 5. Does NOT warm any positive PnL (keeps it young, subject to ADL)
    /// 6. Unpaid losses (capital exhausted) go through apply_adl waterfall
    ///
    /// Like panic_settle_all, uses multi-phase processing so ADL can see full picture.
    pub fn force_realize_losses(&mut self, oracle_price: u64) -> Result<()> {
        // Force realize is a risk-reducing operation
        self.enforce_op(OpClass::RiskReduce)?;

        // Gate: only allowed when insurance is at or below floor
        if self.insurance_fund.balance > self.params.risk_reduction_threshold {
            return Err(RiskError::Unauthorized);
        }

        // Enter risk-reduction-only mode (freezes warmups)
        self.enter_risk_reduction_only_mode();

        // Accumulate unpaid losses (when capital is exhausted)
        let mut total_unpaid_loss = 0u128;
        // Track sum of mark PNL to compensate for integer division rounding
        let mut total_mark_pnl: i128 = 0;

        // Phase 1: settle funding, apply mark PnL, close positions, pay losses from capital
        let global_funding_index = self.funding_index_qpb_e6;
        for block in 0..BITMAP_WORDS {
            let mut w = self.used[block];
            while w != 0 {
                let bit = w.trailing_zeros() as usize;
                let idx = block * 64 + bit;
                w &= w - 1;

                if idx >= MAX_ACCOUNTS {
                    continue; // Guard against stray high bits in bitmap
                }

                let account = &mut self.accounts[idx];

                // Settle funding first (required for correct PNL accounting)
                Self::settle_account_funding(account, global_funding_index)?;

                // Skip accounts with no position
                if account.position_size == 0 {
                    continue;
                }

                // Compute mark PNL at oracle price
                let pos = account.position_size;
                let abs_pos = saturating_abs_i128(pos) as u128;
                let mark_pnl = Self::mark_pnl_for_position(pos, account.entry_price, oracle_price)?;

                // Track total mark PNL for rounding compensation
                total_mark_pnl = total_mark_pnl.saturating_add(mark_pnl);

                // Apply mark PNL to account
                account.pnl = account.pnl.saturating_add(mark_pnl);

                // Close position
                account.position_size = 0;
                account.entry_price = oracle_price;

                // Update OI
                self.total_open_interest = self.total_open_interest.saturating_sub(abs_pos);

                // Force settle losses only (not positive PnL)
                if account.pnl < 0 {
                    let need = neg_i128_to_u128(account.pnl);
                    let pay = core::cmp::min(need, account.capital);

                    // Pay from capital
                    account.capital = sub_u128(account.capital, pay);
                    account.pnl = account.pnl.saturating_add(pay as i128); // toward 0

                    // Track in warmed_neg_total (losses realized)
                    self.warmed_neg_total = add_u128(self.warmed_neg_total, pay);

                    // Accumulate unpaid portion (capital exhausted)
                    if need > pay {
                        let unpaid = need - pay;
                        total_unpaid_loss = total_unpaid_loss.saturating_add(unpaid);
                        // Clamp remaining negative PnL to zero
                        account.pnl = 0;
                    }
                }
                // Positive PnL is left as-is (young, subject to ADL, warmup frozen)

                // Update warmup start marker
                let effective_slot = core::cmp::min(self.current_slot, self.warmup_pause_slot);
                account.warmup_started_at_slot = effective_slot;
            }
        }

        // Compensate for non-zero-sum mark PNL from rounding.
        // If positive: treat as additional loss to socialize via ADL
        // If negative: absorbed by bounded conservation slack (don't mint to insurance)
        if total_mark_pnl > 0 {
            total_unpaid_loss = total_unpaid_loss.saturating_add(total_mark_pnl as u128);
        }

        // Phase 2: Socialize any unpaid losses via ADL waterfall
        // All accounts now have their mark_pnl applied, so ADL can haircut properly
        if total_unpaid_loss > 0 {
            self.apply_adl(total_unpaid_loss)?;
        }

        // Recompute reserved after all operations (W- and insurance may have changed)
        self.recompute_warmup_insurance_reserved();

        // Assert reserved equality invariant in test/kani
        #[cfg(any(test, kani))]
        {
            let raw = self.insurance_spendable_raw();
            let needed = self.warmed_pos_total.saturating_sub(self.warmed_neg_total);
            let expect_reserved = core::cmp::min(needed, raw);
            debug_assert!(
                self.warmup_insurance_reserved == expect_reserved,
                "Reserved invariant violated in force_realize_losses"
            );
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

            // Recompute reserved after insurance increase
            self.recompute_warmup_insurance_reserved();

            // Assert reserved equality invariant in test/kani
            #[cfg(any(test, kani))]
            {
                let raw = self.insurance_spendable_raw();
                let needed = self.warmed_pos_total.saturating_sub(self.warmed_neg_total);
                let expect_reserved = core::cmp::min(needed, raw);
                debug_assert!(
                    self.warmup_insurance_reserved == expect_reserved,
                    "Reserved invariant violated in top_up_insurance_fund (loss branch)"
                );
            }

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

            // Recompute reserved after insurance increase
            self.recompute_warmup_insurance_reserved();

            // Assert reserved equality invariant in test/kani
            #[cfg(any(test, kani))]
            {
                let raw = self.insurance_spendable_raw();
                let needed = self.warmed_pos_total.saturating_sub(self.warmed_neg_total);
                let expect_reserved = core::cmp::min(needed, raw);
                debug_assert!(
                    self.warmup_insurance_reserved == expect_reserved,
                    "Reserved invariant violated in top_up_insurance_fund (no-loss branch)"
                );
            }

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
    ///
    /// # Rounding Slack
    ///
    /// We require `actual >= expected` (vault has at least what is owed) and
    /// `(actual - expected) <= MAX_ROUNDING_SLACK` (bounded dust). Funding payments
    /// are rounded UP when accounts pay, ensuring the vault never has less than
    /// what's owed. The bounded dust check catches accidental minting bugs.
    pub fn check_conservation(&self) -> bool {
        let mut total_capital = 0u128;
        let mut net_pnl: i128 = 0;
        let global_index = self.funding_index_qpb_e6;

        self.for_each_used(|_idx, account| {
            total_capital = add_u128(total_capital, account.capital);

            // Compute "would-be settled" PNL for this account
            // This accounts for lazy funding settlement with same rounding as settle_account_funding
            let mut settled_pnl = account.pnl;
            if account.position_size != 0 {
                let delta_f = global_index.saturating_sub(account.funding_index);
                if delta_f != 0 {
                    // payment = position × ΔF / 1e6
                    // Round UP for positive (account pays), truncate for negative (account receives)
                    let raw = account.position_size.saturating_mul(delta_f);
                    let payment = if raw > 0 {
                        raw.saturating_add(999_999).saturating_div(1_000_000)
                    } else {
                        raw.saturating_div(1_000_000)
                    };
                    settled_pnl = settled_pnl.saturating_sub(payment);
                }
            }
            net_pnl = net_pnl.saturating_add(settled_pnl);
        });

        // Conservation formula:
        // vault + loss_accum >= sum(capital) + sum(settled_pnl) + insurance
        //
        // Where:
        // - loss_accum: value that "left" the system (unrecoverable losses)
        // - settled_pnl: pnl after accounting for unsettled funding
        //
        // Funding payments are rounded UP when accounts pay, so the vault always has
        // at least what's owed. The slack (dust) is bounded by MAX_ROUNDING_SLACK.
        let base = add_u128(total_capital, self.insurance_fund.balance);

        let expected = if net_pnl >= 0 {
            add_u128(base, net_pnl as u128)
        } else {
            base.saturating_sub(neg_i128_to_u128(net_pnl))
        };

        let actual = add_u128(self.vault, self.loss_accum);

        // One-sided conservation check:
        // actual >= expected (vault has at least what is owed)
        // (actual - expected) <= MAX_ROUNDING_SLACK (bounded dust)
        if actual < expected {
            return false;
        }
        let slack = actual - expected;
        slack <= MAX_ROUNDING_SLACK
    }

    /// Advance to next slot (for testing warmup)
    pub fn advance_slot(&mut self, slots: u64) {
        self.current_slot = self.current_slot.saturating_add(slots);
    }
}
