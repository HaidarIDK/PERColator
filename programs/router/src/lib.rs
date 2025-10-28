#![cfg_attr(target_os = "solana", no_std)]

#[macro_use]
extern crate alloc;

pub mod state;
pub mod instructions;
pub mod pda;
pub mod liquidation;
pub mod chooser;
pub mod lp_adapter_serde;
pub mod authority_proofs;

// Always expose entrypoint for testing, but only register as entrypoint when feature enabled
pub mod entrypoint;

// Panic handler for no_std builds (only for Solana BPF)
#[cfg(all(target_os = "solana", not(test)))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

pub use state::*;
pub use instructions::*;

pinocchio_pubkey::declare_id!("RoutR1VdCpHqj89WEMJhb6TkGT9cPfr1rVjhM3e2YQr");
