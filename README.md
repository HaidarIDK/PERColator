# Percolator: Risk Engine for Perpetual DEXs

⚠️ **EDUCATIONAL RESEARCH PROJECT — NOT PRODUCTION READY** ⚠️  
Do **NOT** use with real funds. Not audited. Experimental design.

Percolator is a **formally verified risk engine** for perpetual futures DEXs on Solana.

Its **primary design goal** is simple and strict:

> **No user can ever withdraw more value than actually exists on the exchange balance sheet.**

Concretely, **no sequence of trades, oracle updates, funding accruals, warmups, ADL events, panic settles, force-realize scans, or withdrawals can allow an attacker to extract more than**:
- their **realized equity** (i.e., what can actually become withdrawable capital),
- plus **realized losses paid by other users**,
- plus **insurance fund balance above the protected threshold**.

This property is enforced **by construction** and **proven with formal verification**.

---

## Primary Security Goal

### Balance-Sheet Safety Guarantee

At all times:

> **Total withdrawals are bounded by real assets held by the system.**

More precisely, for any account (after funding + warmup settlement):

withdrawable ≤ max(0, capital + pnl)

subject to:
- equity-based margin requirements, and
- global solvency constraints.

Users **cannot**:
- withdraw principal while losses are unpaid,
- mature artificial profits faster than losses are realized,
- drain insurance backing other users’ profits,
- exploit funding, rounding, or timing gaps to mint value.

This invariant is the core property the entire engine enforces.

---

## Design Overview

### The Fundamental Problem

Oracle manipulation enables a classic exploit pattern:

1. Create artificial mark-to-market profits,
2. Close positions,
3. Withdraw funds before losses are realized.

Most historical perpetual DEX exploits follow this sequence.

---

### Core Insight

Percolator eliminates this attack surface by enforcing **asymmetric treatment of profits and losses**:

- **Positive PnL is time-gated** (warmup).
- **Negative PnL is realized immediately**.
- **Profit maturation is globally budgeted** by realized losses and insurance (above the floor).
- **ADL only touches profits, never principal**.

There is **no timing window** in which profits can outrun losses.

---

## How the Code Enforces the Primary Goal

### 1. Immediate Loss Realization (N1)

Negative PnL is **never** time-gated.

In `settle_warmup_to_capital`:

```text
pay = min(capital, -pnl)
capital -= pay
pnl += pay
```

This enforces (at settle boundaries):

```pnl < 0  ⇒  capital == 0```

Consequence
 * A user cannot withdraw capital while losses exist.
 * There are no “young losses” that can be delayed.

Formally proven: N1 proofs in tests/kani.rs.

---

2. Equity-Based Withdrawals (I8)

Withdrawals are gated by equity, not nominal capital:

equity = max(0, capital + pnl)

Margin checks use equity consistently for:
 * withdrawals,
 * trading,
 * liquidation thresholds.

Consequence
 * Closing a position does not allow losses to be ignored.
 * Negative PnL always reduces withdrawable value.

Formally proven: I8 proofs in tests/kani.rs.

---

3. Profit Warmup (Time-Gating Artificial Gains)

Positive PnL must vest over time T before becoming capital:
 * No instant withdrawal of profits.
 * Warmup is deterministic and monotonic.
 * Warmup can be frozen during insolvency (risk-reduction-only mode).

Important:
Users never withdraw PnL directly.
All withdrawals are from capital, and positive PnL must first be converted into capital via warmup settlement.

Formally proven: I5 proofs in tests/kani.rs.

---

4. Global Warmup Budget (Prevents Profit Outrunning Losses)

Profit conversion is globally constrained by:

W⁺ ≤ W⁻ + max(0, I − I_min)

Where:
 * W⁺ = total profits converted to capital,
 * W⁻ = total losses paid from capital,
 * I − I_min = insurance above the protected floor (insurance_spendable_raw()).

Consequence
 * Profits can only mature if losses have already been paid, or insurance explicitly backs them.
 * Artificial profits cannot be withdrawn unless balance-sheet-backed.

Formally proven: WB-A, WB-B, WB-C, WB-D proofs in tests/kani.rs.

