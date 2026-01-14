//! Passive LP Matcher for Percolator
//!
//! Quotes ±50 bps off oracle price. Returns TradeExecution for Percolator to apply.
//! No CPI, no floats, deterministic integer math only.

/// Reason codes for trade rejection or success
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Reason {
    Ok = 0,
    ZeroQty = 1,
    OracleZero = 2,
    NotCrossed = 3,
    TakerLimitTooTight = 4,
    LpMaxSize = 5,
    LpInventoryLimit = 6,
    MathOverflow = 7,
}

/// Trade execution result - matches Percolator's TradeExecution ABI
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct TradeExecution {
    pub price: u64,
    pub size: i128,
}

/// Extended result with reason code for diagnostics
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MatchResult {
    pub exec: TradeExecution,
    pub reason: Reason,
    /// Quote delta for LP: positive = LP receives quote, negative = LP pays quote
    pub quote_delta_lp: i128,
}

impl MatchResult {
    #[inline]
    pub const fn unfilled(reason: Reason) -> Self {
        Self {
            exec: TradeExecution { price: 0, size: 0 },
            reason,
            quote_delta_lp: 0,
        }
    }

    #[inline]
    pub const fn filled(price: u64, size: i128, quote_delta_lp: i128) -> Self {
        Self {
            exec: TradeExecution { price, size },
            reason: Reason::Ok,
            quote_delta_lp,
        }
    }
}

/// Configuration for passive LP matcher
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PassiveMatcherConfig {
    /// Edge in basis points (default 50 = 0.50%)
    pub edge_bps: u16,
    /// Minimum base quantity to fill
    pub min_base_qty: u64,
    /// Maximum base quantity per fill
    pub max_base_qty: u64,
    /// Maximum absolute inventory LP can hold
    pub max_abs_inventory: i128,
}

impl Default for PassiveMatcherConfig {
    fn default() -> Self {
        Self {
            edge_bps: 50,
            min_base_qty: 1,
            max_base_qty: u64::MAX,
            max_abs_inventory: i128::MAX,
        }
    }
}

/// LP state tracking inventory
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct PassiveLpState {
    /// Current inventory in base units (positive = long, negative = short)
    pub inventory_base: i128,
}

/// Computed bid/ask quotes
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Quote {
    pub bid: u64,
    pub ask: u64,
}

/// Ceiling division for u128: ceil(n / d)
#[inline]
const fn ceil_div_u128(n: u128, d: u128) -> u128 {
    if d == 0 {
        return 0; // Caller must check oracle != 0 before calling
    }
    (n + d - 1) / d
}

/// Compute bid/ask quotes from oracle price
///
/// - bid = floor(oracle * (10000 - edge_bps) / 10000) - rounds down (passive)
/// - ask = ceil(oracle * (10000 + edge_bps) / 10000) - rounds up (passive)
///
/// Returns None if oracle_price is 0.
pub fn compute_quote(cfg: &PassiveMatcherConfig, oracle_price: u64) -> Option<Quote> {
    if oracle_price == 0 {
        return None;
    }

    let oracle = oracle_price as u128;
    let edge = cfg.edge_bps as u128;
    const BPS_DENOM: u128 = 10_000;

    // bid = floor(oracle * (10000 - edge) / 10000)
    let bid_numer = oracle.checked_mul(BPS_DENOM.saturating_sub(edge))?;
    let bid = bid_numer / BPS_DENOM;

    // ask = ceil(oracle * (10000 + edge) / 10000)
    let ask_numer = oracle.checked_mul(BPS_DENOM.checked_add(edge)?)?;
    let ask = ceil_div_u128(ask_numer, BPS_DENOM);

    // Convert back to u64, should never overflow for reasonable prices
    let bid_u64 = if bid > u64::MAX as u128 {
        return None;
    } else {
        bid as u64
    };

    let ask_u64 = if ask > u64::MAX as u128 {
        return None;
    } else {
        ask as u64
    };

    Some(Quote {
        bid: bid_u64,
        ask: ask_u64,
    })
}

/// Passive oracle-based matching engine
#[derive(Clone, Copy, Debug, Default)]
pub struct PassiveOracleBpsMatcher;

