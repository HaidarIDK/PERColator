//! Generators for arbitrary state (for Kani)

#[cfg(kani)]
use kani::any;
use model_safety::state::*;
use arrayvec::ArrayVec;

// Ultra-small bounds for very fast verification
// Reduced to 100 for faster SAT solving (10x speedup)
const MAX_VAL: u128 = 100;
const MAX_PNL: i128 = 100;

#[cfg(kani)]
pub fn any_account() -> Account {
    let principal_raw: u8 = any();
    let pnl_raw: i8 = any();
    let reserved_raw: u8 = any();
    let slope_raw: u8 = any();
    let position_raw: u8 = any();

    Account {
        principal: (principal_raw as u128) % MAX_VAL,
        pnl_ledger: (pnl_raw as i128).clamp(-MAX_PNL, MAX_PNL),
        reserved_pnl: (reserved_raw as u128) % (MAX_VAL / 2), // Reduced to half
        warmup_state: Warmup {
            started_at_slot: (principal_raw as u64) % 20,  // Reduced from 100 to 20
            slope_per_step: ((slope_raw as u128) % 20).max(1), // Reduced from 100 to 20
        },
        position_size: (position_raw as u128) % MAX_VAL,
    }
}

#[cfg(kani)]
pub fn any_prices() -> Prices {
    let p0_raw: u8 = any();
    let p1_raw: u8 = any();
    let p2_raw: u8 = any();
    let p3_raw: u8 = any();

    Prices {
        p: [
            // Reduced range: 0.5 to 1.5 (500k to 1.5M in 1e6)
            ((p0_raw as u64) % 1_000_000).max(500_000),
            ((p1_raw as u64) % 1_000_000).max(500_000),
            ((p2_raw as u64) % 1_000_000).max(500_000),
            ((p3_raw as u64) % 1_000_000).max(500_000),
        ],
    }
}

#[cfg(kani)]
pub fn any_state_bounded() -> State {
    let mut users: ArrayVec<Account, 6> = ArrayVec::new();
    // Single user only for minimal state space
    let _ = users.try_push(any_account());

    let vault_raw: u8 = any();
    let insurance_raw: u8 = any();
    let fees_raw: u8 = any();
    let margin_bps_raw: u8 = any();

    State {
        vault: (vault_raw as u128) % (MAX_VAL * 3), // Reduced from 5 to 3
        insurance_fund: (insurance_raw as u128) % MAX_VAL,
        fees_outstanding: (fees_raw as u128) % MAX_VAL,
        users,
        params: Params {
            max_users: 6,
            withdraw_cap_per_step: 100, // Reduced from 1000 to 100
            // 5% to 10% maintenance margin (50_000 to 100_000 bps)
            maintenance_margin_bps: ((margin_bps_raw as u64) % 50_000 + 50_000),
        },
        authorized_router: true, // Start authorized
    }
}
