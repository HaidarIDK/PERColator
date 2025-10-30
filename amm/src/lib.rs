//! Percolator AMM - Constant Product Market Maker (x·y=k)
//!
//! This program implements a constant-product automated market maker that
//! implements the same v0 boundary contract as the orderbook slab:
//! - Same SlabHeader and QuoteCache layout
//! - Same commit_fill CPI interface
//! - Router-readable quote synthesis

#![allow(clippy::arithmetic_side_effects)]

pub mod entrypoint;
pub mod instructions;
pub mod math;
pub mod state;

pub use state::*;

/// Program ID - Our generated vanity address
pub const ID: &str = "AMMjkEeFdasQ8fs9a9HQyJdciPHtDHVEat8yxiXrTP6p";
