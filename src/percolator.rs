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
/// Mask for wrapping indices (MAX_ACCOUNTS must be power of 2)
const ACCOUNT_IDX_MASK: usize = MAX_ACCOUNTS - 1;

/// Maximum number of dust accounts to close per crank call.
/// Limits compute usage while still making progress on cleanup.
pub const GC_CLOSE_BUDGET: u32 = 32;

/// Number of crank steps to complete a full sweep (16 steps × 256 = 4096)
pub const NUM_STEPS: u8 = 16;

/// Accounts scanned per crank step in the deterministic sweep
pub const WINDOW: usize = 256;

/// Hard liquidation budget per crank call (caps total work)
pub const LIQ_BUDGET_PER_CRANK: u16 = 128;

/// Max number of force-realize closes per crank call.
/// Hard CU bound in force-realize mode. Liquidations are skipped when active.
pub const FORCE_REALIZE_BUDGET_PER_CRANK: u16 = 32;

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

/// Deferred ADL result from liquidation (internal, for batched ADL).
/// Instead of calling ADL immediately during liquidation, we collect
/// these totals and run 0-2 batched ADL passes after the window scan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DeferredAdl {
    /// Sum of mark_pnl > 0 that needs funding via ADL (excluding this account)
    profit_to_fund: u128,
    /// Residual negative PnL after capital settlement (needs socialization)
    unpaid_loss: u128,
    /// True if this account had profit_to_fund > 0 (should be excluded from profit ADL)
    excluded: bool,
}

impl DeferredAdl {
    const ZERO: Self = Self {
        profit_to_fund: 0,
        unpaid_loss: 0,
        excluded: false,
    };
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

    /// Scratch: sorted index list for ADL remainder distribution.
    /// Used to avoid O(n²) largest-remainder selection.
    pub adl_idx_scratch: [u16; MAX_ACCOUNTS],

    /// Scratch: per-account exclusion flags for batched ADL during liquidation.
    /// Set to 1 for accounts that should be excluded from profit-funding ADL pass.
    /// Only meaningful for indices visited in current window; cleared per-window.
    pub adl_exclude_scratch: [u8; MAX_ACCOUNTS],

    // ========================================
    // Deferred Socialization Buckets (replaces global ADL)
    // ========================================
    /// Accumulated profit-funding needs from liquidations (mark_pnl > 0)
    pub pending_profit_to_fund: u128,

    /// Accumulated unpaid losses from liquidations (capital exhausted)
    pub pending_unpaid_loss: u128,

    /// Epoch for exclusion deduplication (increments each sweep start)
    pub pending_epoch: u8,

    /// Per-account exclusion epoch marker for profit-funding
    /// If pending_exclude_epoch[idx] == pending_epoch, exclude from paying own profit
    pub pending_exclude_epoch: [u8; MAX_ACCOUNTS],

    // ========================================
    // Crank Cursors (bounded scan support)
    // ========================================
    /// Cursor for liquidation scan (wraps around MAX_ACCOUNTS)
    pub liq_cursor: u16,

    /// Cursor for garbage collection scan (wraps around MAX_ACCOUNTS)
    pub gc_cursor: u16,

    /// Slot when the current full sweep started (step 0 was executed)
    pub last_full_sweep_start_slot: u64,

    /// Slot when the last full sweep completed (step 7 finished)
    pub last_full_sweep_completed_slot: u64,

    /// Crank step within current sweep (0..7)
    pub crank_step: u8,

    // ========================================
    // Lifetime Counters (telemetry)
    // ========================================
    /// Total number of liquidations performed (lifetime)
    pub lifetime_liquidations: u64,

    /// Total number of force-realize closes performed (lifetime)
    pub lifetime_force_realize_closes: u64,

    // ========================================
    // LP Aggregates (O(1) maintained for funding/threshold)
    // ========================================
    /// Net LP position: sum of position_size across all LP accounts
    /// Updated incrementally in execute_trade and close paths
    pub net_lp_pos: i128,

    /// Sum of abs(position_size) across all LP accounts
    /// Updated incrementally in execute_trade and close paths
    pub lp_sum_abs: u128,

    /// Max abs(position_size) across all LP accounts (monotone upper bound)
    /// Only increases; reset via bounded sweep at sweep completion
    pub lp_max_abs: u128,

    /// In-progress max abs for current sweep (reset at sweep start, committed at completion)
    pub lp_max_abs_sweep: u128,

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
    /// Whether force-realize mode is active (insurance at/below threshold)
    pub force_realize_needed: bool,
    /// Whether panic_settle_all should be called (system in stress)
    pub panic_needed: bool,
    /// Number of accounts liquidated during this crank
    pub num_liquidations: u32,
    /// Number of liquidation errors (triggers risk_reduction_only)
    pub num_liq_errors: u16,
    /// Number of dust accounts garbage collected during this crank
    pub num_gc_closed: u32,
    /// Number of positions force-closed during this crank (when force_realize_needed)
    pub force_realize_closed: u16,
    /// Number of force-realize errors during this crank
    pub force_realize_errors: u16,
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
            adl_idx_scratch: [0; MAX_ACCOUNTS],
            adl_exclude_scratch: [0; MAX_ACCOUNTS],
            pending_profit_to_fund: 0,
            pending_unpaid_loss: 0,
            pending_epoch: 0,
            pending_exclude_epoch: [0; MAX_ACCOUNTS],
            liq_cursor: 0,
            gc_cursor: 0,
            last_full_sweep_start_slot: 0,
            last_full_sweep_completed_slot: 0,
            crank_step: 0,
            lifetime_liquidations: 0,
            lifetime_force_realize_closes: 0,
            net_lp_pos: 0,
            lp_sum_abs: 0,
            lp_max_abs: 0,
            lp_max_abs_sweep: 0,
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

        // Check maintenance margin if account has a position (MTM check)
        if account.position_size != 0 {
            // Re-borrow immutably for margin check
            let account_ref = &self.accounts[idx as usize];
            if !self.is_above_maintenance_margin_mtm(account_ref, oracle_price)? {
                return Err(RiskError::Undercollateralized);
            }
        }

