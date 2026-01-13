# Percolator: Risk Engine for Perpetual DEXs

⚠️ **EDUCATIONAL RESEARCH PROJECT — NOT PRODUCTION READY** ⚠️  
Do **NOT** use with real funds. Not audited. Experimental design.

Percolator is a **formally verified risk engine** for perpetual futures DEXs on Solana.

Its **primary design goal** is simple and strict:

> **No user can ever withdraw more value than actually exists on the exchange balance sheet.**

---

## Balance‑Sheet‑Backed Net Extraction (Formal Security Claim)

Concretely, **no sequence of trades, oracle updates, funding accruals, warmups, ADL events, panic settles, force‑realize scans, or withdrawals can allow an attacker to extract net value that is not balance‑sheet‑backed**.

### Formal Statement

Over any execution trace, define:

- **NetOutₐ** = Withdrawalsₐ − Depositsₐ  
- **LossPaid¬ₐ** = realized losses actually paid from capital by non‑attacker accounts  
- **SpendableInsurance_end** = max(0, insurance_balance_end − insurance_floor)

Then the engine enforces:

```
NetOutₐ ≤ LossPaid¬ₐ + SpendableInsurance_end
```

Equivalently:

```
Withdrawalsₐ ≤ Depositsₐ + LossPaid¬ₐ + SpendableInsurance_end
```

This property is enforced **by construction** and **proven with formal verification**.

---

## Top‑Level Program API (Wrapper Usage)

Percolator is a **pure accounting and risk engine**.  
It **does not move tokens**.

All real token transfers must occur **outside** the engine, and the wrapper program
must verify balance deltas before calling into it.

### Deposits

1. Transfer tokens into the vault SPL token account.
2. Verify: `vault_balance_after − engine.vault == amount`
3. Call `RiskEngine::deposit(account_id, amount)`

### Withdrawals

1. Call `RiskEngine::withdraw(account_id, amount, now_slot, oracle_price)`
2. If successful, transfer tokens out of the vault.

**Users never withdraw PnL directly.**

**Safety requirements enforced by withdraw:**

- **Fresh crank** — crank must have run recently (prevents stale state)
- **Recent sweep started** — a full 16‑step sweep must have started recently (ensures liquidations are current)
- **No pending socialization** — blocks withdrawals while `pending_profit_to_fund` or `pending_unpaid_loss` are non‑zero (prevents extracting unfunded value)

### Trading

- Wrapper validates signatures and oracles.
- Matching engine executes.
- Wrapper calls `RiskEngine::execute_trade(...)`.

### Keeper Crank & Liquidation

`RiskEngine::keeper_crank(...)` is permissionless, safe at any time, and no‑op when idle.

#### 16‑Step Windowed Sweep

To bound compute usage, the crank uses a **16‑step windowed sweep**:

- **NUM_STEPS = 16** — a full sweep requires 16 crank calls
- **WINDOW = 256** — each call scans up to 256 accounts
- **16 × 256 = 4096** — maximum accounts covered per full sweep

**On each call**, keeper_crank performs:

1. **Funding accrual** — updates global funding index
2. **Caller maintenance settle** (best effort) — settles maintenance fees for the calling account with 50% forgiveness
3. **Windowed liquidation scan** — scans accounts in the current window (1/16th of total) with budget limit
4. **Windowed force‑realize** — if insurance is critical, force‑closes positions in current window
5. **Garbage collection** — closes dust accounts with zero balance
6. **Socialization step** — applies pending profit/loss haircuts to accounts in current window
7. **LP max position update** — tracks maximum LP position size for risk limits

**Budget limits per crank call:**

| Action | Budget |
|--------|--------|
| Liquidations | 120 max |
| Force‑realize closes | 32 max |
| Garbage collection | 8 max |

**Sweep completion:**

After step 15 completes, `finalize_pending_after_window()` runs to:
- Fund any remaining pending profit from insurance
- Absorb any remaining unpaid loss into `loss_accum`
- Commit the swept `lp_max_abs` value

The sweep then restarts from step 0.

#### Liquidation Semantics

- Accounts are closed at the **oracle price** (no LP/AMM required)
- Liquidation fee (default: 0.5% of notional) is paid from account capital to insurance

**Mark PnL routing (critical for invariant preservation):**

| Scenario | mark_pnl | Effect | Routing |
|----------|----------|--------|---------|
| User has profit on close | > 0 | System deficit | → `apply_adl()` (socializes via ADL waterfall) |
| User has loss on close | < 0 | System surplus | → `insurance_fund.balance` |

- **Deficit** (mark_pnl > 0): The liquidated user has unrealized profit. This profit must come from somewhere, so it's socialized via ADL (haircutting other accounts' positive PnL).
- **Surplus** (mark_pnl < 0): The liquidated user has unrealized loss. This loss benefits the system, so it's credited to insurance.

After mark PnL routing:
- `settle_warmup_to_capital()` realizes any remaining PnL (negative PnL reduces capital immediately; positive PnL is subject to warmup budget)
- Liquidation fee is deducted from remaining capital

#### ADL Socialization

When liquidations create deficits (profitable positions being closed), the system socializes losses:

1. **Immediate ADL** — haircuts positive PnL from accounts in priority order (highest PnL first)
2. **Pending accumulation** — unfunded profit and unpaid losses accumulate in pending buckets
3. **Per‑window socialization** — each crank step applies pending haircuts to accounts in current window
4. **End‑of‑sweep finalization** — after 16 steps, remaining pending amounts are resolved via insurance or loss_accum

This ensures losses are distributed fairly while maintaining bounded compute per transaction.

### Closing Accounts

`RiskEngine::close_account(...)` returns **capital only** after full settlement.

---

## Formal Verification

All invariants are machine‑checked using **Kani**.

```bash
cargo install --locked kani-verifier
cargo kani setup
cargo kani
```

---

## License

Apache‑2.0

