#![cfg_attr(target_os = "solana", no_std)]

pub mod state;
pub mod instructions;
pub mod pda;
pub mod liquidation;
pub mod chooser;

// Always expose entrypoint for testing, but only register as entrypoint when feature enabled
pub mod entrypoint;

pub use state::*;
pub use instructions::*;

// Use our generated vanity address
pinocchio_pubkey::declare_id!("RoutqcxkpVH8jJ2cULG9u6WbdRskQwXkJe8CqZehcyr");
