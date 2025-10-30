//! Deposit instruction - deposit collateral to vault

use crate::state::Vault;
use percolator_common::*;

/// Process deposit instruction
///
/// Deposits collateral from user's token account to the router vault.
/// Updates vault balance and total_pledged tracking.
pub fn process_deposit(
    vault: &mut Vault,
    amount: u128,
) -> Result<(), PercolatorError> {
    // Validate amount
    if amount == 0 {
        return Err(PercolatorError::InvalidQuantity);
    }

    // Deposit to vault
    vault.deposit(amount);

    Ok(())
}
