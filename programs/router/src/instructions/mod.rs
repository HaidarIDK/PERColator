/// Router instruction handlers

pub mod deposit;
pub mod withdraw;
pub mod initialize;
pub mod cap_ops;
pub mod multi_reserve;
pub mod multi_commit;
pub mod liquidate;

pub use deposit::*;
pub use withdraw::*;
pub use initialize::*;
pub use cap_ops::*;
pub use multi_reserve::*;
pub use multi_commit::*;
pub use liquidate::*;

use percolator_common::*;

/// Instruction discriminator
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouterInstruction {
    /// Initialize router
    Initialize = 0,
    /// Deposit collateral
    Deposit = 1,
    /// Withdraw collateral
    Withdraw = 2,
    /// Multi-slab reserve orchestration
    MultiReserve = 3,
    /// Multi-slab commit orchestration
    MultiCommit = 4,
    /// Liquidation coordinator
    Liquidate = 5,
}

/// Process router instruction
///
/// Routes instruction to appropriate handler based on discriminator.
/// Note: This is a simplified dispatcher - actual BPF entrypoint will
/// handle account deserialization and validation.
pub fn process_instruction(
    instruction: RouterInstruction,
    _data: &[u8],
) -> Result<(), PercolatorError> {
    match instruction {
        RouterInstruction::Initialize => process_initialize(),
        RouterInstruction::Deposit => {
            // TODO: Deserialize vault and amount from _data
            // process_deposit(vault, amount)
            Ok(())
        }
        RouterInstruction::Withdraw => {
            // TODO: Deserialize vault and amount from _data
            // process_withdraw(vault, amount)
            Ok(())
        }
        RouterInstruction::MultiReserve => process_multi_reserve(),
        RouterInstruction::MultiCommit => process_multi_commit(),
        RouterInstruction::Liquidate => process_liquidate(),
    }
}
