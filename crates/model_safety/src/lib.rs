//! Pure Rust safety model for Kani verification
//! No Solana dependencies, no unwrap/panic, all functions total
//!
//! This crate is no_std compatible for use in Solana programs.

#![no_std]
#![forbid(unsafe_code)]

pub mod state;
pub mod math;
pub mod warmup;
pub mod helpers;
pub mod transitions;

// Re-export commonly used types
pub use state::*;
pub use helpers::*;
pub use transitions::*;
