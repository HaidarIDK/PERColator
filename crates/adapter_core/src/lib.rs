//! Adapter Core - Stable ABI for LP Adapter Interface
//!
//! This crate defines the stable, version-gated interface between Router and
//! external matcher implementations (AMM, orderbook, hybrid strategies).
//!
//! # Design Principles
//! - no_std + alloc for Solana BPF compatibility
//! - Custody isolation: matcher never receives writable vault accounts
//! - Version-gated capabilities for forward compatibility
//! - Pure functions with Kani-provable invariants

#![no_std]

extern crate alloc;
use alloc::vec::Vec;
use alloc::boxed::Box;

// ============================================================================
// Adapter Hello & Versioning
// ============================================================================

/// Adapter hello response (version 1)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdapterHelloV1 {
    /// Protocol version
    pub version: u16,
    /// Capability bitmap
    pub caps: u64,
    /// Matcher program hash (for whitelisting)
    pub matcher_hash: [u8; 32],
}

/// Matcher capabilities
#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    /// Supports AMM liquidity
    SupportsAMM = 1 << 0,
    /// Supports orderbook liquidity
    SupportsOrderBook = 1 << 1,
    /// Supports hybrid strategies
    SupportsHybrid = 1 << 2,
    /// Supports custom hooks
    SupportsHooks = 1 << 3,
}

impl Capability {
    /// Check if capability is enabled
    pub const fn is_enabled(caps: u64, capability: Capability) -> bool {
        (caps & (capability as u64)) != 0
    }
}

// ============================================================================
// Identifiers
// ============================================================================

/// LP seat identifier (PDA: router, matcher_state, portfolio, context_id)
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SeatId(pub [u8; 32]);

/// Asset identifier (e.g., "BASE\0\0\0\0" or "QUOTE\0\0\0")
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetId(pub [u8; 8]);

impl AssetId {
    /// Base asset constant
    pub const BASE: Self = Self(*b"BASE\0\0\0\0");
    /// Quote asset constant
    pub const QUOTE: Self = Self(*b"QUOTE\0\0\0");
}

// ============================================================================
// Capital Management
// ============================================================================

/// Capital commitment specification
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CommitSpec {
    /// Asset to commit
    pub asset: AssetId,
    /// Amount in Q64 fixed-point (amount * 2^64)
    pub amount_q64: u128,
    /// Risk class for haircut calculation
    pub risk_class: u8,
    /// Max leverage in basis points
    pub max_leverage_bps: u16,
    /// Padding
    pub _padding: [u8; 5],
}

/// Capital operation intent
#[derive(Debug, Clone)]
pub enum CapitalIntent {
    /// Reserve collateral for LP operations
    Reserve { adds: Vec<CommitSpec> },
    /// Release reserved collateral
    Release { asset: AssetId, amount_q64: u128 },
    /// Freeze seat (no new operations)
    Freeze,
    /// Unfreeze seat
    Unfreeze,
}

// ============================================================================
// Risk Guards
// ============================================================================

/// Risk guard constraints for liquidity operations
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RiskGuard {
    /// Maximum slippage in basis points
    pub max_slippage_bps: u16,
    /// Maximum fee in basis points
    pub max_fee_bps: u16,
    /// Oracle price band in basis points
    pub oracle_bound_bps: u16,
    /// Padding
    pub _padding: [u8; 2],
}

impl RiskGuard {
    /// Default conservative guard
    pub const fn conservative() -> Self {
        Self {
            max_slippage_bps: 50,   // 0.5%
            max_fee_bps: 30,        // 0.3%
            oracle_bound_bps: 100,  // 1.0%
            _padding: [0; 2],
        }
    }

    /// Permissive guard for testing
    pub const fn permissive() -> Self {
        Self {
            max_slippage_bps: 1000,  // 10%
            max_fee_bps: 300,         // 3%
            oracle_bound_bps: 500,    // 5%
            _padding: [0; 2],
        }
    }
}