impl PassiveOracleBpsMatcher {
    /// Execute a match request
    ///
    /// # Arguments
    /// * `cfg` - Matcher configuration
    /// * `lp` - Mutable LP state for inventory tracking
    /// * `oracle_price` - Current oracle price (1e6 scaled)
    /// * `req_size` - Requested size: positive = user buys (LP sells), negative = user sells (LP buys)
    /// * `limit_price` - Optional limit price (1e6 scaled)
    ///
    /// # Returns
    /// MatchResult with execution details or rejection reason
    pub fn execute_match(
        &self,
        cfg: &PassiveMatcherConfig,
        lp: &mut PassiveLpState,
        oracle_price: u64,
        req_size: i128,
        limit_price: Option<u64>,
    ) -> MatchResult {
        // Zero quantity check
        if req_size == 0 {
            return MatchResult::unfilled(Reason::ZeroQty);
        }

        let abs_req_size = req_size.unsigned_abs();

        // Min quantity check
        if abs_req_size < cfg.min_base_qty as u128 {
            return MatchResult::unfilled(Reason::ZeroQty);
        }

        // Compute quotes
        let quote = match compute_quote(cfg, oracle_price) {
            Some(q) => q,
            None => return MatchResult::unfilled(Reason::OracleZero),
        };

        // Determine execution price and check limit
        // req_size > 0: user buys at ask, LP sells
        // req_size < 0: user sells at bid, LP buys
        let is_user_buy = req_size > 0;
        let exec_price = if is_user_buy { quote.ask } else { quote.bid };

        // Check taker limit price
        if let Some(limit) = limit_price {
            if is_user_buy {
                // User buying: limit must be >= ask (willing to pay at least ask)
                if limit < exec_price {
                    return MatchResult::unfilled(Reason::TakerLimitTooTight);
                }
            } else {
                // User selling: limit must be <= bid (willing to receive at most bid)
                if limit > exec_price {
                    return MatchResult::unfilled(Reason::TakerLimitTooTight);
                }
            }
        }

        // Apply size cap
        let capped_abs_size = if abs_req_size > cfg.max_base_qty as u128 {
            cfg.max_base_qty as u128
        } else {
            abs_req_size
        };

        if capped_abs_size == 0 {
            return MatchResult::unfilled(Reason::LpMaxSize);
        }

        // Convert to i128 for inventory math
        let fill_size_abs = capped_abs_size as i128;
        let fill_size = if is_user_buy {
            fill_size_abs
        } else {
            -fill_size_abs
        };

        // LP inventory change is opposite of user's trade
        // User buys (positive) => LP sells => LP inventory decreases (more short)
        // User sells (negative) => LP buys => LP inventory increases (more long)
        let lp_inventory_delta = -fill_size;

        // Check inventory limit
        let new_inventory = match lp.inventory_base.checked_add(lp_inventory_delta) {
            Some(inv) => inv,
            None => return MatchResult::unfilled(Reason::MathOverflow),
        };

        if new_inventory.unsigned_abs() > cfg.max_abs_inventory.unsigned_abs() {
            return MatchResult::unfilled(Reason::LpInventoryLimit);
        }

        // Compute quote amount
        // quote_amount = fill_size_abs * exec_price
        let quote_amount_u128 = (capped_abs_size).checked_mul(exec_price as u128);
        let quote_amount_u128 = match quote_amount_u128 {
            Some(q) => q,
            None => return MatchResult::unfilled(Reason::MathOverflow),
        };

        // Convert to i128
        if quote_amount_u128 > i128::MAX as u128 {
            return MatchResult::unfilled(Reason::MathOverflow);
        }
        let quote_amount = quote_amount_u128 as i128;

        // Quote delta for LP:
        // User buys (LP sells base) => LP receives quote => positive
        // User sells (LP buys base) => LP pays quote => negative
        let quote_delta_lp = if is_user_buy {
            quote_amount
        } else {
            -quote_amount
        };

        // Update LP state
        lp.inventory_base = new_inventory;

        MatchResult::filled(exec_price, fill_size, quote_delta_lp)
    }
}

/// Trait for matching engines (compatible with Percolator's expected interface)
pub trait MatchingEngine {
    fn execute_match(
        &self,
        cfg: &PassiveMatcherConfig,
        lp: &mut PassiveLpState,
        oracle_price: u64,
        req_size: i128,
        limit_price: Option<u64>,
    ) -> MatchResult;
}

