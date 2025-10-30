//! State space sanitizer - bounds values for Kani exploration

use model_safety::state::*;

pub const N_USERS: usize = 3;
pub const MAX_STEPS: u8 = 4;

/// Bounds for tractable verification
const MAX_PRINCIPAL: u128 = 1_000_000u128;
const MAX_PNL_ABS: i128 = 1_000_000i128;
const MAX_RESERVED: u128 = 1_000_000u128;
const MAX_SLOPE: u128 = 10_000u128;
const MAX_VAULT: u128 = 10_000_000u128;
const MAX_INSURANCE: u128 = 1_000_000u128;
const MAX_FEES: u128 = 1_000_000u128;
const MAX_POSITION: u128 = 2_000_000u128;
const MAX_PRICE: u64 = 5_000_000u64; // Max $5.00 per unit (in 1e6 scale)
const MIN_PRICE: u64 = 100_000u64;    // Min $0.10 per unit

pub trait Sanitize {
    fn sanitize(self) -> Self;
}

impl Sanitize for State {
    fn sanitize(mut self) -> State {
        // Clamp user count
        while self.users.len() > N_USERS {
            self.users.pop();
        }

        // Clamp user values to stress edges
        for u in self.users.iter_mut() {
            // Keep principal reasonable but stress u128::MAX edges
            u.principal = if u.principal > MAX_PRINCIPAL {
                u.principal % MAX_PRINCIPAL
            } else {
                u.principal
            };

            // Clamp PnL to reasonable range
            u.pnl_ledger = u.pnl_ledger.clamp(-MAX_PNL_ABS, MAX_PNL_ABS);

            // Clamp reserved PnL
            u.reserved_pnl = if u.reserved_pnl > MAX_RESERVED {
                u.reserved_pnl % MAX_RESERVED
            } else {
                u.reserved_pnl
            };

            // Ensure non-zero slope to avoid division issues
            u.warmup_state.slope_per_step = if u.warmup_state.slope_per_step == 0 {
                1
            } else if u.warmup_state.slope_per_step > MAX_SLOPE {
                (u.warmup_state.slope_per_step % MAX_SLOPE) + 1
            } else {
                u.warmup_state.slope_per_step
            };

            // Clamp position size
            u.position_size = if u.position_size > MAX_POSITION {
                u.position_size % MAX_POSITION
            } else {
                u.position_size
            };
        }

        // Clamp vault, insurance, fees
        self.vault = if self.vault > MAX_VAULT {
            self.vault % MAX_VAULT
        } else {
            self.vault
        };

        self.insurance_fund = if self.insurance_fund > MAX_INSURANCE {
            self.insurance_fund % MAX_INSURANCE
        } else {
            self.insurance_fund
        };

        self.fees_outstanding = if self.fees_outstanding > MAX_FEES {
            self.fees_outstanding % MAX_FEES
        } else {
            self.fees_outstanding
        };

        self
    }
}

impl Sanitize for Prices {
    fn sanitize(mut self) -> Prices {
        // Clamp all prices to reasonable bounds and ensure non-zero
        for i in 0..self.p.len() {
            self.p[i] = if self.p[i] > MAX_PRICE {
                (self.p[i] % (MAX_PRICE - MIN_PRICE)) + MIN_PRICE
            } else if self.p[i] < MIN_PRICE {
                MIN_PRICE
            } else {
                self.p[i]
            };
        }
        self
    }
}

/// Convenience function for sanitizing state in proofs
pub fn sanitize_state(s: State) -> State {
    s.sanitize()
}

/// Convenience function for sanitizing prices in proofs
pub fn sanitize_prices(p: Prices) -> Prices {
    p.sanitize()
}
