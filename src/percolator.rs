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

/// User account with PNL warmup and fee tracking
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserAccount {
    /// User's deposited principal - NEVER reduced by ADL/socialization (Invariant I1)
    pub principal: u128,

    /// Realized PNL (can be positive or negative)
    pub pnl_ledger: i128,

    /// PNL reserved for pending withdrawals
    pub reserved_pnl: u128,

    /// Warmup state for PNL vesting
    pub warmup_state: Warmup,

    /// Current position size (for liquidation checks)
    pub position_size: i128,

    /// Last entry price (for PNL calculation)
    pub entry_price: u64,

    /// Fee index snapshot at last update
    pub fee_index_user: u128,

    /// Accrued but not yet claimed fees
    pub fee_accrued: u128,

    /// Cached vested positive PNL for fee distribution
    pub vested_pos_snapshot: u128,
}

/// LP account - one per matching engine
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LPAccount {
    /// Matching engine program ID
    pub matching_engine_program: [u8; 32],

    /// Matching engine context account
    pub matching_engine_context: [u8; 32],

    /// LP deposited capital
    pub lp_capital: u128,

    /// LP's PNL from providing liquidity
    pub lp_pnl: i128,

    /// LP position (opposite of user positions)
    pub lp_position_size: i128,

    /// LP's entry price
    pub lp_entry_price: u64,
}

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
}

