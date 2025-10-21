#![no_std]

pub mod state;
pub mod instructions;
pub mod pda;
pub mod init;

#[cfg(feature = "bpf-entrypoint")]
mod entrypoint;

pub use state::*;
pub use instructions::*;
pub use init::*;

pinocchio_pubkey::declare_id!("RoutR1VdCpHqj89WEMJhb6TkGT9cPfr1rVjhM3e2YQr");