        Ok(due) // Return fee due for keeper rebate calculation
    }

    /// Best-effort maintenance settle for crank paths.
    /// - Always advances last_fee_slot
    /// - Charges fees into insurance if possible
    /// - NEVER fails due to margin checks
    /// - Still returns Unauthorized if idx invalid
    fn settle_maintenance_fee_best_effort_for_crank(
        &mut self,
        idx: u16,
        now_slot: u64,
    ) -> Result<u128> {
        if idx as usize >= MAX_ACCOUNTS || !self.is_used(idx as usize) {
            return Err(RiskError::Unauthorized);
        }

        let account = &mut self.accounts[idx as usize];

        let dt = now_slot.saturating_sub(account.last_fee_slot);
        if dt == 0 {
            return Ok(0);
        }

        let due = self.params.maintenance_fee_per_slot.saturating_mul(dt as u128);

        // Advance slot marker regardless
        account.last_fee_slot = now_slot;

        // Deduct from fee_credits first
        account.fee_credits = account.fee_credits.saturating_sub(due as i128);

        // If negative, pay what we can from capital (no margin check)
        if account.fee_credits < 0 {
            let owed = neg_i128_to_u128(account.fee_credits);
            let pay = core::cmp::min(owed, account.capital);

            account.capital = account.capital.saturating_sub(pay);
            self.insurance_fund.balance = self.insurance_fund.balance.saturating_add(pay);
            self.insurance_fund.fee_revenue = self.insurance_fund.fee_revenue.saturating_add(pay);

            account.fee_credits = account.fee_credits.saturating_add(pay as i128);
        }

        Ok(due)
    }

    /// Touch account for force-realize paths: settles funding and fees but
    /// uses best-effort fee settle that can't stall on margin checks.
    fn touch_account_for_force_realize(&mut self, idx: u16, now_slot: u64) -> Result<()> {
        // Funding settle is required for correct pnl
        self.touch_account(idx)?;
        // Best-effort fees; never fails due to maintenance margin
        let _ = self.settle_maintenance_fee_best_effort_for_crank(idx, now_slot)?;
        Ok(())
    }

    /// Touch account for liquidation paths: settles funding and fees but
    /// uses best-effort fee settle since we're about to liquidate anyway.
    fn touch_account_for_liquidation(&mut self, idx: u16, now_slot: u64) -> Result<()> {
        // Funding settle is required for correct pnl
        self.touch_account(idx)?;
        // Best-effort fees; margin check would just block the liquidation we need to do
        let _ = self.settle_maintenance_fee_best_effort_for_crank(idx, now_slot)?;
        Ok(())
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
    /// - pnl must be 0 after settlement (positive pnl must be warmed up first)
    ///
    /// Returns Err(PnlNotWarmedUp) if pnl > 0 (user must wait for warmup).
    /// Returns Err(Undercollateralized) if pnl < 0 (shouldn't happen after settlement).
    /// Returns the capital amount on success.
    pub fn close_account(
        &mut self,
        idx: u16,
        now_slot: u64,
        oracle_price: u64,
    ) -> Result<u128> {
        if idx as usize >= MAX_ACCOUNTS || !self.is_used(idx as usize) {
            return Err(RiskError::AccountNotFound);
        }

        // Block closing accounts while socialization debt is pending
        // This prevents extracting capital "through the side" while debt exists
        self.require_no_pending_socialization()?;

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

        // PnL must be zero to close. This enforces:
        // 1. Users can't bypass warmup by closing with positive unwarmed pnl
        // 2. Conservation is maintained (forfeiting pnl would create unbounded slack)
        // 3. Negative pnl after full settlement implies insolvency
        if account.pnl > 0 {
            return Err(RiskError::PnlNotWarmedUp);
        }
        if account.pnl < 0 {
            return Err(RiskError::Undercollateralized);
        }

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
        // Collect dust candidates: accounts with zero position, capital, reserved, and non-positive pnl
        let mut to_free: [u16; GC_CLOSE_BUDGET as usize] = [0; GC_CLOSE_BUDGET as usize];
        let mut num_to_free = 0usize;

        // Fixed WINDOW per crank step, capped to MAX_ACCOUNTS to avoid wrap-around
        let max_scan = if WINDOW < MAX_ACCOUNTS { WINDOW } else { MAX_ACCOUNTS };
        let start = self.gc_cursor as usize;

        for offset in 0..max_scan {
            // Budget check
            if num_to_free >= GC_CLOSE_BUDGET as usize {
                break;
            }

            let idx = (start + offset) & ACCOUNT_IDX_MASK;

            // Check if slot is used via bitmap
            let block = idx >> 6;
            let bit = idx & 63;
            if (self.used[block] & (1u64 << bit)) == 0 {
                continue;
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

            // Handle negative pnl by adding to pending bucket (no global ADL)
            if account.pnl < 0 {
                let loss = neg_i128_to_u128(account.pnl);
                self.pending_unpaid_loss = self.pending_unpaid_loss.saturating_add(loss);
                // Zero the pnl so account becomes true dust
                self.accounts[idx].pnl = 0;
            }

            // Queue for freeing
            to_free[num_to_free] = idx as u16;
            num_to_free += 1;
        }

        // Update cursor for next call
        self.gc_cursor = ((start + max_scan) & ACCOUNT_IDX_MASK) as u16;

        // Free all collected dust accounts
        for i in 0..num_to_free {
            self.free_slot(to_free[i]);
        }

        num_to_free as u32
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

    /// Check if a full sweep started recently.
    /// For risk-increasing ops, we require a sweep to have STARTED recently.
    /// The priority-liquidation phase runs every crank, so once a sweep starts,
    /// the worst accounts are immediately addressed.
    pub fn require_recent_full_sweep(&self, now_slot: u64) -> Result<()> {
        if now_slot.saturating_sub(self.last_full_sweep_start_slot)
            > self.max_crank_staleness_slots
        {
            return Err(RiskError::Unauthorized); // SweepStale
        }
        Ok(())
    }

    /// Check that no socialization debt is pending.
    /// Blocks value extraction (withdraw, positive pnl warmup) while pending buckets non-zero.
    /// This prevents users from withdrawing "unfunded" profit before socialization completes.
    pub fn require_no_pending_socialization(&self) -> Result<()> {
        if self.pending_profit_to_fund > 0 || self.pending_unpaid_loss > 0 {
            return Err(RiskError::Unauthorized); // PendingSocialization
        }
        Ok(())
    }

    /// Check if force-realize mode is active (insurance at or below threshold).
    /// When active, keeper_crank will run windowed force-realize steps.
    #[inline]
    fn force_realize_active(&self) -> bool {
        self.insurance_fund.balance <= self.params.risk_reduction_threshold
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

        // If starting a new sweep (step 0), record the start slot and reset state
        if self.crank_step == 0 {
            self.last_full_sweep_start_slot = now_slot;
            // Increment epochs (wrapping) - avoids O(MAX_ACCOUNTS) clears
            self.pending_epoch = self.pending_epoch.wrapping_add(1);
            // Reset in-progress lp_max_abs for fresh sweep
            self.lp_max_abs_sweep = 0;
        }

        // Accrue funding first (always) - propagate errors, don't continue with corrupt state
        self.accrue_funding(now_slot, oracle_price, funding_rate_bps_per_slot)?;

        // Check if we're advancing the global crank slot
        let advanced = now_slot > self.last_crank_slot;
        if advanced {
            self.last_crank_slot = now_slot;
        }

        // Always attempt caller's maintenance settle (best-effort, no timestamp games)
        // Use best-effort settle so undercollateralized callers can't get rebates
        let (slots_forgiven, caller_settle_ok) = if (caller_idx as usize) < MAX_ACCOUNTS
            && self.is_used(caller_idx as usize)
        {
            // Compute forgiveness for reporting only (don't mutate before settle)
            let last_fee = self.accounts[caller_idx as usize].last_fee_slot;
            let dt = now_slot.saturating_sub(last_fee);
            let forgive = dt / 2;

            // Use best-effort settle - always succeeds, no margin check
            // Forgiveness is applied by only charging for half the elapsed time
            if forgive > 0 && dt > 0 {
                // Apply forgiveness: advance last_fee_slot by half, then settle for rest
                self.accounts[caller_idx as usize].last_fee_slot =
                    last_fee.saturating_add(forgive);
            }
            let settle_result = self.settle_maintenance_fee_best_effort_for_crank(caller_idx, now_slot);

            (forgive, settle_result.is_ok())
        } else {
            (0, true) // No caller to settle, considered ok
        };

        // Window sweep with hard budget:
        // Window is defined by crank_step (deterministic), not liq_cursor
        // Use window_len as stride so sweep is meaningful even when MAX_ACCOUNTS < WINDOW
        let window_len = WINDOW.min(MAX_ACCOUNTS);
        let window_start = (self.crank_step as usize * window_len) & ACCOUNT_IDX_MASK;

        // Skip liquidation when force-realize is active (insurance at/below threshold).
        // Force-realize closes ALL positions; liquidation just adds unnecessary CU.
        let (num_liquidations, num_liq_errors) = if self.force_realize_active() {
            (0, 0)
        } else {
            // Single-pass window sweep with hard budget
            let (_sweep_checked, sweep_liqs, sweep_errors) =
                self.scan_and_liquidate_window(now_slot, oracle_price, window_start, window_len as u16, LIQ_BUDGET_PER_CRANK);
            (sweep_liqs as u32, sweep_errors)
        };

        // Windowed force-realize step: when insurance is at/below threshold,
        // force-close positions in the current window. This is bounded to O(WINDOW).
        let (force_realize_closed, force_realize_errors) =
            self.force_realize_step_window(now_slot, oracle_price, window_start, window_len);

        // Detect conditions for informational flags
        let force_realize_needed = self.force_realize_active();
        let panic_needed = !force_realize_needed
            && (self.loss_accum > 0 || self.risk_reduction_only)
            && allow_panic
            && self.total_open_interest > 0;

        // Garbage collect dust accounts BEFORE socialization
        // This ensures pending_unpaid_loss from GC'd accounts is available for haircuts
        // in the current window (if victim is in the same window as the GC'd account).
        let num_gc_closed = self.garbage_collect_dust();

        // Bounded socialization: apply pending profit/loss haircuts to WINDOW accounts
        self.socialization_step(window_start, window_len);

        // Bounded lp_max_abs update: scan LP accounts in window
        for offset in 0..window_len {
            let idx = (window_start + offset) & ACCOUNT_IDX_MASK;
            let block = idx >> 6;
            let bit = idx & 63;
            if (self.used[block] & (1u64 << bit)) == 0 {
                continue;
            }
            if !self.accounts[idx].is_lp() {
                continue;
            }
            let abs_pos = (self.accounts[idx].position_size as i128).unsigned_abs();
            self.lp_max_abs_sweep = self.lp_max_abs_sweep.max(abs_pos);
        }

        // Advance crank step; when completing final step, record completion and wrap
        self.crank_step += 1;
        if self.crank_step == NUM_STEPS {
            // Full sweep complete - finalize pending now that all accounts have been scanned
            // This ensures socialization has had a chance to haircut all positive PnL before
            // spending insurance. Guarantees pending buckets can't remain non-zero forever (liveness).
            self.finalize_pending_after_window();

            self.crank_step = 0;
            self.last_full_sweep_completed_slot = now_slot;
            // Commit bounded lp_max_abs from sweep
            self.lp_max_abs = self.lp_max_abs_sweep;
        }

        Ok(CrankOutcome {
            advanced,
            slots_forgiven,
            caller_settle_ok,
            force_realize_needed,
            panic_needed,
            num_liquidations,
            num_liq_errors,
            num_gc_closed,
            force_realize_closed,
            force_realize_errors,
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
    /// 2. Compute max safe remaining position: abs_pos_safe_max = floor(E_mtm * 10_000 * 1_000_000 / (P * target_bps))
    /// 3. close_abs = abs_pos - abs_pos_safe_max
    /// 4. If remaining position < min_liquidation_abs, do full close (dust kill-switch)
    ///
    /// Uses MTM equity (capital + realized_pnl + mark_pnl) for correct risk calculation.
    /// This is deterministic, requires no iteration, and guarantees single-pass liquidation.
    pub fn compute_liquidation_close_amount(
        &self,
        account: &Account,
        oracle_price: u64,
    ) -> Result<(u128, bool)> {
        let abs_pos = saturating_abs_i128(account.position_size) as u128;
        if abs_pos == 0 {
            return Ok((0, false));
        }

        // MTM equity at oracle price (the only correct equity for margin calculation)
        let equity = self.account_equity_mtm_at_oracle(account, oracle_price)?;

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
            return Ok((abs_pos, true)); // Full close
        }

        Ok((close_abs, close_abs == abs_pos))
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

    /// Deferred-ADL variant of oracle_close_position_core for batched liquidation.
    /// Instead of calling ADL immediately, returns DeferredAdl with totals to batch.
    fn oracle_close_position_core_deferred_adl(
        &mut self,
        idx: u16,
        oracle_price: u64,
    ) -> Result<(ClosedOutcome, DeferredAdl)> {
        // NOTE: Caller must have already called touch_account_full()

        // Check if there's a position to close
        if self.accounts[idx as usize].position_size == 0 {
            return Ok((
                ClosedOutcome {
                    abs_pos: 0,
                    mark_pnl: 0,
                    cap_before: self.accounts[idx as usize].capital,
                    cap_after: self.accounts[idx as usize].capital,
                    position_was_closed: false,
                },
                DeferredAdl::ZERO,
            ));
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

        // Update LP aggregates if this is an LP account (O(1))
        if self.accounts[idx as usize].kind == AccountKind::LP {
            self.net_lp_pos = self.net_lp_pos.saturating_sub(pos);
            self.lp_sum_abs = self.lp_sum_abs.saturating_sub(abs_pos);
            // lp_max_abs: can't decrease without full scan, leave as conservative upper bound
        }

        // DEFERRED: Instead of calling apply_adl_excluding, record profit_to_fund
        let mut deferred = DeferredAdl::ZERO;
        if mark_pnl > 0 {
            deferred.profit_to_fund = mark_pnl as u128;
            deferred.excluded = true;
            // DO NOT call apply_adl_excluding here
        }

        // Handle negative PnL: pay from capital immediately, record unpaid remainder
        // NOTE: We skip settle_warmup_to_capital for crank perf - do it inline for losses only
        let pnl = self.accounts[idx as usize].pnl;
        if pnl < 0 {
            let need = neg_i128_to_u128(pnl);
            let capital = self.accounts[idx as usize].capital;
            let pay = core::cmp::min(need, capital);

            // Pay from capital
            self.accounts[idx as usize].capital = capital.saturating_sub(pay);
            self.accounts[idx as usize].pnl = pnl.saturating_add(pay as i128);

            // Track paid losses in warmed_neg_total
            self.warmed_neg_total = add_u128(self.warmed_neg_total, pay);

            // Record unpaid portion as deferred loss
            if need > pay {
                deferred.unpaid_loss = need - pay;
                // Clamp remaining negative PnL to zero
                self.accounts[idx as usize].pnl = 0;
            }
        }
        // Update warmup markers after pnl change (matching force-close semantics)
        // This ensures profits from liquidation obey the same warmup clock rules
        self.update_warmup_slope(idx)?;
        let effective_slot = if self.warmup_paused {
            core::cmp::min(self.current_slot, self.warmup_pause_slot)
        } else {
            self.current_slot
        };
        self.accounts[idx as usize].warmup_started_at_slot = effective_slot;

        let cap_after = self.accounts[idx as usize].capital;

        Ok((
            ClosedOutcome {
                abs_pos,
                mark_pnl,
                cap_before,
                cap_after,
                position_was_closed: true,
            },
            deferred,
        ))
    }

    /// Deferred-ADL variant of oracle_close_position_slice_core for batched liquidation.
    fn oracle_close_position_slice_core_deferred_adl(
        &mut self,
        idx: u16,
        oracle_price: u64,
        close_abs: u128,
    ) -> Result<(ClosedOutcome, DeferredAdl)> {
        // NOTE: Caller must have already called touch_account_full()

        let pos = self.accounts[idx as usize].position_size;
        let current_abs_pos = saturating_abs_i128(pos) as u128;

        // Validate: can't close more than we have
        if close_abs == 0 || current_abs_pos == 0 {
            return Ok((
                ClosedOutcome {
                    abs_pos: 0,
                    mark_pnl: 0,
                    cap_before: self.accounts[idx as usize].capital,
                    cap_after: self.accounts[idx as usize].capital,
                    position_was_closed: false,
                },
                DeferredAdl::ZERO,
            ));
        }

        // If close_abs >= current position, delegate to full close
        if close_abs >= current_abs_pos {
            return self.oracle_close_position_core_deferred_adl(idx, oracle_price);
        }

        // Partial close: close_abs < current_abs_pos
        let entry = self.accounts[idx as usize].entry_price;
        let cap_before = self.accounts[idx as usize].capital;

        // Compute proportional mark PnL for the closed slice
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
        let new_pos = if pos > 0 {
            new_abs_pos as i128
        } else {
            -(new_abs_pos as i128)
        };
        self.accounts[idx as usize].position_size = new_pos;

        // Update OI
        self.total_open_interest = self.total_open_interest.saturating_sub(close_abs);

        // Update LP aggregates if this is an LP account (O(1))
        if self.accounts[idx as usize].kind == AccountKind::LP {
            // Partial close: delta = new_pos - old_pos
            self.net_lp_pos = self.net_lp_pos
                .saturating_sub(pos)
                .saturating_add(new_pos);
            self.lp_sum_abs = self.lp_sum_abs.saturating_sub(close_abs);
            // lp_max_abs: can't decrease without full scan, leave as conservative upper bound
        }

        // DEFERRED: Instead of calling apply_adl_excluding, record profit_to_fund
        let mut deferred = DeferredAdl::ZERO;
        if mark_pnl > 0 {
            deferred.profit_to_fund = mark_pnl as u128;
            deferred.excluded = true;
        }

        // Handle negative PnL: pay from capital immediately, record unpaid remainder
        // NOTE: We skip settle_warmup_to_capital for crank perf - do it inline for losses only
        let pnl = self.accounts[idx as usize].pnl;
        if pnl < 0 {
            let need = neg_i128_to_u128(pnl);
            let capital = self.accounts[idx as usize].capital;
            let pay = core::cmp::min(need, capital);

            // Pay from capital
            self.accounts[idx as usize].capital = capital.saturating_sub(pay);
            self.accounts[idx as usize].pnl = pnl.saturating_add(pay as i128);

            // Track paid losses in warmed_neg_total
            self.warmed_neg_total = add_u128(self.warmed_neg_total, pay);

            // Record unpaid portion as deferred loss
            if need > pay {
                deferred.unpaid_loss = need - pay;
                // Clamp remaining negative PnL to zero
                self.accounts[idx as usize].pnl = 0;
            }
        }

        // Update warmup markers after pnl change (matching force-close semantics)
        // This ensures profits from liquidation obey the same warmup clock rules
        self.update_warmup_slope(idx)?;
        let effective_slot = if self.warmup_paused {
            core::cmp::min(self.current_slot, self.warmup_pause_slot)
        } else {
            self.current_slot
        };
        self.accounts[idx as usize].warmup_started_at_slot = effective_slot;

        let cap_after = self.accounts[idx as usize].capital;

        Ok((
            ClosedOutcome {
                abs_pos: close_abs,
                mark_pnl,
                cap_before,
                cap_after,
                position_was_closed: true,
            },
            deferred,
        ))
    }

    /// Force-close position for force_realize_losses with deferred ADL.
    ///
    /// Key differences from liquidation deferred helpers:
    /// - Does NOT settle warmup for profits (they stay "young")
    /// - Only pays losses from capital immediately (tracks in warmed_neg_total)
    /// - Updates warmup_started_at_slot (freeze semantics)
    ///
    /// Caller must have already settled funding for this account.
    /// Returns (mark_pnl, DeferredAdl) where mark_pnl is needed for rounding compensation.
    fn force_close_position_deferred(
        &mut self,
        idx: usize,
        oracle_price: u64,
    ) -> Result<(i128, DeferredAdl)> {
        let account = &self.accounts[idx];

        // No position = nothing to close
        if account.position_size == 0 {
            return Ok((0, DeferredAdl::ZERO));
        }

        // Snapshot position details
        let pos = account.position_size;
        let abs_pos = saturating_abs_i128(pos) as u128;
        let entry = account.entry_price;

        // Compute mark PnL at oracle price
        let mark_pnl = Self::mark_pnl_for_position(pos, entry, oracle_price)?;

        // Apply mark PnL to account
        self.accounts[idx].pnl = self.accounts[idx].pnl.saturating_add(mark_pnl);

        // Close position
        self.accounts[idx].position_size = 0;
        self.accounts[idx].entry_price = oracle_price; // Determinism

        // Update OI
        self.total_open_interest = self.total_open_interest.saturating_sub(abs_pos);

        // Update LP aggregates if this is an LP account (O(1))
        if self.accounts[idx].kind == AccountKind::LP {
            self.net_lp_pos = self.net_lp_pos.saturating_sub(pos);
            self.lp_sum_abs = self.lp_sum_abs.saturating_sub(abs_pos);
            // lp_max_abs: handled by bounded sweep reset, no action needed here
        }

        // Build deferred ADL result
        let mut deferred = DeferredAdl::ZERO;

        // If profit: record for deferred ADL funding, mark for exclusion
        // DO NOT settle warmup - profit stays "young"
        if mark_pnl > 0 {
            deferred.profit_to_fund = mark_pnl as u128;
            deferred.excluded = true;
        }

        // Handle negative PnL: pay from capital immediately
        if self.accounts[idx].pnl < 0 {
            let need = neg_i128_to_u128(self.accounts[idx].pnl);
            let pay = core::cmp::min(need, self.accounts[idx].capital);

            // Pay from capital
            self.accounts[idx].capital = sub_u128(self.accounts[idx].capital, pay);
            self.accounts[idx].pnl = self.accounts[idx].pnl.saturating_add(pay as i128);

            // Track in warmed_neg_total (losses realized)
            self.warmed_neg_total = add_u128(self.warmed_neg_total, pay);

            // Accumulate unpaid portion
            if need > pay {
                deferred.unpaid_loss = need - pay;
                // Clamp remaining negative PnL to zero
                self.accounts[idx].pnl = 0;
            }
        }

        // Update warmup start marker (freeze semantics)
        let effective_slot = core::cmp::min(self.current_slot, self.warmup_pause_slot);
        self.accounts[idx].warmup_started_at_slot = effective_slot;

        Ok((mark_pnl, deferred))
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

        // Early gate: no position = nothing to liquidate (avoids expensive touch)
        if self.accounts[idx as usize].position_size == 0 {
            return Ok(false);
        }

        // Settle funding + best-effort fees (can't block on margin - we're liquidating)
        self.touch_account_for_liquidation(idx, now_slot)?;

        let account = &self.accounts[idx as usize];
        // MTM eligibility: account is liquidatable if MTM equity < maintenance margin
        if self.is_above_maintenance_margin_mtm(account, oracle_price)? {
            return Ok(false);
        }

        // Compute how much to close (closed-form, single-pass, using MTM equity)
        let (close_abs, is_full_close) = self.compute_liquidation_close_amount(account, oracle_price)?;

        if close_abs == 0 {
            return Ok(false);
        }

        // Close position via deferred helpers (unified semantics: no warmup settle)
        // This matches crank liquidation behavior - profits stay unwrapped, losses paid from capital
        let (mut outcome, mut deferred) = if is_full_close {
            self.oracle_close_position_core_deferred_adl(idx, oracle_price)?
        } else {
            match self.oracle_close_position_slice_core_deferred_adl(idx, oracle_price, close_abs) {
                Ok(r) => r,
                Err(RiskError::Overflow) => {
                    // Overflow in partial close arithmetic → force full close
                    self.oracle_close_position_core_deferred_adl(idx, oracle_price)?
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
            if !self.is_above_margin_bps_mtm(&self.accounts[idx as usize], oracle_price, target_bps)? {
                // Fallback: close remaining position entirely
                let (fallback_outcome, fallback_deferred) =
                    self.oracle_close_position_core_deferred_adl(idx, oracle_price)?;
                if fallback_outcome.position_was_closed {
                    outcome.abs_pos = outcome.abs_pos.saturating_add(fallback_outcome.abs_pos);
                    // Accumulate deferred ADL amounts
                    deferred.profit_to_fund = deferred.profit_to_fund
                        .saturating_add(fallback_deferred.profit_to_fund);
                    deferred.unpaid_loss = deferred.unpaid_loss
                        .saturating_add(fallback_deferred.unpaid_loss);
                    deferred.excluded = deferred.excluded || fallback_deferred.excluded;
                }
            }
        }

        // Accumulate into pending buckets for socialization (same semantics as crank)
        // This avoids double-counting: mark_pnl is already credited to the liquidated account,
        // so we don't immediately haircut others. Instead, let socialization handle it fairly.
        if deferred.profit_to_fund > 0 {
            self.pending_profit_to_fund = self.pending_profit_to_fund
                .saturating_add(deferred.profit_to_fund);
            // Mark epoch exclusion so this account's profit isn't haircut to fund itself
            self.pending_exclude_epoch[idx as usize] = self.pending_epoch;
        }
        if deferred.unpaid_loss > 0 {
            self.pending_unpaid_loss = self.pending_unpaid_loss
                .saturating_add(deferred.unpaid_loss);
        }

        // FEE ORDERING INVARIANT: Fee is charged AFTER position close and pending accumulation.
        // - Fee comes from remaining capital, after any loss has been paid from capital
        // - Fee can drive capital to 0, but position is already closed so margin doesn't matter
        // - This ordering means "fee has lower priority than loss payment"
        // - If fee should have priority, move this before pending accumulation
        let notional = mul_u128(outcome.abs_pos, oracle_price as u128) / 1_000_000;
        let fee_raw = mul_u128(notional, self.params.liquidation_fee_bps as u128) / 10_000;
        let fee = core::cmp::min(fee_raw, self.params.liquidation_fee_cap);

        // Pay fee from account capital (capped by available capital - never underflows)
        let account_capital = self.accounts[idx as usize].capital;
        let pay = core::cmp::min(fee, account_capital);

        self.accounts[idx as usize].capital = account_capital.saturating_sub(pay);
        self.insurance_fund.balance = self.insurance_fund.balance.saturating_add(pay);
        self.insurance_fund.fee_revenue = self.insurance_fund.fee_revenue.saturating_add(pay);

        // Recompute warmup reserved after insurance changes
        self.recompute_warmup_insurance_reserved();

        Ok(true)
    }

    /// Deferred-ADL variant of liquidate_at_oracle for batched liquidation during crank.
    /// Returns (did_liquidate, deferred_adl) instead of calling ADL immediately.
    /// Fee payment is still immediate (fee is not ADL).
    fn liquidate_at_oracle_deferred_adl(
        &mut self,
        idx: u16,
        now_slot: u64,
        oracle_price: u64,
    ) -> Result<(bool, DeferredAdl)> {
        // Validate index
        if (idx as usize) >= MAX_ACCOUNTS || !self.is_used(idx as usize) {
            return Ok((false, DeferredAdl::ZERO));
        }

        // Early gate: no position = nothing to liquidate
        if self.accounts[idx as usize].position_size == 0 {
            return Ok((false, DeferredAdl::ZERO));
        }

        // Settle funding + best-effort fees (can't block on margin - we're liquidating)
        self.touch_account_for_liquidation(idx, now_slot)?;

        let account = &self.accounts[idx as usize];
        // MTM eligibility: account is liquidatable if MTM equity < maintenance margin
        if self.is_above_maintenance_margin_mtm(account, oracle_price)? {
            return Ok((false, DeferredAdl::ZERO));
        }

        // Compute how much to close (using MTM equity)
        let (close_abs, is_full_close) = self.compute_liquidation_close_amount(account, oracle_price)?;

        if close_abs == 0 {
            return Ok((false, DeferredAdl::ZERO));
        }

        // Close position via deferred helpers
        let (mut outcome, mut deferred) = if is_full_close {
            self.oracle_close_position_core_deferred_adl(idx, oracle_price)?
        } else {
            match self.oracle_close_position_slice_core_deferred_adl(idx, oracle_price, close_abs) {
                Ok(r) => r,
                Err(RiskError::Overflow) => {
                    // Overflow in partial close → force full close
                    self.oracle_close_position_core_deferred_adl(idx, oracle_price)?
                }
                Err(e) => return Err(e),
            }
        };

        if !outcome.position_was_closed {
            return Ok((false, DeferredAdl::ZERO));
        }

        // Post-liquidation safety check: if position remains and still below target,
        // fall back to full close
        let remaining_pos = self.accounts[idx as usize].position_size;
        if remaining_pos != 0 {
            let target_bps = self.params.maintenance_margin_bps
                .saturating_add(self.params.liquidation_buffer_bps);
            if !self.is_above_margin_bps_mtm(&self.accounts[idx as usize], oracle_price, target_bps)? {
                // Fallback: close remaining position entirely
                let (fallback_outcome, fallback_deferred) =
                    self.oracle_close_position_core_deferred_adl(idx, oracle_price)?;
                if fallback_outcome.position_was_closed {
                    outcome.abs_pos = outcome.abs_pos.saturating_add(fallback_outcome.abs_pos);
                    // Accumulate deferred ADL amounts
                    deferred.profit_to_fund = deferred.profit_to_fund
                        .saturating_add(fallback_deferred.profit_to_fund);
                    deferred.unpaid_loss = deferred.unpaid_loss
                        .saturating_add(fallback_deferred.unpaid_loss);
                    deferred.excluded = deferred.excluded || fallback_deferred.excluded;
                }
            }
        }

        // Compute and apply liquidation fee (IMMEDIATE, not deferred)
        let notional = mul_u128(outcome.abs_pos, oracle_price as u128) / 1_000_000;
        let fee_raw = mul_u128(notional, self.params.liquidation_fee_bps as u128) / 10_000;
        let fee = core::cmp::min(fee_raw, self.params.liquidation_fee_cap);

        // Pay fee from account capital
        let account_capital = self.accounts[idx as usize].capital;
        let pay = core::cmp::min(fee, account_capital);

        self.accounts[idx as usize].capital = account_capital.saturating_sub(pay);
        self.insurance_fund.balance = self.insurance_fund.balance.saturating_add(pay);
        self.insurance_fund.fee_revenue = self.insurance_fund.fee_revenue.saturating_add(pay);

        // Recompute warmup reserved after insurance changes
        self.recompute_warmup_insurance_reserved();

        Ok((true, deferred))
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

    /// Windowed liquidation scan with deferred socialization.
    /// Scans `max_checks` accounts starting from `liq_cursor`, liquidates up to `max_liqs`.
    /// Accumulates profit/loss into pending buckets for bounded socialization.
    /// Window start is defined by caller (crank_step), not by internal cursor.
    /// Returns (num_checked, num_liquidated, num_errors).
    /// If any liquidation returns Err, sets risk_reduction_only = true.
    fn scan_and_liquidate_window(
        &mut self,
        now_slot: u64,
        oracle_price: u64,
        start: usize,
        max_checks: u16,
        max_liqs: u16,
    ) -> (u16, u16, u16) {
        let mut checked: u16 = 0;
        let mut liquidated: u16 = 0;
        let mut errors: u16 = 0;
        // Cap to MAX_ACCOUNTS to avoid wrap-around
        let actual_checks = (max_checks as usize).min(MAX_ACCOUNTS);
        let epoch = self.pending_epoch;

        for offset in 0..actual_checks {
            if liquidated >= max_liqs {
                break;
            }

            let idx = (start + offset) & ACCOUNT_IDX_MASK;

            // Check if slot is used
            let block = idx >> 6;
            let bit = idx & 63;
            if (self.used[block] & (1u64 << bit)) == 0 {
                continue; // Not used, skip
            }

            checked += 1;

            // Early gate: skip accounts with no position
            if self.accounts[idx].position_size == 0 {
                continue;
            }

            // Attempt deferred liquidation
            match self.liquidate_at_oracle_deferred_adl(idx as u16, now_slot, oracle_price) {
                Ok((true, deferred)) => {
                    liquidated += 1;
                    self.lifetime_liquidations = self.lifetime_liquidations.saturating_add(1);
                    // Accumulate into pending buckets (no ADL call)
                    self.pending_profit_to_fund = self
                        .pending_profit_to_fund
                        .saturating_add(deferred.profit_to_fund);
                    self.pending_unpaid_loss = self
                        .pending_unpaid_loss
                        .saturating_add(deferred.unpaid_loss);
                    // Mark for exclusion from profit-funding
                    if deferred.excluded {
                        self.pending_exclude_epoch[idx] = epoch;
                    }
                }
                Ok((false, _)) => {} // Not liquidatable, fine
                Err(_) => {
                    // Liquidation error - set risk reduction mode
                    errors += 1;
                    self.risk_reduction_only = true;
                }
            }
        }

        (checked, liquidated, errors)
    }

    /// Windowed force-realize step: closes positions in the current window when
    /// insurance is at/below threshold. Bounded to O(WINDOW) work per crank.
    ///
    /// Returns (closed_positions, errors).
    ///
    /// This is NOT liquidation - it's a forced unwind of all positions in the window.
    /// Unpaid losses are accumulated into pending_unpaid_loss for socialization_step
    /// to handle across subsequent cranks.
    fn force_realize_step_window(
        &mut self,
        now_slot: u64,
        oracle_price: u64,
        start: usize,
        len: usize,
    ) -> (u16, u16) {
        // Gate: only active when insurance at/below threshold
        if !self.force_realize_active() {
            return (0, 0);
        }

        // Enter risk reduction mode (idempotent)
        self.enter_risk_reduction_only_mode();

        let mut closed: u16 = 0;
        let mut errors: u16 = 0;
        let mut budget_left = FORCE_REALIZE_BUDGET_PER_CRANK;

        for offset in 0..len {
            // Hard budget: stop scanning when we've done enough work
            if budget_left == 0 {
                break;
            }

            let idx = (start + offset) & ACCOUNT_IDX_MASK;

            // Check if slot is used
            let block = idx >> 6;
            let bit = idx & 63;
            if (self.used[block] & (1u64 << bit)) == 0 {
                continue;
            }

            // Skip accounts with no position
            if self.accounts[idx].position_size == 0 {
                continue;
            }

            // Best-effort touch: can't stall on margin checks
            if self
                .touch_account_for_force_realize(idx as u16, now_slot)
                .is_err()
            {
                errors += 1;
                self.risk_reduction_only = true;
                continue;
            }

            // Force-close the position (not liquidation)
            match self.force_close_position_deferred(idx, oracle_price) {
                Ok((mark_pnl, deferred)) => {
                    closed += 1;
                    budget_left = budget_left.saturating_sub(1);
                    self.lifetime_force_realize_closes =
                        self.lifetime_force_realize_closes.saturating_add(1);

                    // Accumulate unpaid loss into pending bucket
                    self.pending_unpaid_loss = self
                        .pending_unpaid_loss
                        .saturating_add(deferred.unpaid_loss);

                    // Rounding compensation: positive mark_pnl represents "profit"
                    // that must be funded by others. In force-realize, we treat this
                    // as additional loss to socialize (matches full force_realize_losses behavior).
                    if mark_pnl > 0 {
                        self.pending_unpaid_loss = self
                            .pending_unpaid_loss
                            .saturating_add(mark_pnl as u128);
                    }

                    // Note: We ignore deferred.profit_to_fund. Force-realize is batch-close;
                    // winners are naturally funded by losers, and any mismatch is handled
                    // via pending_unpaid_loss + socialization_step.
                }
                Err(_) => {
                    errors += 1;
                    self.risk_reduction_only = true;
                }
            }
        }

        // Recompute warmup insurance reserved (safe, bounded)
        self.recompute_warmup_insurance_reserved();

        (closed, errors)
    }

    // ========================================
    // Bounded Socialization (replaces global ADL in crank)
    // ========================================

    /// Bounded socialization step: haircuts profits from WINDOW accounts.
    ///
    /// Applies pending profit-funding and loss socialization to accounts in
    /// [start..start+len) window. Starvation-free because deterministic sweep
    /// guarantees all accounts are eventually visited.
    ///
    /// Cost: O(len), bounded by WINDOW.
    fn socialization_step(&mut self, start: usize, len: usize) {
        let epoch = self.pending_epoch;
        let effective_slot = self.effective_warmup_slot();

        for offset in 0..len {
            // Early exit if nothing left to socialize
            if self.pending_profit_to_fund == 0 && self.pending_unpaid_loss == 0 {
                break;
            }

            let idx = (start + offset) & ACCOUNT_IDX_MASK;

            // Check if slot is used
            let block = idx >> 6;
            let bit = idx & 63;
            if (self.used[block] & (1u64 << bit)) == 0 {
                continue;
            }

            // Compute unwrapped PnL for this account (subject to ADL haircuts)
            let unwrapped = self.compute_unwrapped_pnl_at(&self.accounts[idx], effective_slot);
            if unwrapped == 0 {
                continue;
            }

            let mut remaining = unwrapped;

            // Pass 1: Profit funding (if not excluded)
            if self.pending_profit_to_fund > 0 && self.pending_exclude_epoch[idx] != epoch {
                let take = core::cmp::min(remaining, self.pending_profit_to_fund);
                if take > 0 {
                    self.accounts[idx].pnl =
                        self.accounts[idx].pnl.saturating_sub(take as i128);
                    self.pending_profit_to_fund =
                        self.pending_profit_to_fund.saturating_sub(take);
                    remaining = remaining.saturating_sub(take);
                }
            }

            // Pass 2: Loss socialization (no exclusions)
            if self.pending_unpaid_loss > 0 && remaining > 0 {
                let take = core::cmp::min(remaining, self.pending_unpaid_loss);
                if take > 0 {
                    self.accounts[idx].pnl =
                        self.accounts[idx].pnl.saturating_sub(take as i128);
                    self.pending_unpaid_loss =
                        self.pending_unpaid_loss.saturating_sub(take);
                }
            }
        }
    }

    /// Finalize pending buckets after window socialization.
    ///
    /// This ensures pending_profit_to_fund and pending_unpaid_loss cannot
    /// remain non-zero forever (which would block withdrawals permanently).
    ///
    /// After haircuts from socialization_step:
    /// 1. Spend insurance (above floor, respecting reserved) to cover remaining
    /// 2. Move any uncovered remainder to loss_accum and clear pending buckets
    ///
    /// This guarantees liveness: pending progress every sweep.
    fn finalize_pending_after_window(&mut self) {
        // If nothing pending, early exit
        if self.pending_profit_to_fund == 0 && self.pending_unpaid_loss == 0 {
            return;
        }

        // Spend insurance to cover pending (spendable = above floor, minus reserved)
        let spendable = self.insurance_spendable_unreserved();

        if spendable > 0 {
            // First: cover profit funding (profit needs to come from somewhere)
            if self.pending_profit_to_fund > 0 {
                let spend_profit = core::cmp::min(spendable, self.pending_profit_to_fund);
                self.insurance_fund.balance = self
                    .insurance_fund
                    .balance
                    .saturating_sub(spend_profit);
                self.pending_profit_to_fund = self
                    .pending_profit_to_fund
                    .saturating_sub(spend_profit);
                // Recompute reserved immediately so spendable_after is accurate
                self.recompute_warmup_insurance_reserved();
            }

            // Recompute spendable after profit funding (reserved was just updated)
            let spendable_after = self.insurance_spendable_unreserved();

            // Second: cover unpaid losses
            if self.pending_unpaid_loss > 0 && spendable_after > 0 {
                let spend_loss = core::cmp::min(spendable_after, self.pending_unpaid_loss);
                self.insurance_fund.balance = self
                    .insurance_fund
                    .balance
                    .saturating_sub(spend_loss);
                self.pending_unpaid_loss = self
                    .pending_unpaid_loss
                    .saturating_sub(spend_loss);
            }

            // Recompute warmup reserved after insurance changes
            self.recompute_warmup_insurance_reserved();
        }

        // Handle remaining pending_unpaid_loss: can go to loss_accum (that's what it's for)
        if self.pending_unpaid_loss > 0 {
            self.loss_accum = self.loss_accum.saturating_add(self.pending_unpaid_loss);
            self.pending_unpaid_loss = 0;
            // Enter risk-reduction mode (uncovered losses exist)
            self.enter_risk_reduction_only_mode();
        }

        // Handle remaining pending_profit_to_fund: CANNOT go to loss_accum
        // Unfunded profits must remain pending to block value extraction.
        // If we can't fund it, the system is insolvent relative to that credited profit.
        if self.pending_profit_to_fund > 0 {
            // Leave pending_profit_to_fund non-zero - this will block withdrawals
            // via require_no_pending_socialization() until properly funded or admin resolves
            self.enter_risk_reduction_only_mode();
        }
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

    /// Minimal touch for crank liquidations: funding + maintenance only.
    /// Skips warmup settlement for performance - losses are handled inline
    /// by the deferred close helpers, positive warmup left for user ops.
    fn touch_account_for_crank(
        &mut self,
        idx: u16,
        now_slot: u64,
        oracle_price: u64,
    ) -> Result<()> {
        // 1. Settle funding
        self.touch_account(idx)?;

        // 2. Settle maintenance fees (may trigger undercollateralized error)
        self.settle_maintenance_fee(idx, now_slot, oracle_price)?;

        // NOTE: No warmup settlement - handled inline for losses in close helpers
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
        // Require fresh crank (time-based) before state-changing operations
        self.require_fresh_crank(now_slot)?;

        // Require recent full sweep started
        self.require_recent_full_sweep(now_slot)?;

        // Block withdrawals while socialization debt is pending
        // This prevents extracting unfunded value
        self.require_no_pending_socialization()?;

        // Withdrawals are neutral in risk mode (allowed)
        self.enforce_op(OpClass::RiskNeutral)?;

        // Validate account exists
        if !self.is_used(idx as usize) {
            return Err(RiskError::AccountNotFound);
        }

        // Full settlement: funding + maintenance fees + warmup
        self.touch_account_full(idx, now_slot, oracle_price)?;

        // Read account state (scope the borrow)
        let (old_capital, pnl, position_size, entry_price) = {
            let account = &self.accounts[idx as usize];
            (account.capital, account.pnl, account.position_size, account.entry_price)
        };

        // Check we have enough capital
        if old_capital < amount {
            return Err(RiskError::InsufficientBalance);
        }

        // Calculate MTM equity after withdrawal
        // equity_mtm = max(0, new_capital + pnl + mark_pnl)
        let new_capital = sub_u128(old_capital, amount);
        let mark_pnl = Self::mark_pnl_for_position(position_size, entry_price, oracle_price)?;
        let cap_i = u128_to_i128_clamped(new_capital);
        let new_eq_i = cap_i.saturating_add(pnl).saturating_add(mark_pnl);
        let new_equity_mtm = if new_eq_i > 0 { new_eq_i as u128 } else { 0 };

        // If account has position, must maintain initial margin at ORACLE price (MTM check)
        // This prevents withdrawing to a state that's immediately liquidatable
        if position_size != 0 {
            let position_notional = mul_u128(
                saturating_abs_i128(position_size) as u128,
                oracle_price as u128,
            ) / 1_000_000;

            let initial_margin_required =
                mul_u128(position_notional, self.params.initial_margin_bps as u128) / 10_000;

            if new_equity_mtm < initial_margin_required {
                return Err(RiskError::Undercollateralized);
            }
        }

        // Commit the withdrawal
        self.accounts[idx as usize].capital = new_capital;
        self.vault = sub_u128(self.vault, amount);

        // Post-withdrawal MTM maintenance margin check at oracle price
        // This is a safety belt to ensure we never leave an account in liquidatable state
        if self.accounts[idx as usize].position_size != 0 {
            if !self.is_above_maintenance_margin_mtm(&self.accounts[idx as usize], oracle_price)? {
                // Revert the withdrawal
                self.accounts[idx as usize].capital = old_capital;
                self.vault = add_u128(self.vault, amount);
                return Err(RiskError::Undercollateralized);
            }
        }

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

    /// Realized-only equity: max(0, capital + realized_pnl).
    ///
    /// DEPRECATED for margin checks: Use account_equity_mtm_at_oracle instead.
    /// This helper is retained for reporting, PnL display, and test assertions that
    /// specifically need realized-only equity.
    #[inline]
    pub fn account_equity(&self, account: &Account) -> u128 {
        let cap_i = u128_to_i128_clamped(account.capital);
        let eq_i = cap_i.saturating_add(account.pnl);
        if eq_i > 0 { eq_i as u128 } else { 0 }
    }

    /// Mark-to-market equity at oracle price (the ONLY correct equity for margin checks).
    /// equity_mtm = max(0, capital + realized_pnl + mark_pnl(position, entry, oracle))
    pub fn account_equity_mtm_at_oracle(&self, account: &Account, oracle_price: u64) -> Result<u128> {
        let mark = Self::mark_pnl_for_position(
            account.position_size,
            account.entry_price,
            oracle_price,
        )?;
        let cap_i = u128_to_i128_clamped(account.capital);
        let eq_i = cap_i.saturating_add(account.pnl).saturating_add(mark);
        Ok(if eq_i > 0 { eq_i as u128 } else { 0 })
    }

    /// MTM margin check: is equity_mtm > required margin?
    /// This is the ONLY correct margin predicate for all risk checks.
    pub fn is_above_margin_bps_mtm(
        &self,
        account: &Account,
        oracle_price: u64,
        bps: u64,
    ) -> Result<bool> {
        let equity = self.account_equity_mtm_at_oracle(account, oracle_price)?;

        // Position value at oracle price
        let position_value = mul_u128(
            saturating_abs_i128(account.position_size) as u128,
            oracle_price as u128,
        ) / 1_000_000;

        // Margin requirement at given bps
        let margin_required = mul_u128(position_value, bps as u128) / 10_000;

        Ok(equity > margin_required)
    }

    /// MTM maintenance margin check
    #[inline]
    pub fn is_above_maintenance_margin_mtm(&self, account: &Account, oracle_price: u64) -> Result<bool> {
        self.is_above_margin_bps_mtm(account, oracle_price, self.params.maintenance_margin_bps)
    }

    /// Check if account is above maintenance margin (DEPRECATED: uses realized-only equity)
    /// Use is_above_maintenance_margin_mtm for all margin checks.
    pub fn is_above_maintenance_margin(&self, account: &Account, oracle_price: u64) -> bool {
        self.is_above_margin_bps(account, oracle_price, self.params.maintenance_margin_bps)
    }

    /// Cheap priority score for ranking liquidation candidates.
    /// Score = max(maint_required - equity, 0).
    /// Higher score = more urgent to liquidate.
    ///
    /// This is a ranking heuristic only - NOT authoritative.
    /// Real liquidation still calls touch_account_full() and checks margin properly.
    /// A "wrong" top-K pick is harmless: it just won't liquidate.
    #[inline]
    fn liq_priority_score(&self, a: &Account, oracle_price: u64) -> u128 {
        if a.position_size == 0 {
            return 0;
        }

        // Use MTM equity for consistent prioritization (fallback to 0 on error = not liquidatable)
        let equity = match self.account_equity_mtm_at_oracle(a, oracle_price) {
            Ok(e) => e,
            Err(_) => return 0,
        };

        let pos_value = mul_u128(
            saturating_abs_i128(a.position_size) as u128,
            oracle_price as u128,
        ) / 1_000_000;

        let maint = mul_u128(pos_value, self.params.maintenance_margin_bps as u128) / 10_000;

        if equity >= maint {
            0
        } else {
            maint - equity
        }
    }

    /// Check if account is above a given margin threshold (DEPRECATED: uses realized-only equity).
    ///
    /// Use is_above_margin_bps_mtm for all margin checks. This helper is retained for
    /// tests that specifically need realized-only margin comparison.
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
        // Require fresh crank (time-based) before state-changing operations
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
            // Risk-increasing: require recent full sweep
            self.require_recent_full_sweep(now_slot)?;
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

        // Check user maintenance margin (MTM: includes unrealized mark PnL)
        if new_user_position != 0 {
            // MTM equity = capital + new_realized_pnl + mark_pnl(new_pos, new_entry, oracle)
            let user_mark = Self::mark_pnl_for_position(new_user_position, new_user_entry, oracle_price)?;
            let user_cap_i = u128_to_i128_clamped(user.capital);
            let user_eq_i = user_cap_i.saturating_add(new_user_pnl).saturating_add(user_mark);
            let user_equity_mtm = if user_eq_i > 0 { user_eq_i as u128 } else { 0 };
            let position_value = mul_u128(
                saturating_abs_i128(new_user_position) as u128,
                oracle_price as u128,
            ) / 1_000_000;
            let margin_required =
                mul_u128(position_value, self.params.maintenance_margin_bps as u128) / 10_000;
            if user_equity_mtm <= margin_required {
                return Err(RiskError::Undercollateralized);
            }
        }

        // Check LP maintenance margin (MTM: includes unrealized mark PnL)
        if new_lp_position != 0 {
            // MTM equity = capital + new_realized_pnl + mark_pnl(new_pos, new_entry, oracle)
            let lp_mark = Self::mark_pnl_for_position(new_lp_position, new_lp_entry, oracle_price)?;
            let lp_cap_i = u128_to_i128_clamped(lp.capital);
            let lp_eq_i = lp_cap_i.saturating_add(new_lp_pnl).saturating_add(lp_mark);
            let lp_equity_mtm = if lp_eq_i > 0 { lp_eq_i as u128 } else { 0 };
            let position_value = mul_u128(
                saturating_abs_i128(new_lp_position) as u128,
                oracle_price as u128,
            ) / 1_000_000;
            let margin_required =
                mul_u128(position_value, self.params.maintenance_margin_bps as u128) / 10_000;
            if lp_equity_mtm <= margin_required {
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

        // Update LP aggregates for funding/threshold (O(1))
        let old_lp_abs = saturating_abs_i128(old_lp_pos) as u128;
        let new_lp_abs = saturating_abs_i128(new_lp_position) as u128;
        // net_lp_pos: delta = new - old
        self.net_lp_pos = self.net_lp_pos
            .saturating_sub(old_lp_pos)
            .saturating_add(new_lp_position);
        // lp_sum_abs: delta of abs values
        if new_lp_abs > old_lp_abs {
            self.lp_sum_abs = self.lp_sum_abs.saturating_add(new_lp_abs - old_lp_abs);
        } else {
            self.lp_sum_abs = self.lp_sum_abs.saturating_sub(old_lp_abs - new_lp_abs);
        }
        // lp_max_abs: monotone increase only (conservative upper bound)
        self.lp_max_abs = self.lp_max_abs.max(new_lp_abs);

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

    /// Compute effective slot for warmup (hoisted for efficiency)
    #[inline]
    fn effective_warmup_slot(&self) -> u64 {
        if self.warmup_paused {
            core::cmp::min(self.current_slot, self.warmup_pause_slot)
        } else {
            self.current_slot
        }
    }

    /// Calculate withdrawable PNL with pre-computed effective_slot
    #[inline]
    fn compute_withdrawable_pnl_at(&self, account: &Account, effective_slot: u64) -> u128 {
        if account.pnl <= 0 {
            return 0;
        }
        let positive_pnl = account.pnl as u128;
        let available_pnl = positive_pnl.saturating_sub(account.reserved_pnl);
        let elapsed_slots = effective_slot.saturating_sub(account.warmup_started_at_slot);
        let warmed_up_cap = mul_u128(account.warmup_slope_per_step, elapsed_slots as u128);
        core::cmp::min(available_pnl, warmed_up_cap)
    }

    /// Calculate withdrawable PnL for an account (inline helper)
    /// withdrawable = min(available_pnl, warmed_up_cap)
    #[inline]
    fn compute_withdrawable_pnl(&self, account: &Account) -> u128 {
        self.compute_withdrawable_pnl_at(account, self.effective_warmup_slot())
    }

    /// Calculate unwrapped PNL with pre-computed effective_slot
    #[inline]
    fn compute_unwrapped_pnl_at(&self, account: &Account, effective_slot: u64) -> u128 {
        if account.pnl <= 0 {
            return 0;
        }
        let positive_pnl = account.pnl as u128;
        let reserved = account.reserved_pnl;
        let withdrawable = self.compute_withdrawable_pnl_at(account, effective_slot);
        positive_pnl
            .saturating_sub(reserved)
            .saturating_sub(withdrawable)
    }

    /// Calculate unwrapped PNL for an account (inline helper for ADL)
    /// unwrapped = max(0, positive_pnl - reserved_pnl - withdrawable_pnl)
    /// This is PnL that hasn't yet warmed and isn't reserved - subject to ADL haircuts
    #[inline]
    fn compute_unwrapped_pnl(&self, account: &Account) -> u128 {
        self.compute_unwrapped_pnl_at(account, self.effective_warmup_slot())
    }

    /// ADL heap comparator: a "wins" (is larger) if rem_a > rem_b, or tie-break by lower idx
    #[inline]
    fn adl_heap_better(&self, a: u16, b: u16) -> bool {
        let ra = self.adl_remainder_scratch[a as usize];
        let rb = self.adl_remainder_scratch[b as usize];
        ra > rb || (ra == rb && a < b)
    }

    /// Sift down for ADL max-heap
    fn adl_sift_down(&mut self, heap_size: usize, mut pos: usize) {
        loop {
            let left = 2 * pos + 1;
            if left >= heap_size {
                break;
            }
            let right = left + 1;

            let mut best = left;
            if right < heap_size
                && self.adl_heap_better(self.adl_idx_scratch[right], self.adl_idx_scratch[left])
            {
                best = right;
            }
            if self.adl_heap_better(self.adl_idx_scratch[pos], self.adl_idx_scratch[best]) {
                break;
            }
            self.adl_idx_scratch.swap(pos, best);
            pos = best;
        }
    }

    /// Build max-heap for ADL remainder distribution
    fn adl_build_heap(&mut self, heap_size: usize) {
        if heap_size < 2 {
            return;
        }
        let mut i = (heap_size - 2) / 2;
        loop {
            self.adl_sift_down(heap_size, i);
            if i == 0 {
                break;
            }
            i -= 1;
        }
    }

    /// Pop max from ADL heap, returns the index
    fn adl_pop_max(&mut self, heap_size: &mut usize) -> u16 {
        debug_assert!(*heap_size > 0);
        let best = self.adl_idx_scratch[0];
        *heap_size -= 1;
        if *heap_size > 0 {
            self.adl_idx_scratch[0] = self.adl_idx_scratch[*heap_size];
            self.adl_sift_down(*heap_size, 0);
        }
        best
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

    // ========================================
    // LP Aggregates (O(1) access for funding/threshold)
    // ========================================

    /// Net LP position: sum of position_size across all LP accounts.
    /// Used for inventory-based funding rate calculation.
    #[inline]
    pub fn get_net_lp_pos(&self) -> i128 {
        self.net_lp_pos
    }

    /// Sum of abs(position_size) across all LP accounts.
    /// Used for risk threshold calculation.
    #[inline]
    pub fn get_lp_sum_abs(&self) -> u128 {
        self.lp_sum_abs
    }

    /// Max abs(position_size) across all LP accounts (monotone upper bound).
    /// May be conservative; only increases, reset via bounded sweep.
    #[inline]
    pub fn get_lp_max_abs(&self) -> u128 {
        self.lp_max_abs
    }

    /// Compute LP risk units for threshold: max_abs + sum_abs/8.
    /// This is O(1) using maintained aggregates.
    #[inline]
    pub fn compute_lp_risk_units(&self) -> u128 {
        self.lp_max_abs.saturating_add(self.lp_sum_abs / 8)
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
        // SAFETY: Block positive conversion while socialization debt is pending
        // This prevents converting unfunded profit to withdrawable capital
        let pnl = self.accounts[idx as usize].pnl;
        if pnl > 0 && cap > 0 {
            self.require_no_pending_socialization()?;
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
    ///
    /// Optimized: 2 bitmap scans (down from 4), O(m + take*log(m)) heap selection.
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

        // Hoist effective_slot once (saves repeated warmup pause checks)
        let effective_slot = self.effective_warmup_slot();

        // Pass 1: Compute total unwrapped PNL (excluding specified account if any)
        let mut total_unwrapped = 0u128;

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

                let unwrapped = self.compute_unwrapped_pnl_at(&self.accounts[idx], effective_slot);
                total_unwrapped = total_unwrapped.saturating_add(unwrapped);
            }
        }

        // Determine how much loss can be socialized via unwrapped PNL
        let loss_to_socialize = core::cmp::min(total_loss, total_unwrapped);

        // Track total applied for conservation
        let mut applied_from_pnl: u128 = 0;

        // Index list count for heap (built inline during Pass 2)
        let mut m: usize = 0;

        if loss_to_socialize > 0 && total_unwrapped > 0 {
            // Pass 2: Compute floor haircuts, store remainders, build idx list inline
            // (Merged: no separate scratch zeroing, no separate idx collection pass)
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
                    if account.pnl <= 0 {
                        continue;
                    }

                    let unwrapped = self.compute_unwrapped_pnl_at(account, effective_slot);
                    if unwrapped == 0 {
                        continue;
                    }

                    let numer = loss_to_socialize
                        .checked_mul(unwrapped)
                        .ok_or(RiskError::Overflow)?;
                    let haircut = numer / total_unwrapped;
                    let rem = numer % total_unwrapped;

                    self.accounts[idx].pnl =
                        self.accounts[idx].pnl.saturating_sub(haircut as i128);
                    applied_from_pnl += haircut;

                    // Store remainder and add to idx list only if non-zero
                    if rem != 0 {
                        self.adl_remainder_scratch[idx] = rem;
                        self.adl_idx_scratch[m] = idx as u16;
                        m += 1;
                    }
                }
            }

            // Step 3: Distribute leftover using largest-remainder method
            // Use heap pop top-K: O(m) build + O(take * log m) pops
            let leftover = loss_to_socialize - applied_from_pnl;

            if leftover > 0 && m > 0 {
                // Build max-heap
                self.adl_build_heap(m);
                let mut heap_size = m;

                // Pop top `take` elements and apply +1 haircut to each
                let take = core::cmp::min(leftover as usize, m);
                for _ in 0..take {
                    let idx = self.adl_pop_max(&mut heap_size) as usize;
                    self.accounts[idx].pnl = self.accounts[idx].pnl.saturating_sub(1);
                }
                applied_from_pnl += take as u128;
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

    /// ADL variant that excludes accounts marked in adl_exclude_scratch.
    /// Used for batched liquidation to exclude all winners from funding their own profit.
    pub fn apply_adl_excluding_set(&mut self, total_loss: u128) -> Result<()> {
        // ADL reduces risk (allowed in risk mode)
        self.enforce_op(OpClass::RiskReduce)?;

        if total_loss == 0 {
            return Ok(());
        }

        // Inline helper - check adl_exclude_scratch
        #[inline]
        fn is_excluded_by_scratch(scratch: &[u8; MAX_ACCOUNTS], idx: usize) -> bool {
            scratch[idx] != 0
        }

        // Hoist effective_slot once
        let effective_slot = self.effective_warmup_slot();

        // Pass 1: Compute total unwrapped PNL (excluding accounts marked in scratch)
        let mut total_unwrapped = 0u128;

        for block in 0..BITMAP_WORDS {
            let mut w = self.used[block];
            while w != 0 {
                let bit = w.trailing_zeros() as usize;
                let idx = block * 64 + bit;
                w &= w - 1;
                if idx >= MAX_ACCOUNTS {
                    continue;
                }
                if is_excluded_by_scratch(&self.adl_exclude_scratch, idx) {
                    continue;
                }

                let unwrapped = self.compute_unwrapped_pnl_at(&self.accounts[idx], effective_slot);
                total_unwrapped = total_unwrapped.saturating_add(unwrapped);
            }
        }

        // Determine how much loss can be socialized via unwrapped PNL
        let loss_to_socialize = core::cmp::min(total_loss, total_unwrapped);

        // Track total applied for conservation
        let mut applied_from_pnl: u128 = 0;

        // Index list count for heap
        let mut m: usize = 0;

        if loss_to_socialize > 0 && total_unwrapped > 0 {
            // Pass 2: Compute floor haircuts, store remainders, build idx list inline
            for block in 0..BITMAP_WORDS {
                let mut w = self.used[block];
                while w != 0 {
                    let bit = w.trailing_zeros() as usize;
                    let idx = block * 64 + bit;
                    w &= w - 1;
                    if idx >= MAX_ACCOUNTS {
                        continue;
                    }
                    if is_excluded_by_scratch(&self.adl_exclude_scratch, idx) {
                        continue;
                    }

                    let account = &self.accounts[idx];
                    if account.pnl <= 0 {
                        continue;
                    }

                    let unwrapped = self.compute_unwrapped_pnl_at(account, effective_slot);
                    if unwrapped == 0 {
                        continue;
                    }

                    let numer = loss_to_socialize
                        .checked_mul(unwrapped)
                        .ok_or(RiskError::Overflow)?;
                    let haircut = numer / total_unwrapped;
                    let rem = numer % total_unwrapped;

                    self.accounts[idx].pnl =
                        self.accounts[idx].pnl.saturating_sub(haircut as i128);
                    applied_from_pnl += haircut;

                    // Store remainder and add to idx list only if non-zero
                    if rem != 0 {
                        self.adl_remainder_scratch[idx] = rem;
                        self.adl_idx_scratch[m] = idx as u16;
                        m += 1;
                    }
                }
            }

            // Step 3: Distribute leftover using largest-remainder method
            let leftover = loss_to_socialize - applied_from_pnl;

            if leftover > 0 && m > 0 {
                // Build max-heap
                self.adl_build_heap(m);
                let mut heap_size = m;

                // Pop top `take` elements and apply +1 haircut to each
                let take = core::cmp::min(leftover as usize, m);
                for _ in 0..take {
                    let idx = self.adl_pop_max(&mut heap_size) as usize;
                    self.accounts[idx].pnl = self.accounts[idx].pnl.saturating_sub(1);
                }
                applied_from_pnl += take as u128;
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
                "Reserved invariant violated in apply_adl_excluding_set"
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

        // Clear pending socialization buckets - panic does full ADL, superseding incremental
        self.pending_profit_to_fund = 0;
        self.pending_unpaid_loss = 0;

        // Reset LP aggregates - all positions will be closed
        self.net_lp_pos = 0;
        self.lp_sum_abs = 0;
        self.lp_max_abs = 0;

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
    ///
    /// NOTE: Unlike liquidation, force_realize does NOT need profit funding ADL because:
    /// - All positions close at once
    /// - Mark PnLs are zero-sum (profits are funded by losses in the same batch)
    /// - Only unpaid losses (capital exhausted) need ADL socialization
    pub fn force_realize_losses(&mut self, oracle_price: u64) -> Result<()> {
        // Force realize is a risk-reducing operation
        self.enforce_op(OpClass::RiskReduce)?;

        // Gate: only allowed when insurance is at or below floor
        if self.insurance_fund.balance > self.params.risk_reduction_threshold {
            return Err(RiskError::Unauthorized);
        }

        // Enter risk-reduction-only mode (freezes warmups)
        self.enter_risk_reduction_only_mode();

        // Reset LP aggregates - all positions will be closed
        self.net_lp_pos = 0;
        self.lp_sum_abs = 0;
        self.lp_max_abs = 0;

        // Track unpaid losses (capital exhausted) and rounding
        let mut unpaid_total: u128 = 0;
        let mut total_mark_pnl: i128 = 0;

        // Phase 1: settle funding, close positions via deferred helper
        let global_funding_index = self.funding_index_qpb_e6;
        for block in 0..BITMAP_WORDS {
            let mut w = self.used[block];
            while w != 0 {
                let bit = w.trailing_zeros() as usize;
                let idx = block * 64 + bit;
                w &= w - 1;

                if idx >= MAX_ACCOUNTS {
                    continue;
                }

                // Settle funding first (required for correct PNL accounting)
                Self::settle_account_funding(&mut self.accounts[idx], global_funding_index)?;

                // Skip accounts with no position
                if self.accounts[idx].position_size == 0 {
                    continue;
                }

                // Close position via deferred helper
                // NOTE: We ignore profit_to_fund because in force_realize_losses,
                // profits are naturally funded by losses (zero-sum batch close)
                let (mark_pnl, deferred) = self.force_close_position_deferred(idx, oracle_price)?;

                // Accumulate rounding compensation
                total_mark_pnl = total_mark_pnl.saturating_add(mark_pnl);

                // Only accumulate unpaid losses (capital exhausted)
                unpaid_total = unpaid_total.saturating_add(deferred.unpaid_loss);
            }
        }

        // Rounding compensation:
        // If positive: treat as additional loss to socialize via ADL
        // If negative: absorbed by bounded conservation slack (don't mint to insurance)
        if total_mark_pnl > 0 {
            unpaid_total = unpaid_total.saturating_add(total_mark_pnl as u128);
        }

        // Phase 2: Socialize unpaid losses via ADL waterfall
        // All accounts now have their mark_pnl applied, so ADL can haircut properly
        if unpaid_total > 0 {
            let _ = self.apply_adl(unpaid_total);
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