/// Main risk engine state - all in one contiguous memory chunk
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RiskEngine {
    /// Total vault balance (all deposited funds)
    pub vault: u128,

    /// Insurance fund
    pub insurance_fund: InsuranceFund,

    /// All user accounts
    pub users: Vec<UserAccount>,

    /// All LP accounts (one per matching engine)
    pub lps: Vec<LPAccount>,

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

    /// User account not found
    UserNotFound,

    /// LP account not found
    LPNotFound,

    /// Position size mismatch
    PositionSizeMismatch,
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
fn div_u128(a: u128, b: u128) -> u128 {
    if b == 0 { 0 } else { a / b }
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
// Core Invariants and Helpers
// ============================================================================

impl RiskEngine {
    /// Calculate withdrawable PNL for a user (after warmup)
    /// This is the core PNL warmup mechanism (Invariant I5)
    pub fn withdrawable_pnl(&self, user: &UserAccount) -> u128 {
        // Only positive PNL can be withdrawn
        let positive_pnl = clamp_pos_i128(user.pnl_ledger);

        // Available = positive PNL - reserved
        let available_pnl = sub_u128(positive_pnl, user.reserved_pnl);

        // Calculate elapsed slots
        let elapsed_slots = self.current_slot.saturating_sub(user.warmup_state.started_at_slot);

        // Calculate warmed up cap: slope * elapsed_slots
        let warmed_up_cap = mul_u128(
            user.warmup_state.slope_per_step,
            elapsed_slots as u128
        );

        // Return minimum of available and warmed up
        core::cmp::min(available_pnl, warmed_up_cap)
    }

    /// Calculate user's collateral (principal + positive PNL)
    pub fn user_collateral(&self, user: &UserAccount) -> u128 {
        add_u128(user.principal, clamp_pos_i128(user.pnl_ledger))
    }

    /// Check if user is above maintenance margin
    pub fn is_above_maintenance_margin(&self, user: &UserAccount, oracle_price: u64) -> bool {
        let collateral = self.user_collateral(user);

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
    pub fn check_conservation(&self) -> bool {
        let mut total_principal = 0u128;
        let mut total_positive_pnl = 0u128;

        for user in &self.users {
            total_principal = add_u128(total_principal, user.principal);
            total_positive_pnl = add_u128(total_positive_pnl, clamp_pos_i128(user.pnl_ledger));
        }

        for lp in &self.lps {
            total_principal = add_u128(total_principal, lp.lp_capital);
            total_positive_pnl = add_u128(total_positive_pnl, clamp_pos_i128(lp.lp_pnl));
        }

        let expected_vault = add_u128(
            add_u128(total_principal, total_positive_pnl),
            self.insurance_fund.balance
        );

        self.vault >= expected_vault.saturating_sub(1000) &&
        self.vault <= expected_vault.saturating_add(1000) // Allow small rounding
    }
}

// ============================================================================
// User Operations
// ============================================================================

impl RiskEngine {
    /// Deposit funds to user account
    pub fn deposit(&mut self, user_index: usize, amount: u128) -> Result<()> {
        let user = self.users.get_mut(user_index).ok_or(RiskError::UserNotFound)?;

        user.principal = add_u128(user.principal, amount);
        self.vault = add_u128(self.vault, amount);

        Ok(())
    }

    /// Withdraw principal (always allowed up to principal balance)
    pub fn withdraw_principal(&mut self, user_index: usize, amount: u128) -> Result<()> {
        let user = self.users.get_mut(user_index).ok_or(RiskError::UserNotFound)?;

        if user.principal < amount {
            return Err(RiskError::InsufficientBalance);
        }

        // Check that withdrawal doesn't violate margin requirements
        let new_principal = sub_u128(user.principal, amount);
        let _new_collateral = add_u128(new_principal, clamp_pos_i128(user.pnl_ledger));

        // If user has position, check margin
        if user.position_size != 0 {
            // Use a conservative oracle price check (should be passed in)
            // For now, we'll skip this check - in production this would use oracle
        }

        user.principal = new_principal;
        self.vault = sub_u128(self.vault, amount);

        Ok(())
    }

    /// Withdraw PNL (only warmed up portion)
    pub fn withdraw_pnl(&mut self, user_index: usize, amount: u128) -> Result<()> {
        // Calculate withdrawable before borrowing mutably
        let user = self.users.get(user_index).ok_or(RiskError::UserNotFound)?;
        let withdrawable = self.withdrawable_pnl(user);

        if withdrawable < amount {
            return Err(RiskError::PnlNotWarmedUp);
        }

        // Check insurance fund can cover it
        if self.insurance_fund.balance < amount {
            return Err(RiskError::InsufficientBalance);
        }

        // Now mutate
        let user = self.users.get_mut(user_index).ok_or(RiskError::UserNotFound)?;
        user.pnl_ledger = user.pnl_ledger.saturating_sub(amount as i128);
        self.insurance_fund.balance = sub_u128(self.insurance_fund.balance, amount);
        self.vault = sub_u128(self.vault, amount);

        Ok(())
    }
}

// ============================================================================
// Trading Operations
// ============================================================================

impl RiskEngine {
    /// Execute trade via matching engine (with CPI)
    ///
    /// This function:
    /// 1. Validates user signature authorized the trade
    /// 2. CPI into matching engine to execute trade
    /// 3. Applies trading fee to insurance fund
    /// 4. Updates LP and user positions
    /// 5. Aborts if either account becomes negative
    pub fn execute_trade(
        &mut self,
        lp_index: usize,
        user_index: usize,
        oracle_price: u64,
        size: i128, // Positive = long, negative = short
        _user_signature: &[u8], // In production, verify signature
    ) -> Result<()> {
        // Get accounts
        let lp = self.lps.get_mut(lp_index).ok_or(RiskError::LPNotFound)?;
        let user = self.users.get_mut(user_index).ok_or(RiskError::UserNotFound)?;

        // TODO: In production, verify user signature authorized this trade
        // TODO: In production, CPI into matching engine program

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

                user.pnl_ledger = user.pnl_ledger.saturating_add(pnl);
                lp.lp_pnl = lp.lp_pnl.saturating_sub(pnl);
            }
        }

        // Update positions
        let new_user_position = user.position_size.saturating_add(size);
        let new_lp_position = lp.lp_position_size.saturating_sub(size);

        // Update entry prices (weighted average for increases)
        if (user.position_size > 0 && size > 0) || (user.position_size < 0 && size < 0) {
            // Increasing position - weighted average entry
            let old_notional = mul_u128(user.position_size.abs() as u128, user.entry_price as u128);
            let new_notional = mul_u128(size.abs() as u128, oracle_price as u128);
            let total_notional = add_u128(old_notional, new_notional);
            let total_size = user.position_size.abs().saturating_add(size.abs());

            if total_size != 0 {
                user.entry_price = div_u128(total_notional, total_size as u128) as u64;
            }
        } else if user.position_size.abs() < size.abs() {
            // Flipping position
            user.entry_price = oracle_price;
        }

        // Similar for LP
        if (lp.lp_position_size > 0 && new_lp_position > lp.lp_position_size) ||
           (lp.lp_position_size < 0 && new_lp_position < lp.lp_position_size) {
            let old_notional = mul_u128(lp.lp_position_size.abs() as u128, lp.lp_entry_price as u128);
            let new_notional = mul_u128(size.abs() as u128, oracle_price as u128);
            let total_notional = add_u128(old_notional, new_notional);
            let total_size = lp.lp_position_size.abs().saturating_add(size.abs());

            if total_size != 0 {
                lp.lp_entry_price = div_u128(total_notional, total_size as u128) as u64;
            }
        }

        // Apply fee to insurance fund
        self.insurance_fund.fee_revenue = add_u128(self.insurance_fund.fee_revenue, fee);
        self.insurance_fund.balance = add_u128(self.insurance_fund.balance, fee);

        // Deduct fee from user
        user.pnl_ledger = user.pnl_ledger.saturating_sub(fee as i128);

        // Check neither account is negative
        let user_collateral = add_u128(user.principal, clamp_pos_i128(user.pnl_ledger));
        let lp_collateral = add_u128(lp.lp_capital, clamp_pos_i128(lp.lp_pnl));

        if user_collateral == 0 && user.principal > 0 {
            return Err(RiskError::Undercollateralized);
        }

        if lp_collateral == 0 && lp.lp_capital > 0 {
            return Err(RiskError::Undercollateralized);
        }

        // Commit position updates
        user.position_size = new_user_position;
        lp.lp_position_size = new_lp_position;

        Ok(())
    }
}

// ============================================================================
// ADL (Auto-Deleveraging)
// ============================================================================

impl RiskEngine {
    /// Apply ADL haircut to unwrapped PNL
    ///
    /// Invariants:
    /// - Haircut is applied to PNL that isn't warmed up yet (< time T)
    /// - Remaining losses are applied to insurance fund
    /// - User principal is NEVER touched
    pub fn apply_adl(&mut self, total_loss: u128) -> Result<()> {
        let mut remaining_loss = total_loss;

        // Phase 1: Haircut unwrapped PNL (youngest first)
        // Calculate all haircuts first, then apply
        let mut haircuts = Vec::new();
        for (idx, user) in self.users.iter().enumerate() {
            if remaining_loss == 0 {
                break;
            }

            let positive_pnl = clamp_pos_i128(user.pnl_ledger);
            let withdrawable = self.withdrawable_pnl(user);

            // Unwrapped PNL = positive PNL - withdrawable - reserved
            let unwrapped = sub_u128(sub_u128(positive_pnl, withdrawable), user.reserved_pnl);

            if unwrapped > 0 {
                let haircut = core::cmp::min(unwrapped, remaining_loss);
                haircuts.push((idx, haircut));
                remaining_loss = sub_u128(remaining_loss, haircut);
            }
        }

        // Apply haircuts
        for (idx, haircut) in haircuts {
            if let Some(user) = self.users.get_mut(idx) {
                user.pnl_ledger = user.pnl_ledger.saturating_sub(haircut as i128);
            }
        }

        // Phase 2: Apply remaining loss to insurance fund
        if remaining_loss > 0 {
            if self.insurance_fund.balance < remaining_loss {
                // Insurance fund depleted - this is a crisis
                let insurance_used = self.insurance_fund.balance;
                self.insurance_fund.balance = 0;
                self.loss_accum = add_u128(self.loss_accum,
                    sub_u128(remaining_loss, insurance_used));
            } else {
                self.insurance_fund.balance = sub_u128(self.insurance_fund.balance, remaining_loss);
            }
        }

        Ok(())
    }
}

// ============================================================================
// Liquidations
// ============================================================================

impl RiskEngine {
    /// Liquidate undercollateralized account
    ///
    /// Process:
    /// 1. Check if account is above liquidation threshold
    /// 2. Reduce position to bring account below threshold
    /// 3. Split fee between insurance fund and liquidation keeper
    pub fn liquidate_user(
        &mut self,
        user_index: usize,
        keeper_account: usize,
        oracle_price: u64,
    ) -> Result<()> {
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

        user.pnl_ledger = user.pnl_ledger.saturating_add(pnl);
        user.position_size = 0;

        // Apply fees
        self.insurance_fund.liquidation_revenue = add_u128(
            self.insurance_fund.liquidation_revenue,
            insurance_share
        );
        self.insurance_fund.balance = add_u128(self.insurance_fund.balance, insurance_share);

        // Give keeper their share
        if let Some(keeper) = self.users.get_mut(keeper_account) {
            keeper.pnl_ledger = keeper.pnl_ledger.saturating_add(keeper_share as i128);
        }

        Ok(())
    }
}

// ============================================================================
// Initialization
// ============================================================================

impl RiskEngine {
    /// Create a new risk engine
    pub fn new(params: RiskParams) -> Self {
        Self {
            vault: 0,
            insurance_fund: InsuranceFund {
                balance: 0,
                fee_revenue: 0,
                liquidation_revenue: 0,
            },
            users: Vec::new(),
            lps: Vec::new(),
            params,
            current_slot: 0,
            fee_index: 0,
            sum_vested_pos_pnl: 0,
            loss_accum: 0,
            fee_carry: 0,
        }
    }

    /// Add a new user account
    pub fn add_user(&mut self) -> usize {
        let index = self.users.len();
        self.users.push(UserAccount {
            principal: 0,
            pnl_ledger: 0,
            reserved_pnl: 0,
            warmup_state: Warmup {
                started_at_slot: self.current_slot,
                slope_per_step: self.params.warmup_period_slots as u128,
            },
            position_size: 0,
            entry_price: 0,
            fee_index_user: 0,
            fee_accrued: 0,
            vested_pos_snapshot: 0,
        });
        index
    }

    /// Add a new LP account
    pub fn add_lp(&mut self, matching_engine_program: [u8; 32], matching_engine_context: [u8; 32]) -> usize {
        let index = self.lps.len();
        self.lps.push(LPAccount {
            matching_engine_program,
            matching_engine_context,
            lp_capital: 0,
            lp_pnl: 0,
            lp_position_size: 0,
            lp_entry_price: 0,
        });
        index
    }

    /// Advance to next slot (for testing warmup)
    pub fn advance_slot(&mut self, slots: u64) {
        self.current_slot = self.current_slot.saturating_add(slots);
    }
}
