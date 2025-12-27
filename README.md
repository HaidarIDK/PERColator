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

### Trading

- Wrapper validates signatures and oracles.
- Matching engine executes.
- Wrapper calls `RiskEngine::execute_trade(...)`.

### Keeper Crank

`RiskEngine::keeper_crank(...)` is permissionless, safe at any time, and no‑op when idle.

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

