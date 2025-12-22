# Percolator: Risk Engine for Perpetual DEXs

⚠️ **EDUCATIONAL RESEARCH PROJECT — NOT PRODUCTION READY** ⚠️  
Do **NOT** use with real funds. Not audited. Experimental design.

Percolator is a **formally verified risk engine** for perpetual futures DEXs on Solana.

Its **primary design goal** is simple and strict:

> **No user can ever withdraw more value than actually exists on the exchange balance sheet.**

### Balance-Sheet-Backed Net Extraction (Formal Security Claim)

Concretely, **no sequence of trades, oracle updates, funding accruals, warmups, ADL events, panic settles, force-realize scans, or withdrawals can allow an attacker to extract net value that is not balance-sheet-backed**.

Formally, over any execution trace, define:

- **NetOut_A** = *Withdrawals_A − Deposits_A*  
  (successful withdrawals minus deposits made by the attacker)
- **LossPaid_notA** = total **realized losses actually paid from capital** by non-attacker accounts  
  (i.e. decreases in other users’ `capital` caused by settlement or force-realize)
- **SpendableInsurance_end** = `max(0, insurance_balance_end − insurance_floor)`

Then the engine enforces the invariant:

```
NetOut_A ≤ LossPaid_notA + SpendableInsurance_end
```

#### Interpretation

- The attacker can extract net value **only if**:
  - other users have **actually paid losses from principal** (capital decreased), or
  - the system explicitly spends **insurance above the protected threshold**.
- **Unrealized losses do not count**:
  - A “mule” account being underwater does **not** increase `LossPaid_notA`
    unless its capital is actually reduced.
- **Profits cannot outrun losses**:
  - Positive PNL must first be realized into capital via warmup,
  - and that realization is globally budgeted by paid losses and insurance.

#### Equivalent Withdrawal Bound

Equivalently, total attacker withdrawals satisfy:

```
Withdrawals_A ≤ Deposits_A + LossPaid_notA + SpendableInsurance_end
```

This means the attacker’s **realized withdrawable surplus** is strictly bounded by
value that already exists on the exchange balance sheet.

This property is enforced **by construction** and **proven with formal verification**
via an end-to-end Kani security harness in `tests/kani.rs`.

---

## Primary Security Goal

### Balance-Sheet Safety Guarantee

At all times:

> **Total withdrawals are bounded by real assets held by the system.**

More precisely, for any account (after funding + warmup settlement):

```
withdrawable ≤ max(0, capital + pnl)
```

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

```
pay = min(capital, -pnl)
capital -= pay
pnl += pay
```

This enforces (at settle boundaries):

```
pnl < 0  ⇒  capital == 0
```

**Consequence**
- A user cannot withdraw capital while losses exist.
- There are no “young losses” that can be delayed.

**Formally proven:** `N1` proofs in `tests/kani.rs`.

---

### 2. Equity-Based Withdrawals (I8)

Withdrawals are gated by **equity**, not nominal capital:

```
equity = max(0, capital + pnl)
```

Margin checks use equity consistently for:
- withdrawals,
- trading,
- liquidation thresholds.

**Consequence**
- Closing a position does not allow losses to be ignored.
- Negative PnL always reduces withdrawable value.

**Formally proven:** `I8` proofs in `tests/kani.rs`.

---

### 3. Profit Warmup (Time-Gating Artificial Gains) (I5)

Positive PnL must vest over time `T` before becoming capital:

- No instant withdrawal of profits.
- Warmup is deterministic and monotonic.
- Warmup can be frozen during insolvency (risk-reduction-only mode).

**Important:**  
Users **never withdraw PnL directly**.  
All withdrawals are from **capital**, and positive PnL must first be converted into capital via warmup settlement.

**Formally proven:** `I5` proofs in `tests/kani.rs`.

---

### 4. Global Warmup Budget (Prevents Profit Outrunning Losses) (WB-*)

Profit conversion is globally constrained by:

```
W⁺ ≤ W⁻ + max(0, I − I_min)
```

Where:
- `W⁺` = total profits converted to capital (`warmed_pos_total`)
- `W⁻` = total losses paid from capital (`warmed_neg_total`)
- `I − I_min` = insurance above the protected floor

**Formally proven:** `WB-A`, `WB-B`, `WB-C`, `WB-D`.

---

### 5. Reserved Insurance (Protects Matured Profits) (R1–R3)

Insurance above the floor is split into:
- **Reserved insurance**
- **Unreserved insurance**

ADL **cannot** spend reserved insurance.

**Formally proven:** `R1`, `R2`, `R3`.

---

### 6. ADL Cannot Touch Principal (I1)

ADL:
- Only haircuts **unwrapped (young) PnL**
- Never reduces `capital`

**Formally proven:** `I1`.

---

### 7. Risk-Reduction-Only Mode (I10)

When insurance is exhausted or losses are uncovered:
- Warmup frozen
- Risk-increasing trades blocked
- Only position reduction and limited withdrawals allowed

**Formally proven:** `I10` series.

---

### 8. Forced Loss Realization

Allows the system to unstick at insurance floor by forcing loss realization.

**Formally proven:** `force_realize` proofs.

---

### 9. Conservation with Bounded Slack (I2)

```
vault + loss_accum ≥ sum(capital) + sum(pnl) + insurance
```

Bounded rounding dust via `MAX_ROUNDING_SLACK`.

**Formally proven:** `I2`, `C1`, `C1b`.

---

### 10. Net Extraction Bound (SEC)

For any bounded sequence:

```
withdrawals − deposits ≤ losses_paid_by_others + insurance_above_floor
```

**Formally proven:** end-to-end security harness in `tests/kani.rs`.

---

## Formal Verification

All properties are **machine-checked** using **Kani**.

```bash
cargo install --locked kani-verifier
cargo kani setup
cargo kani
```

Notes:
- `MAX_ACCOUNTS = 8` in Kani
- Debug/fuzz = 64
- Production = 4096

---

## Testing

```bash
RUST_MIN_STACK=16777216 cargo test
RUST_MIN_STACK=16777216 cargo test --features fuzz
```

---

## Architecture

- `#![no_std]`
- `#![forbid(unsafe_code)]`
- Fixed-size slab
- Bitmap allocation
- O(N) ADL
- ~6MB state

---

## Limitations

- No signature verification
- No oracle implementation
- No account deallocation
- Not audited

---

## License

Apache-2.0
