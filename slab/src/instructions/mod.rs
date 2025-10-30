pub mod initialize;
pub mod commit_fill;
pub mod reserve;
pub mod commit;
pub mod cancel;

pub use initialize::*;
pub use commit_fill::*;
pub use reserve::*;
pub use commit::*;
pub use cancel::*;

/// Instruction discriminator
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlabInstruction {
    /// Initialize slab
    Initialize = 0,
    /// Commit fill (v0 - single instruction for fills)
    CommitFill = 1,
    /// Reserve liquidity (two-phase trading, step 1)
    Reserve = 2,
    /// Commit a reservation (two-phase trading, step 2)
    Commit = 3,
    /// Cancel a reservation
    Cancel = 4,
}
