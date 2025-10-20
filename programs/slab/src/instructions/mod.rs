pub mod reserve;
pub mod commit;
pub mod cancel;
pub mod batch_open;
pub mod update_funding;

pub use reserve::*;
pub use commit::*;
pub use cancel::*;
pub use batch_open::*;
pub use update_funding::*;

/// Instruction discriminator
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlabInstruction {
    /// Reserve liquidity
    Reserve = 0,
    /// Commit reservation
    Commit = 1,
    /// Cancel reservation
    Cancel = 2,
    /// Open new batch/epoch
    BatchOpen = 3,
    /// Initialize slab
    Initialize = 4,
    /// Add instrument
    AddInstrument = 5,
    /// Update funding rate
    UpdateFunding = 6,
}
