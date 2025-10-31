pub mod initialize;
pub mod commit_fill;
pub mod place_order;
pub mod cancel_order;
pub mod update_funding;
pub mod halt_trading;
pub mod resume_trading;

pub use initialize::*;
pub use commit_fill::*;
pub use place_order::*;
pub use cancel_order::*;
pub use update_funding::*;
pub use halt_trading::*;
pub use resume_trading::*;

/// Instruction discriminator
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlabInstruction {
    /// Initialize slab
    Initialize = 0,
    /// Commit fill (v0 - single instruction for fills)
    CommitFill = 1,
    /// Place order (v1 - add resting limit order)
    PlaceOrder = 2,
    /// Cancel order (v1 - remove resting limit order)
    CancelOrder = 3,
    // Note: Discriminator 4 is used for adapter_liquidity (not in this enum)
    /// Update funding rate (periodic crank)
    UpdateFunding = 5,
    /// Halt trading (LP owner only)
    HaltTrading = 6,
    /// Resume trading (LP owner only)
    ResumeTrading = 7,
}
