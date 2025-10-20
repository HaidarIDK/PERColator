#![no_std]

pub mod state;
pub mod instructions;
pub mod pda;
pub mod init;

#[cfg(feature = "bpf-entrypoint")]
mod entrypoint;

// Panic handler for no_std builds (not needed in tests)
#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

pub use state::*;
pub use instructions::*;
pub use init::*;

pinocchio_pubkey::declare_id!("RoutR1VdCpHqj89WEMJhb6TkGT9cPfr1rVjhM3e2YQr");