impl MatchingEngine for PassiveOracleBpsMatcher {
    fn execute_match(
        &self,
        cfg: &PassiveMatcherConfig,
        lp: &mut PassiveLpState,
        oracle_price: u64,
        req_size: i128,
        limit_price: Option<u64>,
    ) -> MatchResult {
        PassiveOracleBpsMatcher::execute_match(self, cfg, lp, oracle_price, req_size, limit_price)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_cfg() -> PassiveMatcherConfig {
        PassiveMatcherConfig::default()
    }

    fn default_lp() -> PassiveLpState {
        PassiveLpState::default()
    }

    #[test]
    fn test_quote_math_basic() {
        // oracle=100_000, edge=50 bps
        // bid = floor(100_000 * 9950 / 10000) = floor(995_000_000 / 10000) = 99_500
        // ask = ceil(100_000 * 10050 / 10000) = ceil(1_005_000_000 / 10000) = 100_500
        let cfg = default_cfg();
        let quote = compute_quote(&cfg, 100_000).unwrap();
        assert_eq!(quote.bid, 99_500);
        assert_eq!(quote.ask, 100_500);
    }

    #[test]
    fn test_ask_rounds_up() {
        // Choose oracle where (oracle * 10050) doesn't divide evenly by 10000
        // oracle = 100_001
        // ask_numer = 100_001 * 10050 = 1_005_010_050
        // ask = ceil(1_005_010_050 / 10000) = ceil(100501.005) = 100_502
        let cfg = default_cfg();
        let quote = compute_quote(&cfg, 100_001).unwrap();
        // 100_001 * 10050 = 1_005_010_050
        // 1_005_010_050 / 10000 = 100_501.005, ceil = 100_502
        assert_eq!(quote.ask, 100_502);
    }

    #[test]
    fn test_bid_rounds_down() {
        // oracle = 100_001
        // bid_numer = 100_001 * 9950 = 995_009_950
        // bid = floor(995_009_950 / 10000) = 99_500.995 => 99_500
        let cfg = default_cfg();
        let quote = compute_quote(&cfg, 100_001).unwrap();
        assert_eq!(quote.bid, 99_500);
    }

    #[test]
    fn test_oracle_zero_returns_none() {
        let cfg = default_cfg();
        assert!(compute_quote(&cfg, 0).is_none());
    }

    #[test]
    fn test_buy_with_limit_too_low_fails() {
        // oracle=100_000 => ask=100_500
        // User buying with limit=100_400 should fail
        let cfg = default_cfg();
        let mut lp = default_lp();
        let matcher = PassiveOracleBpsMatcher;

        let result = matcher.execute_match(&cfg, &mut lp, 100_000, 10, Some(100_400));
        assert_eq!(result.reason, Reason::TakerLimitTooTight);
        assert_eq!(result.exec.size, 0);
    }

    #[test]
    fn test_sell_with_limit_too_high_fails() {
        // oracle=100_000 => bid=99_500
        // User selling with limit=99_600 should fail (wants more than bid)
        let cfg = default_cfg();
        let mut lp = default_lp();
        let matcher = PassiveOracleBpsMatcher;

        let result = matcher.execute_match(&cfg, &mut lp, 100_000, -10, Some(99_600));
        assert_eq!(result.reason, Reason::TakerLimitTooTight);
        assert_eq!(result.exec.size, 0);
    }

    #[test]
    fn test_inventory_limit_buy() {
        // max_abs_inventory=10, inventory=-10 (LP is short 10)
        // User buys 1 (LP sells 1 more) => new inventory = -11 => rejected
        let cfg = PassiveMatcherConfig {
            max_abs_inventory: 10,
            ..default_cfg()
        };
        let mut lp = PassiveLpState { inventory_base: -10 };
        let matcher = PassiveOracleBpsMatcher;

        let result = matcher.execute_match(&cfg, &mut lp, 100_000, 1, None);
        assert_eq!(result.reason, Reason::LpInventoryLimit);
        assert_eq!(lp.inventory_base, -10); // unchanged
    }

    #[test]
    fn test_inventory_limit_sell() {
        // max_abs_inventory=10, inventory=10 (LP is long 10)
        // User sells 1 (LP buys 1 more) => new inventory = 11 => rejected
        let cfg = PassiveMatcherConfig {
            max_abs_inventory: 10,
            ..default_cfg()
        };
        let mut lp = PassiveLpState { inventory_base: 10 };
        let matcher = PassiveOracleBpsMatcher;

        let result = matcher.execute_match(&cfg, &mut lp, 100_000, -1, None);
        assert_eq!(result.reason, Reason::LpInventoryLimit);
        assert_eq!(lp.inventory_base, 10); // unchanged
    }

    #[test]
    fn test_buy_deltas_sign_correctness() {
        // User buys: quote_to_lp > 0 (LP receives quote), exec.size > 0
        let cfg = default_cfg();
        let mut lp = default_lp();
        let matcher = PassiveOracleBpsMatcher;

        let result = matcher.execute_match(&cfg, &mut lp, 100_000, 10, None);
        assert_eq!(result.reason, Reason::Ok);
        assert!(result.exec.size > 0, "exec.size should be positive for buy");
        assert!(
            result.quote_delta_lp > 0,
            "quote_delta_lp should be positive (LP receives quote)"
        );
        assert_eq!(result.exec.price, 100_500); // ask price
        assert_eq!(result.exec.size, 10);
        // quote_delta_lp = 10 * 100_500 = 1_005_000
        assert_eq!(result.quote_delta_lp, 1_005_000);
        // LP inventory decreases (sells base)
        assert_eq!(lp.inventory_base, -10);
    }

    #[test]
    fn test_sell_deltas_sign_correctness() {
        // User sells: quote_to_lp < 0 (LP pays quote), exec.size < 0
        let cfg = default_cfg();
        let mut lp = default_lp();
        let matcher = PassiveOracleBpsMatcher;

        let result = matcher.execute_match(&cfg, &mut lp, 100_000, -10, None);
        assert_eq!(result.reason, Reason::Ok);
        assert!(result.exec.size < 0, "exec.size should be negative for sell");
        assert!(
            result.quote_delta_lp < 0,
            "quote_delta_lp should be negative (LP pays quote)"
        );
        assert_eq!(result.exec.price, 99_500); // bid price
        assert_eq!(result.exec.size, -10);
        // quote_delta_lp = -10 * 99_500 = -995_000
        assert_eq!(result.quote_delta_lp, -995_000);
        // LP inventory increases (buys base)
        assert_eq!(lp.inventory_base, 10);
    }

    #[test]
    fn test_zero_qty_rejected() {
        let cfg = default_cfg();
        let mut lp = default_lp();
        let matcher = PassiveOracleBpsMatcher;

        let result = matcher.execute_match(&cfg, &mut lp, 100_000, 0, None);
        assert_eq!(result.reason, Reason::ZeroQty);
    }

    #[test]
    fn test_below_min_qty_rejected() {
        let cfg = PassiveMatcherConfig {
            min_base_qty: 10,
            ..default_cfg()
        };
        let mut lp = default_lp();
        let matcher = PassiveOracleBpsMatcher;

        let result = matcher.execute_match(&cfg, &mut lp, 100_000, 5, None);
        assert_eq!(result.reason, Reason::ZeroQty);
    }

    #[test]
    fn test_max_size_cap() {
        let cfg = PassiveMatcherConfig {
            max_base_qty: 5,
            ..default_cfg()
        };
        let mut lp = default_lp();
        let matcher = PassiveOracleBpsMatcher;

        // Request 100, should fill only 5
        let result = matcher.execute_match(&cfg, &mut lp, 100_000, 100, None);
        assert_eq!(result.reason, Reason::Ok);
        assert_eq!(result.exec.size, 5);
    }

    #[test]
    fn test_buy_with_exact_limit_succeeds() {
        // oracle=100_000 => ask=100_500
        // User buying with limit=100_500 exactly should succeed
        let cfg = default_cfg();
        let mut lp = default_lp();
        let matcher = PassiveOracleBpsMatcher;

        let result = matcher.execute_match(&cfg, &mut lp, 100_000, 10, Some(100_500));
        assert_eq!(result.reason, Reason::Ok);
        assert_eq!(result.exec.size, 10);
    }

    #[test]
    fn test_sell_with_exact_limit_succeeds() {
        // oracle=100_000 => bid=99_500
        // User selling with limit=99_500 exactly should succeed
        let cfg = default_cfg();
        let mut lp = default_lp();
        let matcher = PassiveOracleBpsMatcher;

        let result = matcher.execute_match(&cfg, &mut lp, 100_000, -10, Some(99_500));
        assert_eq!(result.reason, Reason::Ok);
        assert_eq!(result.exec.size, -10);
    }

    #[test]
    fn test_inventory_updates_correctly() {
        let cfg = default_cfg();
        let mut lp = PassiveLpState { inventory_base: 5 }; // Start long 5
        let matcher = PassiveOracleBpsMatcher;

        // User buys 3 => LP sells 3 => inventory = 5 - 3 = 2
        let result = matcher.execute_match(&cfg, &mut lp, 100_000, 3, None);
        assert_eq!(result.reason, Reason::Ok);
        assert_eq!(lp.inventory_base, 2);

        // User sells 10 => LP buys 10 => inventory = 2 + 10 = 12
        let result = matcher.execute_match(&cfg, &mut lp, 100_000, -10, None);
        assert_eq!(result.reason, Reason::Ok);
        assert_eq!(lp.inventory_base, 12);
    }

    #[test]
    fn test_large_price_no_overflow() {
        let cfg = default_cfg();
        let mut lp = default_lp();
        let matcher = PassiveOracleBpsMatcher;

        // Use a large but reasonable price (1e12 = $1M with 1e6 scaling)
        let large_price = 1_000_000_000_000u64;
        let result = matcher.execute_match(&cfg, &mut lp, large_price, 1000, None);
        assert_eq!(result.reason, Reason::Ok);
        assert!(result.exec.size > 0);
    }
}
