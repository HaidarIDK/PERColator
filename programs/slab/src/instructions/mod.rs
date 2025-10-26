pub mod initialize;
pub mod commit_fill;
pub mod place_order;
pub mod cancel_order;

pub use initialize::*;
pub use commit_fill::*;
pub use place_order::*;
pub use cancel_order::*;

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
}
