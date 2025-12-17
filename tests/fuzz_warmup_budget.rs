// Deterministic fuzz test for warmup budget invariant
// Uses xorshift64 PRNG for reproducibility

use percolator::*;

// xorshift64 PRNG for deterministic randomness
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed)
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn u64(&mut self, lo: u64, hi: u64) -> u64 {
        if lo >= hi {
            return lo;
        }
        lo + (self.next() % (hi - lo + 1))
    }

    fn u128(&mut self, lo: u128, hi: u128) -> u128 {
        if lo >= hi {
            return lo;
        }
        lo + ((self.next() as u128) % (hi - lo + 1))
    }

    fn i128(&mut self, lo: i128, hi: i128) -> i128 {
        if lo >= hi {
            return lo;
        }
        lo + ((self.next() as i128).abs() % (hi - lo + 1))
    }

    fn usize(&mut self, lo: usize, hi: usize) -> usize {
        if lo >= hi {
            return lo;
        }
        lo + ((self.next() as usize) % (hi - lo + 1))
    }
}

fn default_params() -> RiskParams {
    RiskParams {
        warmup_period_slots: 100,
        maintenance_margin_bps: 500,
        initial_margin_bps: 1000,
        trading_fee_bps: 10,
        max_accounts: 1000,
        account_fee_bps: 10000,
        risk_reduction_threshold: 1000, // Floor at 1000
    }
}

// No-op matcher for trades
struct NoOpMatcher;

impl MatchingEngine for NoOpMatcher {
    fn execute_match(
        &self,
        _program: &[u8; 32],
        _context: &[u8; 32],
        _lp_account_id: u64,
        oracle_price: u64,
        size: i128,
    ) -> Result<TradeExecution> {
        Ok(TradeExecution {
            price: oracle_price,
            size,
        })
    }
}

#[test]
fn fuzz_warmup_budget_invariant() {
    // For 500 seeds
    // Tests that the warmup budget invariant is enforced at warmup time.
    // Note: After insurance is spent by ADL, the global invariant may not hold,
    // but this is expected because the budget was "spent" on both warmup and ADL.
    // The key property is that settle_warmup_to_capital never violates the budget
    // at the time of warmup.

    for seed in 1..=500 {
        let mut rng = Rng::new(seed);
        let mut engine = Box::new(RiskEngine::new(default_params()));

        // Initialize with 6-8 accounts
        let num_accounts = rng.usize(6, 8);
        let mut account_indices = Vec::new();

        // Create LP with initial capital from deposit
        let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 1).unwrap();
        account_indices.push(lp_idx);
        let _ = engine.deposit(lp_idx, rng.u128(10_000, 100_000));

        // Create users with initial capital from deposit
        for _ in 0..(num_accounts - 1) {
            let user_idx = engine.add_user(1).unwrap();
            account_indices.push(user_idx);
            let _ = engine.deposit(user_idx, rng.u128(1_000, 50_000));
        }

        // Set insurance well above floor to provide ample budget for warmup
        let floor = engine.params.risk_reduction_threshold;
        let old_insurance = engine.insurance_fund.balance;
        let new_insurance = floor + rng.u128(10_000, 100_000);
        // Only add the increase to vault to maintain conservation
        if new_insurance > old_insurance {
            engine.vault += new_insurance - old_insurance;
        }
        engine.insurance_fund.balance = new_insurance;

        // For 200 steps
        for step in 0..200 {
            let action = rng.usize(0, 5);

            match action {
                0 => {
                    // advance_slot(0..5)
                    let slots = rng.u64(0, 5);
                    engine.advance_slot(slots);
                }
                1 => {
                    // settle_warmup_to_capital(random idx)
                    let idx = account_indices[rng.usize(0, account_indices.len() - 1)];
                    let _ = engine.settle_warmup_to_capital(idx);
                }
                2 => {
                    // deposit(random idx, amount)
                    let idx = account_indices[rng.usize(0, account_indices.len() - 1)];
                    let amount = rng.u128(0, 10_000);
                    let _ = engine.deposit(idx, amount);
                }
                3 => {
                    // withdraw(random idx, amount) - ignore errors
                    let idx = account_indices[rng.usize(0, account_indices.len() - 1)];
                    let amount = rng.u128(0, 10_000);
                    let _ = engine.withdraw(idx, amount);
                }
                4 => {
                    // execute_trade with NoOpMatcher using small sizes - ignore errors
                    if account_indices.len() >= 2 {
                        // Pick two different accounts (LP and user)
                        let lp = lp_idx;
                        let user_pos = rng.usize(1, account_indices.len() - 1);
                        let user = account_indices[user_pos];

                        let size = rng.i128(-1000, 1000);
                        let oracle_price = rng.u64(1_000_000, 10_000_000);

                        let _ = engine.execute_trade(&NoOpMatcher, lp, user, oracle_price, size);
                    }
                }
                5 => {
                    // panic_settle_all(random oracle price) - ignore errors
                    // This can trigger ADL which spends unreserved insurance
                    let oracle_price = rng.u64(1_000_000, 10_000_000);
                    let _ = engine.panic_settle_all(oracle_price);
                }
                _ => {}
            }

            // After each step, check stable warmup budget invariant
            // W+ ≤ W- + raw_spendable (reserved insurance backs warmed profits)
            let raw = engine.insurance_spendable_raw();
            assert!(
                engine.warmed_pos_total <= engine.warmed_neg_total.saturating_add(raw),
                "Seed {}, Step {}: Warmup budget invariant violated!\n\
                 W+={}, W-={}, raw={}, reserved={}\n\
                 W+ <= W- + raw should hold",
                seed, step,
                engine.warmed_pos_total, engine.warmed_neg_total, raw,
                engine.warmup_insurance_reserved
            );

            // Also verify reserved never exceeds raw spendable
            assert!(
                engine.warmup_insurance_reserved <= raw,
                "Seed {}, Step {}: Reserved exceeds raw spendable!\n\
                 reserved={}, raw={}",
                seed, step,
                engine.warmup_insurance_reserved, raw
            );

            // Always check conservation
            assert!(engine.check_conservation(),
                    "Seed {}, Step {}: Conservation violated", seed, step);
        }
    }
}
