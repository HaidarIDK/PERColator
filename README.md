# Percolator: Risk Engine for Perpetual DEXs

⚠️ **EDUCATIONAL RESEARCH PROJECT — NOT PRODUCTION READY** ⚠️  
Do **NOT** use with real funds. Not audited. Experimental design.

Percolator is a **formally verified risk engine** for perpetual futures DEXs on Solana.

Its **primary design goal** is simple and strict:

> **No user can ever withdraw more value than actually exists on the exchange balance sheet.**

Concretely, **no sequence of trades, oracle updates, funding accruals, warmups, ADL events, or withdrawals can allow an attacker to extract more than**:
- their realized equity,
- plus realized losses paid by other users,
- plus insurance fund balance **above the protected threshold**.

This property is enforced by construction and proven with formal verification.

---

## Primary Security Goal (What This Engine Guarantees)

### Balance-Sheet Safety Guarantee

At all times:

> **Total user withdrawals are bounded by real assets held by the system.**

More precisely, a user can never withdraw more than:

max(0, capital + pnl)

subject to:
- margin requirements, and
- global solvency constraints.

They **cannot**:
- withdraw principal while losses are still unpaid,
- mature artificial profits faster than losses are realized,
- drain insurance backing other users’ profits,
- exploit funding, rounding, or timing gaps to mint value.

This is the core invariant the entire design enforces.

---

## Design Overview

### The Fundamental Problem

Oracle manipulation allows attackers to:
1. Create artificial mark-to-market profits,
2. Close positions,
3. Withdraw funds before losses are realized.

Most historical perp exploits follow this exact pattern.

---

### Core Insight

Percolator prevents this by enforcing **asymmetric treatment of profits and losses**:

- **Positive PNL is time-gated** (warmup).
- **Negative PNL is realized immediately**.
- **Profit maturation is globally budgeted** by realized losses and insurance.
- **ADL only touches profits, never principal**.

There is no timing window where profits can outrun losses.

---

## How the Code Enforces the Primary Goal

### 1. Immediate Loss Realization (Fix N1)

Negative PNL is **never** time-gated.

In `settle_warmup_to_capital`:

pay = min(capital, -pnl)
capital -= pay
pnl += pay

This enforces the invariant:

pnl < 0  ⇒  capital == 0

**Consequence:**
- A user cannot withdraw capital while losses exist.
- “Young losses” do not exist — losses are paid immediately.

**Formally proven:** `N1` proofs in `tests/kani.rs`.

---

### 2. Equity-Based Withdrawals (Fix I8)

Withdrawals are gated by **equity**, not nominal capital:

equity = max(0, capital + pnl)

Margin checks use equity everywhere:
- Withdrawals
- Trading
- Liquidation thresholds

**Consequence:**
- Closing a position does not let a user ignore losses.
- Negative PNL always reduces withdrawable value.

**Formally proven:** `I8` proofs.

---

### 3. Profit Warmup (Time-Gating Artificial Gains)

Positive PNL must vest over time `T` before becoming capital:

- No instant withdrawal of profits.
- Warmup is deterministic and monotonic.
- Warmup can be frozen during insolvency.

**Formally proven:** `I5`.

---

### 4. Global Warmup Budget (Prevents Profit Outrunning Losses)

Profit conversion is globally constrained by:

W⁺ ≤ W⁻ + max(0, I − I_min)

Where:
- `W⁺` = total profits converted to capital,
- `W⁻` = total losses paid from capital,
- `I − I_min` = insurance above the protected floor.

This ensures:
- Profits can only mature if **someone has already paid losses** or **insurance explicitly backs them**.
- Artificial profits cannot be withdrawn unless they are balance-sheet-backed.

**Formally proven:** Warmup budget proofs (`WB-A`, `WB-B`, `WB-C`, `WB-D`).

---

### 5. Reserved Insurance (Protects Already-Warmed Profits)

Insurance above the floor is split into:
- **Reserved insurance** (backs warmed profits),
- **Unreserved insurance** (can be spent by ADL).

ADL **cannot** spend reserved insurance.

**Consequence:**
- One user cannot drain insurance backing another user’s matured profits.
- Insurance use is ordered and safe.

**Formally proven:** `R1`, `R2`, `R3`.

---

### 6. ADL Cannot Touch Principal (I1)

ADL:
- Only haircuts **unwrapped (young) PNL**,
- Never reduces `capital`.

Even in extreme insolvency, principal is not socialized.

**Formally proven:** `I1` (users and LPs).

---

### 7. Risk-Reduction-Only Mode (Closes Timing Windows)

When insurance is exhausted or losses are uncovered:
- Warmup is frozen,
- Risk-increasing trades are blocked,
- Only position reduction and limited withdrawals are allowed.

This prevents:
- Waiting out warmup while the system is insolvent.

**Formally proven:** `I10` series.

---

### 8. Forced Loss Realization (Threshold Unstick)

When insurance is at/below the floor, the engine can:
- Close all positions at the oracle price,
- Force losses to be paid from capital,
- Socialize any unpaid remainder via ADL.

This guarantees:
- Losses are realized before any further profit maturation.
- The system cannot deadlock with “paper profits”.

---

### 9. Conservation with Bounded Slack (I2)

Conservation is enforced one-sided:

vault + loss_accum ≥ sum(capital) + sum(pnl) + insurance

- Funding rounds in a vault-favoring way.
- Any rounding dust is bounded by `MAX_ROUNDING_SLACK`.

**Consequence:**
- The vault always has at least what accounts think they own.
- No minting via arithmetic edge cases.

**Formally proven:** `I2`, `C1`, `C1b`.

---

## Proven Invariants Summary

| ID | What It Prevents |
|----|------------------|
| **I1** | ADL stealing principal |
| **I2** | Minting or balance-sheet drift |
| **I4** | Insurance being spent before profits |
| **I5** | Time-based profit exploits |
| **I8** | Ignoring losses via withdrawals |
| **N1** | “Young losses” withdrawal exploit |
| **R1–R3** | Draining insurance backing profits |
| **I10** | Insolvency timing attacks |

---

## Formal Verification

All properties above are **machine-checked** using **Kani**.

Proofs live in `tests/kani.rs` and cover:
- safety invariants,
- frame conditions,
- edge cases,
- insolvency transitions,
- rounding behavior.

```bash
cargo install --locked kani-verifier
cargo kani setup
cargo kani

Notes:
	•	MAX_ACCOUNTS = 8 in Kani for tractability,
	•	Debug/fuzz uses 64,
	•	Production uses 4096.

⸻

Testing

# Unit tests
RUST_MIN_STACK=16777216 cargo test

# Fuzzing
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
	•	No signature verification
	•	No oracle implementation
	•	No account deallocation
	•	Maximum 4096 accounts
	•	Not audited for production use

⸻

License

Apache-2.0


