//! Withdraw instruction - withdraw SOL collateral from portfolio

use crate::state::{Portfolio, SlabRegistry};
use percolator_common::*;
use pinocchio::{
    account_info::AccountInfo,
    msg,
    ProgramResult,
};

/// Process withdraw instruction (SOL only for MVP)
///
/// Withdraws SOL from portfolio account to user's wallet.
/// Enforces adaptive warmup throttling on PnL withdrawals.
///
/// # Security Checks
/// - Verifies user is a signer
/// - Verifies portfolio belongs to user
/// - Validates withdrawal amount is non-zero
/// - Checks adaptive warmup withdrawal limit (principal + vested PnL)
/// - Ensures portfolio account remains rent-exempt after withdrawal
///
/// # Arguments
/// * `portfolio_account` - The user's portfolio account (sends SOL)
/// * `portfolio` - Mutable reference to portfolio state
/// * `user_account` - The user's wallet account (receives SOL)
/// * `system_program` - The System Program account
/// * `registry` - The registry account (for warmup state)
/// * `amount` - Amount of lamports to withdraw
pub fn process_withdraw(
    portfolio_account: &AccountInfo,
    portfolio: &mut Portfolio,
    user_account: &AccountInfo,
    _system_program: &AccountInfo,
    registry: &SlabRegistry,
    amount: u64,
) -> ProgramResult {
    // SECURITY: Validate amount
    if amount == 0 {
        msg!("Error: Withdrawal amount must be greater than zero");
        return Err(PercolatorError::InvalidQuantity.into());
    }

    // SECURITY: Verify user is a signer
    if !user_account.is_signer() {
        msg!("Error: User must be a signer");
        return Err(PercolatorError::Unauthorized.into());
    }

    // SECURITY: Verify portfolio belongs to this user
    if portfolio.user != *user_account.key() {
        msg!("Error: Portfolio does not belong to user");
        return Err(PercolatorError::Unauthorized.into());
    }

    // Check adaptive warmup withdrawal limit
    // Principal is always withdrawable, but vested PnL is capped by unlocked_frac
    let max_withdrawable = portfolio.max_withdrawable_with_warmup(registry.warmup_state.unlocked_frac);

    // Convert to u64 for comparison (max with 0 to handle negative equity)
    let max_withdrawable_u64 = max_withdrawable.max(0) as u64;

    if amount > max_withdrawable_u64 {
        msg!("Error: Insufficient withdrawable funds");
        return Err(PercolatorError::InsufficientFunds.into());
    }

    // Check portfolio account will remain rent-exempt after withdrawal
    let min_rent_exempt = 1_000_000_000u64; // ~1 SOL for 135KB account (approximate)
    let portfolio_lamports = portfolio_account.lamports();

    if portfolio_lamports < amount + min_rent_exempt {
        msg!("Error: Withdrawal would make portfolio account not rent-exempt");
        return Err(PercolatorError::InsufficientFunds.into());
    }

    // Transfer SOL from portfolio to user
    // Since the router program owns the portfolio account, we can directly manipulate lamports
    // without requiring a System Program CPI (which would need the portfolio to be a signer)
    unsafe {
        *portfolio_account.borrow_mut_lamports_unchecked() -= amount;
        *user_account.borrow_mut_lamports_unchecked() += amount;
    }

    // Update portfolio state
    // Reduce principal and equity
    let amount_i128 = amount as i128;

    portfolio.principal = portfolio.principal
        .checked_sub(amount_i128)
        .ok_or(PercolatorError::Underflow)?;

    portfolio.equity = portfolio.equity
        .checked_sub(amount_i128)
        .ok_or(PercolatorError::Underflow)?;

    msg!("Withdrawal successful");

    Ok(())
}
