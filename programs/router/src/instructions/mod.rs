/// Router instruction handlers (v0 minimal)

pub mod initialize;
pub mod initialize_portfolio;
pub mod deposit;
pub mod withdraw;
pub mod execute_cross_slab;
pub mod liquidate_user;
pub mod burn_lp_shares;
pub mod cancel_lp_orders;
// pub mod register_slab;  // REMOVED - permissionless matchers
pub mod router_reserve;
pub mod router_release;
pub mod router_liquidity;
pub mod router_seat_init;

pub use initialize::*;
pub use initialize_portfolio::*;
pub use deposit::*;
pub use withdraw::*;
pub use execute_cross_slab::*;
pub use liquidate_user::*;
pub use burn_lp_shares::*;
pub use cancel_lp_orders::*;
// pub use register_slab::*;  // REMOVED - permissionless matchers
pub use router_reserve::*;
pub use router_release::*;
pub use router_liquidity::*;
pub use router_seat_init::*;

/// Instruction discriminator (v0 minimal)
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouterInstruction {
    /// Initialize router registry
    Initialize = 0,
    /// Initialize user portfolio
    InitializePortfolio = 1,
    /// Deposit collateral to vault
    Deposit = 2,
    /// Withdraw collateral from vault
    Withdraw = 3,
    /// Execute cross-slab order (v0 main instruction)
    ExecuteCrossSlab = 4,
    /// Liquidate user positions (reduce-only)
    LiquidateUser = 5,
    /// Burn AMM LP shares (ONLY way to reduce AMM LP exposure)
    BurnLpShares = 6,
    /// Cancel Slab LP orders (ONLY way to reduce Slab LP exposure)
    CancelLpOrders = 7,
    // RegisterSlab = 8,  // REMOVED - permissionless matchers (users choose their own)
    /// Reserve collateral from portfolio into LP seat
    RouterReserve = 9,
    /// Release collateral from LP seat back to portfolio
    RouterRelease = 10,
    /// Process liquidity operation via matcher adapter
    RouterLiquidity = 11,
    /// Initialize LP seat for adapter pattern
    RouterSeatInit = 12,
}

// Note: Instruction dispatching is handled in entrypoint.rs
// The functions in this module are called from the entrypoint after
// account deserialization and validation.
