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

pub use state::*;
pub use init::*;

// Re-export modules without glob to avoid ambiguous names
pub use instructions::SlabInstruction;
pub use matching::{insert_order, remove_order, promote_pending};
pub use matching::{calculate_equity, calculate_margin_requirements, is_liquidatable};

pinocchio_pubkey::declare_id!("SLabZ6PsDLh2X6HzEoqxFDMqCVcJXDKCNEYuPzUvGPk");
