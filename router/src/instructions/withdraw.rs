//! Withdraw instruction - withdraw collateral from vault

use crate::state::Vault;
use percolator_common::*;

/// Process withdraw instruction
///
/// Withdraws collateral from the router vault to user's token account.
/// Ensures sufficient available (non-pledged) balance exists.
pub fn process_withdraw(
    vault: &mut Vault,
    amount: u128,
) -> Result<(), PercolatorError> {
    // Validate amount
    if amount == 0 {
        return Err(PercolatorError::InvalidQuantity);
    }

    // Attempt withdrawal
    vault.withdraw(amount)
        .map_err(|_| PercolatorError::InsufficientFunds)?;

    Ok(())
}
