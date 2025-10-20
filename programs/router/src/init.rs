//! Account initialization helpers for Router program

use crate::state::{Vault, Escrow, Cap, Portfolio, SlabRegistry};
use percolator_common::*;

/// Initialize a new Vault account
///
/// # Arguments
/// * `vault` - Mutable reference to allocated Vault
/// * `router_id` - Router program ID
/// * `mint` - Token mint pubkey
/// * `bump` - PDA bump seed
pub fn initialize_vault(
    vault: &mut Vault,
    router_id: pinocchio::pubkey::Pubkey,
    mint: pinocchio::pubkey::Pubkey,
    token_account: pinocchio::pubkey::Pubkey,
    bump: u8,
) -> Result<(), PercolatorError> {
    *vault = Vault {
        router_id,
        mint,
        token_account,
        balance: 0,
        total_pledged: 0,
        bump,
        _padding: [0; 7],
    };
    Ok(())
}

/// Initialize a new Escrow account
///
/// # Arguments
/// * `escrow` - Mutable reference to allocated Escrow
/// * `router_id` - Router program ID
/// * `slab_id` - Slab program ID
/// * `user` - User pubkey
/// * `mint` - Token mint pubkey
/// * `bump` - PDA bump seed
pub fn initialize_escrow(
    escrow: &mut Escrow,
    router_id: pinocchio::pubkey::Pubkey,
    slab_id: pinocchio::pubkey::Pubkey,
    user: pinocchio::pubkey::Pubkey,
    mint: pinocchio::pubkey::Pubkey,
    bump: u8,
) -> Result<(), PercolatorError> {
    *escrow = Escrow {
        router_id,
        slab_id,
        user,
        mint,
        balance: 0,
        nonce: 0,
        frozen: false,
        bump,
        _padding: [0; 6],
    };
    Ok(())
}

/// Initialize a new Portfolio account
///
/// # Arguments
/// * `portfolio` - Mutable reference to allocated Portfolio
/// * `router_id` - Router program ID
/// * `user` - User pubkey
/// * `bump` - PDA bump seed
pub fn initialize_portfolio(
    portfolio: &mut Portfolio,
    router_id: pinocchio::pubkey::Pubkey,
    user: pinocchio::pubkey::Pubkey,
    bump: u8,
) -> Result<(), PercolatorError> {
    *portfolio = Portfolio::new(router_id, user, bump);
    Ok(())
}

/// Initialize a new SlabRegistry account
///
/// # Arguments
/// * `registry` - Mutable reference to allocated SlabRegistry
/// * `authority` - Program authority pubkey
pub fn initialize_registry(
    registry: &mut SlabRegistry,
    router_id: pinocchio::pubkey::Pubkey,
    governance: pinocchio::pubkey::Pubkey,
    bump: u8,
) -> Result<(), PercolatorError> {
    *registry = SlabRegistry::new(router_id, governance, bump);
    Ok(())
}

/// Initialize a new Cap (Capability) account
///
/// # Arguments
/// * `cap` - Mutable reference to allocated Cap
/// * `router_id` - Router program ID
/// * `route_id` - Unique route identifier
/// * `scope_user` - Scoped user pubkey
/// * `scope_slab` - Scoped slab pubkey
/// * `scope_mint` - Scoped mint pubkey
/// * `amount_max` - Maximum authorized amount
/// * `current_ts` - Current timestamp in milliseconds
/// * `ttl_ms` - Time-to-live in milliseconds (capped at 2 minutes)
/// * `bump` - PDA bump seed
pub fn initialize_cap(
    cap: &mut Cap,
    router_id: pinocchio::pubkey::Pubkey,
    route_id: u64,
    scope_user: pinocchio::pubkey::Pubkey,
    scope_slab: pinocchio::pubkey::Pubkey,
    scope_mint: pinocchio::pubkey::Pubkey,
    amount_max: u128,
    current_ts: u64,
    ttl_ms: u64,
    bump: u8,
) -> Result<(), PercolatorError> {
    *cap = Cap::new(
        router_id,
        route_id,
        scope_user,
        scope_slab,
        scope_mint,
        amount_max,
        current_ts,
        ttl_ms,
        bump,
    );
    Ok(())
}

/// Get required account sizes
pub const fn get_vault_size() -> usize {
    core::mem::size_of::<Vault>()
}

pub const fn get_escrow_size() -> usize {
    core::mem::size_of::<Escrow>()
}

pub const fn get_portfolio_size() -> usize {
    core::mem::size_of::<Portfolio>()
}

pub const fn get_registry_size() -> usize {
    core::mem::size_of::<SlabRegistry>()
}

pub const fn get_cap_size() -> usize {
    core::mem::size_of::<Cap>()
}

