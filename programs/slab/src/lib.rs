#![no_std]

pub mod state;
pub mod instructions;
pub mod matching;
pub mod pda;
pub mod init;

#[cfg(feature = "bpf-entrypoint")]
mod entrypoint;

#[cfg(test)]
mod tests;

// Panic handler for no_std builds (not needed in tests)
#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

pub use state::*;
pub use init::*;

// Re-export modules without glob to avoid ambiguous names
pub use instructions::SlabInstruction;
pub use matching::{insert_order, remove_order, promote_pending};
pub use matching::{calculate_equity, calculate_margin_requirements, is_liquidatable};

pinocchio_pubkey::declare_id!("SLabZ6PsDLh2X6HzEoqxFDMqCVcJXDKCNEYuPzUvGPk");