// ============================================================================
// Liquidity Operations
// ============================================================================

/// Order side
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Bid = 0,
    Ask = 1,
}

/// Orderbook order specification
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ObOrder {
    /// Order side
    pub side: Side,
    /// Price in Q64 fixed-point
    pub px_q64: u128,
    /// Quantity in Q64 fixed-point
    pub qty_q64: u128,
    /// Time-in-force (slots)
    pub tif_slots: u32,
    /// Padding
    pub _padding: [u8; 4],
}

/// Order removal selector
#[derive(Debug, Clone)]
pub enum RemoveSel {
    /// Remove AMM position by burning shares
    AmmByShares { shares: u128 },
    /// Remove orderbook orders by IDs
    ObByIds { ids: Vec<u128> },
    /// Remove all orderbook orders
    ObAll,
}

/// Liquidity operation intent
#[derive(Debug, Clone)]
pub enum LiquidityIntent {
    /// Add AMM liquidity in price range
    AmmAdd {
        lower_px_q64: u128,
        upper_px_q64: u128,
        quote_notional_q64: u128,
        curve_id: u32,
        fee_bps: u16,
    },
    /// Add orderbook orders
    ObAdd {
        orders: Vec<ObOrder>,
        post_only: bool,
        reduce_only: bool,
    },
    /// Custom hook (matcher-specific)
    Hook {
        hook_id: u32,
        payload: Vec<u8>,
    },
    /// Remove liquidity
    Remove {
        selector: RemoveSel,
    },
    /// Modify liquidity (remove then add)
    Modify {
        remove: RemoveSel,
        add: Option<Box<LiquidityIntent>>,
    },
}

/// Exposure delta (base and quote)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Exposure {
    /// Base asset exposure in Q64
    pub base_q64: i128,
    /// Quote asset exposure in Q64
    pub quote_q64: i128,
}

/// Liquidity operation result
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LiquidityResult {
    /// LP shares delta (+mint, -burn; 0 for OB)
    pub lp_shares_delta: i128,
    /// Exposure delta within venue
    pub exposure_delta: Exposure,
    /// Maker fee credits (venue units)
    pub maker_fee_credits: i128,
    /// Realized PnL delta (rebates, etc.)
    pub realized_pnl_delta: i128,
}

// ============================================================================
// Settlement
// ============================================================================

/// Fill delta for settlement
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FillDelta {
    /// Taker portfolio pubkey
    pub taker_portfolio: [u8; 32],
    /// Maker seat ID
    pub maker_seat: SeatId,
    /// Base delta in Q64 (taker perspective)
    pub base_delta_q64: i128,
    /// Quote delta in Q64 (taker perspective)
    pub quote_delta_q64: i128,
    /// Fee to maker
    pub fee_to_maker: i64,
    /// Fee to venue
    pub fee_to_venue: i64,
    /// Execution price in Q64
    pub exec_px_q64: u128,
}

/// Settlement mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettleMode {
    /// Atomic settlement (all-or-nothing)
    Atomic,
    /// Drain up to sequence number
    DrainUpToSeqno(u64),
}

/// Settlement batch result
#[derive(Debug, Clone)]
pub struct SettlementBatch {
    /// Starting sequence number
    pub seqno_start: u64,
    /// Ending sequence number
    pub seqno_end: u64,
    /// Fill deltas
    pub fills: Vec<FillDelta>,
}

// ============================================================================
// Pure Helper Functions
// ============================================================================

/// Add signed delta to LP shares with overflow checking
///
/// # Safety
/// - Returns Err on overflow/underflow
/// - Prevents negative share counts
pub const fn add_shares_checked(current: u128, delta: i128) -> Result<u128, ()> {
    if delta >= 0 {
        let d = delta as u128;
        match current.checked_add(d) {
            Some(result) => Ok(result),
            None => Err(()),
        }
    } else {
        let d = (-delta) as u128;
        if current >= d {
            Ok(current - d)
        } else {
            Err(())
        }
    }
}