/// Calculate rent for Router accounts
#[cfg(target_os = "solana")]
pub fn calculate_rent(size: usize) -> u64 {
    ((size + 128) as u64) * 3_480 * 2
}

#[cfg(not(target_os = "solana"))]
pub fn calculate_rent(size: usize) -> u64 {
    ((size + 128) as u64) * 3_480 * 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_vault_size() {
        let size = get_vault_size();
        assert!(size > 0);
        assert!(size < 1024); // Should be small
    }

    #[test]
    fn test_get_portfolio_size() {
        let size = get_portfolio_size();
        assert!(size > 0);
        // Portfolio has exposures array, could be larger
        assert!(size > 100);
    }

    #[test]
    fn test_calculate_rent_vault() {
        let rent = calculate_rent(get_vault_size());
        assert!(rent > 0);
        assert!(rent < 10_000_000); // Should be < 0.01 SOL
    }

    #[test]
    fn test_initialize_vault() {
        let mut vault = Vault {
            router_id: pinocchio::pubkey::Pubkey::default(),
            mint: pinocchio::pubkey::Pubkey::default(),
            token_account: pinocchio::pubkey::Pubkey::default(),
            balance: 999,
            total_pledged: 888,
            bump: 0,
            _padding: [0; 7],
        };

        let router = pinocchio::pubkey::Pubkey::default();
        let mint = pinocchio::pubkey::Pubkey::default();
        let token_account = pinocchio::pubkey::Pubkey::default();

        initialize_vault(&mut vault, router, mint, token_account, 1).unwrap();

        assert_eq!(vault.balance, 0);
        assert_eq!(vault.total_pledged, 0);
        assert_eq!(vault.bump, 1);
    }

    #[test]
    fn test_initialize_escrow() {
        let mut escrow = Escrow {
            router_id: pinocchio::pubkey::Pubkey::default(),
            slab_id: pinocchio::pubkey::Pubkey::default(),
            user: pinocchio::pubkey::Pubkey::default(),
            mint: pinocchio::pubkey::Pubkey::default(),
            balance: 999,
            nonce: 5,
            frozen: true,
            bump: 0,
            _padding: [0; 6],
        };

        let router = pinocchio::pubkey::Pubkey::default();
        let slab = pinocchio::pubkey::Pubkey::default();
        let user = pinocchio::pubkey::Pubkey::default();
        let mint = pinocchio::pubkey::Pubkey::default();

        initialize_escrow(&mut escrow, router, slab, user, mint, 2).unwrap();

        assert_eq!(escrow.balance, 0);
        assert_eq!(escrow.nonce, 0);
        assert_eq!(escrow.frozen, false);
        assert_eq!(escrow.bump, 2);
    }

    #[test]
    fn test_initialize_portfolio() {
        let mut portfolio = Portfolio::new(
            pinocchio::pubkey::Pubkey::default(),
            pinocchio::pubkey::Pubkey::default(),
            0,
        );
        portfolio.equity = 999;
        portfolio.im = 888;

        let router = pinocchio::pubkey::Pubkey::default();
        let user = pinocchio::pubkey::Pubkey::default();

        initialize_portfolio(&mut portfolio, router, user, 3).unwrap();

        assert_eq!(portfolio.equity, 0);
        assert_eq!(portfolio.im, 0);
        assert_eq!(portfolio.bump, 3);
    }

    #[test]
    fn test_initialize_cap() {
        let mut cap = Cap {
            router_id: pinocchio::pubkey::Pubkey::default(),
            route_id: 999,
            scope_user: pinocchio::pubkey::Pubkey::default(),
            scope_slab: pinocchio::pubkey::Pubkey::default(),
            scope_mint: pinocchio::pubkey::Pubkey::default(),
            amount_max: 888,
            remaining: 777,
            expiry_ts: 666,
            nonce: 555,
            burned: true,
            bump: 0,
            _padding: [0; 6],
        };

        let router = pinocchio::pubkey::Pubkey::default();
        let user = pinocchio::pubkey::Pubkey::default();
        let slab = pinocchio::pubkey::Pubkey::default();
        let mint = pinocchio::pubkey::Pubkey::default();

        initialize_cap(
            &mut cap,
            router,
            123,
            user,
            slab,
            mint,
            10_000,
            1_000_000,
            60_000,
            4,
        ).unwrap();

        assert_eq!(cap.route_id, 123);
        assert_eq!(cap.amount_max, 10_000);
        assert_eq!(cap.remaining, 10_000);
        assert_eq!(cap.expiry_ts, 1_060_000);
        assert!(!cap.burned);
        assert_eq!(cap.bump, 4);
    }
}

