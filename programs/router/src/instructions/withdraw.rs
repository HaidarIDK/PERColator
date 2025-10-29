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

    // Update portfolio state using FORMALLY VERIFIED withdrawal logic
    // This call ensures properties D2-D5 are maintained:
    // - D2: Exact amount decrease
    // - D3: Margin safety preserved
    // - D4: Vesting throttle respected
    // - D5: No immediately liquidatable state
    // See: crates/model_safety/src/deposit_withdraw.rs for Kani proofs
    let amount_i128 = amount as i128;
    let portfolio_lamports = portfolio_account.lamports();

    crate::state::model_bridge::apply_withdraw_verified(
        portfolio,
        registry,
        amount_i128,
        portfolio_lamports,
    )
    .map_err(|e| match e {
        model_safety::deposit_withdraw::DepositWithdrawError::InsufficientWithdrawable => {
            msg!("Error: Insufficient withdrawable funds (vesting limit)");
            PercolatorError::InsufficientFunds
        }
        model_safety::deposit_withdraw::DepositWithdrawError::InsufficientRentExempt => {
            msg!("Error: Withdrawal would make portfolio account not rent-exempt");
            PercolatorError::InsufficientFunds
        }
        model_safety::deposit_withdraw::DepositWithdrawError::WouldBeLiquidatable => {
            msg!("Error: Withdrawal would create liquidatable position");
            PercolatorError::InsufficientFunds
        }
        _ => {
            msg!("Error: Invalid withdrawal");
            PercolatorError::InvalidQuantity
        }
    })?;

    // Transfer SOL from portfolio to user
    // Since the router program owns the portfolio account, we can directly manipulate lamports
    // without requiring a System Program CPI (which would need the portfolio to be a signer)
    unsafe {
        *portfolio_account.borrow_mut_lamports_unchecked() -= amount;
        *user_account.borrow_mut_lamports_unchecked() += amount;
    }

    msg!("Withdrawal successful");

    Ok(())
}
