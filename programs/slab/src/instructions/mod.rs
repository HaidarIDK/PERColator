pub mod initialize;
pub mod commit_fill;
pub mod place_order;
pub mod cancel_order;
pub mod update_funding;
pub mod halt_trading;
pub mod resume_trading;
pub mod modify_order;
pub mod initialize_receipt;

pub use initialize::*;
pub use commit_fill::*;
pub use place_order::*;
pub use cancel_order::*;
pub use update_funding::*;
pub use halt_trading::*;
pub use resume_trading::*;
pub use modify_order::*;
pub use initialize_receipt::*;

/// Instruction discriminator
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlabInstruction {
    /// Initialize slab
    Initialize = 0,
    /// Commit fill (router only - match orders)
    CommitFill = 1,
    // Note: Discriminator 2 is adapter_liquidity (router LP operations - not in this enum)
    /// Place order (TESTING ONLY - deprecated for margin DEX, use adapter_liquidity disc 2)
    PlaceOrder = 3,
    /// Cancel order (TESTING ONLY - deprecated for margin DEX, use adapter_liquidity disc 2)
    CancelOrder = 4,
    /// Update funding rate (periodic crank)
    UpdateFunding = 5,
    /// Halt trading (LP owner only)
    HaltTrading = 6,
    /// Resume trading (LP owner only)
    ResumeTrading = 7,
    /// Modify order (TESTING ONLY - change price/qty while preserving order_id)
    ModifyOrder = 8,
    /// Initialize receipt PDA (must be called before ExecuteCrossSlab)
    InitializeReceipt = 9,
}
