# Percolator: Risk Engine for Perpetual DEXs

⚠️ **EDUCATIONAL RESEARCH PROJECT — NOT PRODUCTION READY** ⚠️  
Do **NOT** use with real funds. Not audited. Experimental design.

Percolator is a **formally verified risk engine** for perpetual futures DEXs on Solana.  
It provides *provable guarantees* about fund safety, loss accounting, and margin correctness under adversarial conditions, including oracle manipulation.

---

## Design

### Core Insight

Oracle manipulation allows attackers to create *temporary artificial profits*.  
Percolator prevents these profits from being extracted by enforcing **PNL warmup**:

- Profits must **vest over time `T`** before becoming withdrawable capital.
- Losses are **realized immediately** into capital.
- During ADL (Auto-Deleveraging), **unwrapped (young) PNL is haircutted first**, protecting deposited capital.

**Guarantee (Qualified):**

> If oracle manipulation lasts at most time `T`, and positive PNL requires time `T` to vest, then an account’s deposited capital is withdrawable **up to `max(0, capital + pnl)`**, subject to margin requirements and system solvency rules.

This prevents attackers from minting profits via oracle manipulation and withdrawing them before corresponding losses are realized.

---

## Proven Invariants

| ID | Property |
|----|----------|
| **I1** | Account capital is **never** reduced by ADL or PNL socialization |
| **I2** | Conservation (one-sided): `vault + loss_accum ≥ sum(capital) + sum(pnl) + insurance` |
| **I4** | ADL haircuts **unwrapped (young) PNL first**, before insurance |
| **I5** | PNL warmup is deterministic and monotonically increasing |
| **I7** | Account isolation — operations on one account cannot affect others |
| **I8** | **Equity-based margin**: `equity = max(0, capital + pnl)` is used consistently |
| **N1** | Negative PNL is realized **immediately** into capital (not time-gated) |

Additional audit-mandated properties verify:
- Insurance floor protection
- Reserved insurance safety
- Idempotent warmup settlement
- Bounded rounding slack
- Correct behavior in withdrawal-only mode

---

## Warmup Budget System

Warmup converts PNL into capital with **sign-sensitive semantics**:

- **Positive PNL** → may increase capital (profits vest gradually)
- **Negative PNL** → immediately decreases capital (losses paid first)

Two invariants govern this system.

### Stable Invariant (Always Holds)

W⁺ ≤ W⁻ + max(0, I − I_min)

### Budget Constraint (Limits Future Warmups)

Budget = W⁻ + S − W⁺ ≥ 0

Where:

- `W⁺` = `warmed_pos_total` — cumulative positive PNL converted to capital  
- `W⁻` = `warmed_neg_total` — cumulative negative PNL paid from capital  
- `I` = `insurance_fund.balance`  
- `I_min` = `risk_reduction_threshold` (protected insurance floor)  
- `raw_spendable = max(0, I − I_min)`  
- `R = min(max(W⁺ − W⁻, 0), raw_spendable)` — **reserved insurance**  
- `S = raw_spendable − R` — **unreserved spendable insurance**

**Key properties:**

- Reserved insurance `R` backs already-warmed profits.
- ADL may only spend **unreserved insurance `S`**.
- When losses are realized (`W⁻` increases), reserved insurance is automatically released.

**Enforcement:**  
Losses are settled **before** profits are warmed. Warmup settlement clamps profits by the remaining budget, preventing profit maturation from outrunning loss realization.

---

## Key Operations

### Trading
- Zero-sum PNL between user and LP
- Trading fees are transferred to the insurance fund
- Funding is settled lazily using a cumulative index

### ADL (Auto-Deleveraging)
When losses must be covered:
1. Proportionally haircut **unwrapped (young) PNL**
2. Spend only **unreserved insurance above the floor**
3. Accumulate any remainder in `loss_accum`

**Exactness Guarantee:**  
The sum of all ADL haircuts equals `loss_to_socialize` exactly.  
Remainders from integer division are distributed deterministically using a largest-remainder rule (ties broken by account index).

### Risk-Reduction-Only Mode
Triggered when insurance is at/below its configured threshold or uncovered losses exist:

- Warmup is frozen (no profits vest)
- Risk-increasing trades are blocked
- Position reduction/closure is allowed
- Capital withdrawals are allowed (subject to margin and solvency)
- Exit occurs via insurance fund top-up

### Forced Loss Realization (Threshold Unstick)
When `insurance_fund.balance ≤ I_min`, the engine may run an atomic scan that:

- Settles all open positions at the oracle price
- Forces negative PNL to be paid from capital (up to available capital)
- Socializes unpaid losses via ADL
- Prevents profit maturation or withdrawal from outrunning realized losses

### Panic Settlement
An emergency instruction that:

- Enters risk-reduction-only mode
- Settles all positions at the oracle price
- Clamps negative PNL to zero
- Socializes losses via ADL
- Converts remaining positive PNL via warmup under budget constraints

---

## Conservation

Conservation is enforced **one-sided with bounded slack**:

vault + loss_accum ≥ sum(capital) + sum(pnl) + insurance_fund.balance

This accounts for:

- Deposits/withdrawals (vault ↔ capital)
- Trading PNL (zero-sum)
- Fees (PNL → insurance)
- ADL redistribution (PNL → losses)
- Funding rounding (vault-favoring: ceil on payments, truncate on receipts)

Any rounding dust is bounded by `MAX_ROUNDING_SLACK`.

---

## Formal Verification

All critical invariants are verified using **Kani** (bounded model checking).  
See `tests/kani.rs` for proof harnesses.

```bash
# Install Kani
cargo install --locked kani-verifier
cargo kani setup

# Run all proofs
cargo kani

# Run a specific proof
cargo kani --harness i1_adl_never_reduces_principal

Note:
Kani builds use MAX_ACCOUNTS = 8 for tractability.
Debug/fuzz builds use 64. Production builds use 4096.

⸻

Testing

# Unit tests
RUST_MIN_STACK=16777216 cargo test

# Fuzzing (property-based)
RUST_MIN_STACK=16777216 cargo test --features fuzz


⸻

Architecture
	•	#![no_std]
	•	#![forbid(unsafe_code)]
	•	Fixed-size account slab (4096 accounts in production)
	•	Bitmap-based allocation (O(1))
	•	O(N) ADL via bitmap scan
	•	Several-MB state footprint (~6MB in current layout)

⸻

Limitations
	•	No signature verification (external concern)
	•	No oracle implementation
	•	No account deallocation
	•	Maximum 4096 accounts
	•	Not audited for production use

⸻

License

Apache-2.0


