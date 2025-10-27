//! Pure state model for Kani verification

/// Price oracle snapshot for liquidation checks
/// Prices are in fixed-point notation (e.g., 1e6 = $1.00)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Prices {
    /// Prices for up to 4 assets (bounded for Kani tractability)
    /// Index 0 = collateral price, 1-3 = asset prices
    pub p: [u64; 4],
}

impl Default for Prices {
    fn default() -> Self {
        Self {
            p: [1_000_000, 1_000_000, 1_000_000, 1_000_000], // $1.00 each
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Warmup {
    pub started_at_slot: u64,
    pub slope_per_step: u128, // Linear cap per step for Kani model
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Account {
    pub principal: u128,      // Never reduced by socialize/loss (I1)
    pub pnl_ledger: i128,     // Can be positive or negative
    pub reserved_pnl: u128,   // Pending withdrawals
    pub warmup_state: Warmup,
    pub position_size: u128,  // Notional position size (for liquidation calc)

    // Fee distribution fields (index-based, scan-free)
    pub fee_index_user: u128,      // Snapshot of global fee_index at last touch
    pub fee_accrued: u128,         // Accrued fees not yet transferred
    pub vested_pos_snapshot: u128, // Cached contribution to sum_vested_pos_pnl
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Params {
    pub max_users: u8,
    pub withdraw_cap_per_step: u128,
    /// Maintenance margin ratio (e.g., 5% = 50_000 in basis points 1e6)
    pub maintenance_margin_bps: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    pub vault: u128,
    pub fees_outstanding: u128, // Fees to be distributed to position holders
    pub users: arrayvec::ArrayVec<Account, 6>, // Small fixed bound for Kani
    pub params: Params,
    pub authorized_router: bool, // For I3: authorization checks

    // Fee distribution global state (index-based, scan-free)
    pub loss_accum: u128,           // Lazy socialized losses (L ≥ 0)
    pub fee_index: u128,            // Fees per unit vested positive PnL (scaled by 1e6)
    pub sum_vested_pos_pnl: u128,   // Sum of all accounts' positive vested PnL (lazy)
    pub fee_carry: u128,            // Rounding dust and carried fees
}

impl Default for Warmup {
    fn default() -> Self {
        Self {
            started_at_slot: 0,
            slope_per_step: 1_000_000,
        }
    }
}

impl Default for Account {
    fn default() -> Self {
        Self {
            principal: 0,
            pnl_ledger: 0,
            reserved_pnl: 0,
            warmup_state: Warmup::default(),
            position_size: 0,
            fee_index_user: 0,
            fee_accrued: 0,
            vested_pos_snapshot: 0,
        }
    }
}

impl Default for Params {
    fn default() -> Self {
        Self {
            max_users: 6,
            withdraw_cap_per_step: 1_000_000,
            maintenance_margin_bps: 50_000, // 5% maintenance margin
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            vault: 0,
            fees_outstanding: 0,
            users: arrayvec::ArrayVec::new(),
            params: Params::default(),
            authorized_router: true,
            loss_accum: 0,
            fee_index: 0,
            sum_vested_pos_pnl: 0,
            fee_carry: 0,
        }
    }
}
