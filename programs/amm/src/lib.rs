//! Percolator AMM - Constant Product Market Maker (x·y=k)
//!
//! This program implements a constant-product automated market maker that
//! implements the same v0 boundary contract as the orderbook slab:
//! - Same SlabHeader and QuoteCache layout
//! - Same commit_fill CPI interface
//! - Router-readable quote synthesis

#![allow(clippy::arithmetic_side_effects)]

pub mod adapter;
pub mod entrypoint;
pub mod instructions;
pub mod math;
pub mod state;

pub use state::*;

/// Program ID (will be set during deployment)
pub const ID: &str = "AMM111111111111111111111111111111111111111";