---

5. Reserved Insurance (Protects Matured Profits)

Insurance above the floor is split into:
 * Reserved insurance (backs already-warmed profits),
 * Unreserved insurance (available to ADL).

ADL cannot spend reserved insurance.

Consequence
 * One user cannot drain insurance backing another user’s matured profits.
 * Insurance use is ordered and safe.

Formally proven: R1, R2, R3 proofs in tests/kani.rs.

---

6. ADL Cannot Touch Principal (I1)

ADL:
 * Only haircuts unwrapped (young) PnL,
 * Never reduces capital.

Even in extreme insolvency, principal is not socialized.

Formally proven: I1 proofs (users and LPs) in tests/kani.rs.

---

7. Risk-Reduction-Only Mode (Closes Timing Windows)

When insurance is exhausted or losses are uncovered:
 * Warmup is frozen,
 * Risk-increasing trades are blocked,
 * Only position reduction and limited withdrawals are allowed.

This prevents:
 * Waiting out warmup while the system is insolvent.

Formally proven: I10 series in tests/kani.rs.

---

8. Forced Loss Realization (Threshold Unstick)

When insurance reaches the protected floor, the engine can:
 * Close all positions at the oracle price,
 * Force losses to be paid from capital,
 * Socialize any unpaid remainder via ADL.

Guarantee
 * Losses are realized before any further profit maturation.
 * The system cannot deadlock with “paper profits”.

Formally proven: force_realize / audit proofs in tests/kani.rs.

---

9. Conservation with Bounded Slack (I2)

Conservation is enforced one-sided:

vault + loss_accum ≥ sum(capital) + sum(pnl) + insurance

 * Funding rounds in a vault-favoring way.
 * Any rounding dust is bounded by MAX_ROUNDING_SLACK.

Consequence
 * The vault always has at least what accounts believe they own.
 * No minting via arithmetic edge cases.

Formally proven: I2, C1, C1b proofs in tests/kani.rs.

---

10. Net Extraction Bound (End-to-End Security Property)

For any bounded sequence of operations, define:

net_extraction = withdrawals − deposits

Then the engine enforces the end-to-end bound:

net_extraction ≤ (realized losses paid by other users)
              + (insurance above protected threshold)

Implementation detail (important for correctness): insurance “above floor” is tracked as the
total spendable insurance ever available over the trace (spent + remaining), not just the final balance.

This is the formal statement of the primary security goal:
no attacker can extract value that is not balance-sheet-backed.

Formally proven: security_goal_bounded_net_extraction_sequence harness in tests/kani.rs.

---

Proven Invariants Summary

ID	What It Prevents
I1	ADL stealing principal
I2	Minting or balance-sheet drift
I4	Insurance being spent before profits
I5	Time-based profit exploits
I8	Ignoring losses via withdrawals
N1	“Young losses” withdrawal exploit
R1–R3	Draining insurance backing profits
I10	Insolvency timing attacks
SEC	Net extraction beyond balance-sheet resources


---

Formal Verification

All properties above are machine-checked using Kani.

Proofs live in tests/kani.rs and cover:
 * safety invariants,
 * frame conditions,
 * insolvency transitions,
 * rounding behavior,
 * bounded adversarial traces.

cargo install --locked kani-verifier
cargo kani setup
cargo kani

Notes
 * MAX_ACCOUNTS = 8 in Kani (tractability)
 * debug/fuzz uses 64
 * production uses 4096

---

## Testing

# Unit tests
RUST_MIN_STACK=16777216 cargo test

# Fuzzing
RUST_MIN_STACK=16777216 cargo test --features fuzz


---

## Architecture
 * #![no_std]
 * #![forbid(unsafe_code)]
 * Fixed-size account slab (4096 accounts in production)
 * Bitmap-based allocation (O(1))
 * O(N) ADL via bitmap scan
 * Several-MB state footprint (~6MB in current layout)

---

## Limitations
 * No signature verification
 * No oracle implementation
 * No account deallocation
 * Maximum 4096 accounts
 * Not audited for production use

---

License

Apache-2.0


