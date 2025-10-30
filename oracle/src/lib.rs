//! Percolator Oracle Program
//!
//! Minimal price oracle for Surfpool testing. Provides price feeds for instruments.
//!
//! ## Instructions
//!
//! - **Initialize** (0): Create a new price oracle for an instrument
//! - **UpdatePrice** (1): Update the price data (authority only)
//!
//! ## Account Structure
//!
//! ```text
//! PriceOracle (128 bytes):
//!   magic: u64           - Magic bytes for validation
//!   version: u8          - Version (currently 0)
//!   bump: u8             - PDA bump seed
//!   authority: Pubkey    - Who can update prices
//!   instrument: Pubkey   - Which instrument
//!   price: i64           - Current price (scaled)
//!   timestamp: i64       - Last update time
//!   confidence: i64      - Price confidence interval
//! ```

#![cfg_attr(target_os = "solana", no_std)]

// Always expose entrypoint for testing
pub mod entrypoint;

pub mod instructions;
pub mod state;

// Panic handler for no_std builds (only for Solana BPF)
#[cfg(all(target_os = "solana", not(test)))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

pub use state::{PriceOracle, PRICE_ORACLE_SIZE};

/// Program ID - Our generated vanity address
pub const ID: &str = "oracpooXY8Nnpx2JTLkrLiJsDaMefERUFFRktkAZ3ki";
