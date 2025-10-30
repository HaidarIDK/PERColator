/// Router instruction handlers (v0 minimal)

pub mod initialize;
pub mod initialize_portfolio;
pub mod create_portfolio;
pub mod swap_via_amm;
pub mod deposit;
pub mod withdraw;
pub mod execute_cross_slab;
pub mod liquidate_user;
pub mod burn_lp_shares;
pub mod cancel_lp_orders;

pub use initialize::*;
pub use initialize_portfolio::*;
pub use create_portfolio::*;
pub use swap_via_amm::*;
pub use deposit::*;
pub use withdraw::*;
pub use execute_cross_slab::*;
pub use liquidate_user::*;
pub use burn_lp_shares::*;
pub use cancel_lp_orders::*;

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
    /// Create portfolio PDA (system create) and initialize
    CreatePortfolio = 8,
    /// Route a swap to AMM via CPI
    SwapViaAmm = 9,
}

// Note: Instruction dispatching is handled in entrypoint.rs
// The functions in this module are called from the entrypoint after
// account deserialization and validation.