/// Absolute value for i128
pub const fn abs_i128(x: i128) -> i128 {
    if x < 0 { -x } else { x }
}

/// Sum fill deltas for conservation check
///
/// Returns (sum_base, sum_quote, sum_fee_venue)
/// Conservation requires sum_base == 0 and sum_quote == 0
pub fn sum_fills(fills: &[FillDelta]) -> (i128, i128, i64) {
    let mut sb: i128 = 0;
    let mut sq: i128 = 0;
    let mut fv: i64 = 0;

    for f in fills {
        sb = sb.saturating_add(f.base_delta_q64);
        sq = sq.saturating_add(f.quote_delta_q64);
        fv = fv.saturating_add(f.fee_to_venue);
    }

    (sb, sq, fv)
}

/// Check slippage guard
///
/// Returns true if execution price is within guard bounds
pub fn check_slippage(
    exec_px_q64: u128,
    ref_mid_px_q64: u128,
    guard: &RiskGuard,
) -> bool {
    if ref_mid_px_q64 == 0 {
        return false;
    }

    let diff = if exec_px_q64 > ref_mid_px_q64 {
        exec_px_q64 - ref_mid_px_q64
    } else {
        ref_mid_px_q64 - exec_px_q64
    };

    let bps = diff.saturating_mul(10_000) / ref_mid_px_q64;
    bps <= guard.max_slippage_bps as u128
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_shares_checked_positive() {
        assert_eq!(add_shares_checked(100, 50), Ok(150));
        assert_eq!(add_shares_checked(0, 100), Ok(100));
    }

    #[test]
    fn test_add_shares_checked_negative() {
        assert_eq!(add_shares_checked(100, -50), Ok(50));
        assert_eq!(add_shares_checked(100, -100), Ok(0));
        assert!(add_shares_checked(50, -100).is_err());
    }

    #[test]
    fn test_add_shares_checked_overflow() {
        assert!(add_shares_checked(u128::MAX, 1).is_err());
    }

    #[test]
    fn test_sum_fills_conservation() {
        let fills = [
            FillDelta {
                taker_portfolio: [0; 32],
                maker_seat: SeatId([1; 32]),
                base_delta_q64: 100,
                quote_delta_q64: -200,
                fee_to_maker: 1,
                fee_to_venue: 2,
                exec_px_q64: 2 << 64,
            },
            FillDelta {
                taker_portfolio: [0; 32],
                maker_seat: SeatId([2; 32]),
                base_delta_q64: -100,
                quote_delta_q64: 200,
                fee_to_maker: -1,
                fee_to_venue: 0,
                exec_px_q64: 2 << 64,
            },
        ];

        let (sb, sq, _fv) = sum_fills(&fills);
        assert_eq!(sb, 0);
        assert_eq!(sq, 0);
    }

    #[test]
    fn test_check_slippage_within_bounds() {
        let guard = RiskGuard::conservative();

        // Exact match
        assert!(check_slippage(100 << 64, 100 << 64, &guard));

        // Within 0.5% = 50 bps
        let mid = 100 << 64;
        let exec_up = mid + (mid * 50) / 10_000;
        assert!(check_slippage(exec_up, mid, &guard));
    }

    #[test]
    fn test_check_slippage_exceeds_bounds() {
        let guard = RiskGuard::conservative();

        let mid = 100 << 64;
        let exec_way_up = mid + (mid * 100) / 10_000; // 1% slippage
        assert!(!check_slippage(exec_way_up, mid, &guard));
    }

    #[test]
    fn test_capability_check() {
        let caps = Capability::SupportsAMM as u64 | Capability::SupportsOrderBook as u64;

        assert!(Capability::is_enabled(caps, Capability::SupportsAMM));
        assert!(Capability::is_enabled(caps, Capability::SupportsOrderBook));
        assert!(!Capability::is_enabled(caps, Capability::SupportsHybrid));
    }
}
