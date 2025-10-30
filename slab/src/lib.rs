#![cfg_attr(target_os = "solana", no_std)]

pub mod state;
pub mod instructions;
pub mod pda;

// Always expose entrypoint for testing
pub mod entrypoint;

#[cfg(test)]
mod tests;

pub use state::*;
pub use instructions::SlabInstruction;

// Use our generated vanity address
pinocchio_pubkey::declare_id!("SLAB98WHcToiuUMMX9NQSg5E5iB8CjpK21T4h9ZXiep");
