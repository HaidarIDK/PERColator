#![cfg_attr(target_os = "solana", no_std)]

pub mod state;
pub mod instructions;
pub mod pda;

// Always expose entrypoint for testing
pub mod entrypoint;

#[cfg(test)]
mod tests;

// Panic handler for no_std builds (only for Solana BPF)
#[cfg(all(target_os = "solana", not(test)))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

pub use state::*;
pub use instructions::SlabInstruction;

pinocchio_pubkey::declare_id!("8HkaHrEmhP7R9UCUhHPibQTCjBFPgNVb4HEuvPzN28ox");
