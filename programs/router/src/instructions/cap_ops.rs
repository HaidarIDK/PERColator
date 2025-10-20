//! Capability token operations

use crate::state::{Cap, Escrow, Vault};
use crate::pda;
use percolator_common::*;
use pinocchio::pubkey::Pubkey;

/// Create escrow and mint capability token for a reserve
///
/// This is called by the router after a successful slab reserve operation.
/// It creates an escrow account (if needed) and mints a capability token
/// that authorizes the slab to debit the escrow up to max_charge.
pub fn mint_cap_for_reserve(
    router_id: &Pubkey,
    user: &Pubkey,
    slab: &Pubkey,
    mint: &Pubkey,
    route_id: u64,
    max_charge: u128,
    current_ts: u64,
    ttl_ms: u64,
    vault: &mut Vault,
    escrow: &mut Escrow,
) -> Result<Cap, PercolatorError> {
    // Pledge amount from vault to escrow
    vault.pledge(max_charge)
        .map_err(|_| PercolatorError::InsufficientFunds)?;

    // Credit escrow
    escrow.credit(max_charge);

    // Derive cap PDA
    let (_cap_pda, cap_bump) = pda::derive_cap_pda(user, slab, mint, route_id, router_id);

    // Create and return cap
    let cap = Cap::new(
        *router_id,
        route_id,
        *user,
        *slab,
        *mint,
        max_charge,
        current_ts,
        ttl_ms,
        cap_bump,
    );

    Ok(cap)
}

/// Verify and debit from capability
///
/// This is called by the slab during commit to debit the escrow.
/// It enforces all cap constraints: scope, expiry, amount limit.
pub fn cap_debit(
    cap: &mut Cap,
    escrow: &mut Escrow,
    amount: u128,
    user: &Pubkey,
    slab: &Pubkey,
    mint: &Pubkey,
    current_ts: u64,
) -> Result<(), PercolatorError> {
    // Verify and debit from cap
    cap.debit(amount, user, slab, mint, current_ts)
        .map_err(|e| match e {
            crate::state::CapError::Expired => PercolatorError::CapabilityExpired,
            crate::state::CapError::InvalidScope => PercolatorError::InvalidScope,
            crate::state::CapError::InsufficientRemaining => PercolatorError::InsufficientFunds,
        })?;

    // Debit from escrow
    escrow.debit(amount)
        .map_err(|_| PercolatorError::InsufficientFunds)?;

    Ok(())
}

/// Burn capability and refund unused escrow
///
/// Called after commit (success or failure) or on explicit cancel.
/// Refunds any unused escrow balance back to vault.
pub fn burn_cap_and_refund(
    cap: &mut Cap,
    escrow: &mut Escrow,
    vault: &mut Vault,
) -> Result<(), PercolatorError> {
    // Calculate unused amount
    let unused = cap.remaining;

    // Burn cap
    cap.burn();

    // If there's unused escrow, refund it
    if unused > 0 && escrow.balance >= unused {
        escrow.debit(unused)
            .map_err(|_| PercolatorError::InsufficientFunds)?;
        vault.unpledge(unused);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_vault() -> Vault {
        Vault {
            router_id: Pubkey::default(),
            mint: Pubkey::default(),
            token_account: Pubkey::default(),
            balance: 10000,
            total_pledged: 0,
            bump: 0,
            _padding: [0; 7],
        }
    }

    fn create_test_escrow() -> Escrow {
        Escrow {
            router_id: Pubkey::default(),
            slab_id: Pubkey::default(),
            user: Pubkey::default(),
            mint: Pubkey::default(),
            balance: 0,
            nonce: 0,
            frozen: false,
            bump: 0,
            _padding: [0; 6],
        }
    }

    #[test]
    fn test_vault_pledge_for_escrow() {
        let mut vault = create_test_vault();
        let mut escrow = create_test_escrow();

        // Test vault pledge and escrow credit
        assert_eq!(vault.available(), 10000);
        vault.pledge(1000).unwrap();
        escrow.credit(1000);

        assert_eq!(vault.total_pledged, 1000);
        assert_eq!(vault.available(), 9000);
        assert_eq!(escrow.balance, 1000);
    }

    #[test]
    fn test_cap_debit() {
        let router_id = Pubkey::default();
        let user = Pubkey::from([1; 32]);
        let slab = Pubkey::from([2; 32]);
        let mint = Pubkey::from([3; 32]);

        let mut cap = Cap::new(
            router_id,
            12345,
            user,
            slab,
            mint,
            1000,
            1000,
            60_000,
            0,
        );

        let mut escrow = create_test_escrow();
        escrow.credit(1000);

        assert!(cap_debit(&mut cap, &mut escrow, 500, &user, &slab, &mint, 1000).is_ok());
        assert_eq!(cap.remaining, 500);
        assert_eq!(escrow.balance, 500);

        assert!(cap_debit(&mut cap, &mut escrow, 600, &user, &slab, &mint, 1000).is_err());
    }

    #[test]
    fn test_burn_cap_and_refund() {
        let mut vault = create_test_vault();
        vault.pledge(1000).unwrap();

        let mut escrow = create_test_escrow();
        escrow.credit(1000);

        let mut cap = Cap::new(
            Pubkey::default(),
            12345,
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            1000,
            1000,
            60_000,
            0,
        );

        // Debit 400, leaving 600 unused
        escrow.debit(400).unwrap();
        cap.remaining = 600;

        burn_cap_and_refund(&mut cap, &mut escrow, &mut vault).unwrap();

        assert!(cap.burned);
        assert_eq!(escrow.balance, 0);
        assert_eq!(vault.total_pledged, 400); // Only the used 400 remains pledged
    }

    #[test]
    fn test_cap_expiry_enforcement() {
        let user = Pubkey::from([1; 32]);
        let slab = Pubkey::from([2; 32]);
        let mint = Pubkey::from([3; 32]);

        let mut cap = Cap::new(
            Pubkey::default(),
            12345,
            user,
            slab,
            mint,
            1000,
            1000,
            60_000,
            0,
        );

        let mut escrow = create_test_escrow();
        escrow.credit(1000);

        // Within TTL - should succeed
        assert!(cap_debit(&mut cap, &mut escrow, 100, &user, &slab, &mint, 50_000).is_ok());

        // After expiry - should fail
        assert!(cap_debit(&mut cap, &mut escrow, 100, &user, &slab, &mint, 200_000).is_err());
    }

    #[test]
    fn test_cap_scope_enforcement() {
        let user = Pubkey::from([1; 32]);
        let slab = Pubkey::from([2; 32]);
        let mint = Pubkey::from([3; 32]);
        let wrong_user = Pubkey::from([99; 32]);

        let mut cap = Cap::new(
            Pubkey::default(),
            12345,
            user,
            slab,
            mint,
            1000,
            1000,
            60_000,
            0,
        );

        let mut escrow = create_test_escrow();
        escrow.credit(1000);

        // Wrong user - should fail
        assert!(cap_debit(&mut cap, &mut escrow, 100, &wrong_user, &slab, &mint, 1000).is_err());

        // Correct scope - should succeed
        assert!(cap_debit(&mut cap, &mut escrow, 100, &user, &slab, &mint, 1000).is_ok());
    }
}

