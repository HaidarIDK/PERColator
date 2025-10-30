# Kani Safety Proofs for Percolator

This directory contains formal verification proofs for the percolator safety module using [Kani](https://model-checking.github.io/kani/), a bit-precise model checker for Rust.

## Overview

The proofs verify 6 core invariants across the percolator safety/accounting system:

1. **I1: Principal Inviolability** - User deposits minus withdrawals never affected by losses
2. **I2: Conservation** - Vault accounting always balances
3. **I3: Authorization** - Only authorized router can mutate balances
4. **I4: Bounded Socialization** - Losses only hit winners, capped at available PnL
5. **I5: Throttle Safety** - PnL withdrawals respect warm-up limits
6. **I6: Matcher Immutability** - Matcher operations can't move funds

See [COVERAGE.md](./COVERAGE.md) for detailed coverage information.

## Structure

```
crates/
├── model_safety/          # Pure Rust model (no Solana deps)
│   ├── src/
│   │   ├── state.rs       # State structures
│   │   ├── math.rs        # Safe arithmetic
│   │   ├── warmup.rs      # PnL vesting logic
│   │   ├── helpers.rs     # Invariant checkers
│   │   └── transitions.rs # State transitions
│   └── Cargo.toml
└── proofs/kani/           # Kani proof harnesses
    ├── src/
    │   ├── safety.rs      # 6 main proofs + extras
    │   ├── sanitizer.rs   # State space bounding
    │   ├── generators.rs  # Arbitrary state generation
    │   └── adversary.rs   # Adversarial transitions
    ├── Cargo.toml
    ├── README.md          # This file
    └── COVERAGE.md        # Coverage checklist
```

## Installation

### Install Kani

```bash
# Install Kani (requires cargo)
cargo install --locked kani-verifier

# Install Kani's solver (CBMC)
cargo kani setup
```

For more details, see the [Kani installation guide](https://model-checking.github.io/kani/install-guide.html).

## Running Proofs

### Run All Proofs

```bash
# From workspace root
cargo kani -p proofs-kani
```

### Run with Bounded Unwinding

For proofs with loops (like the adversarial sequence test), you may need to increase unwinding:

```bash
cargo kani -p proofs-kani --default-unwind 8
```

### Run Specific Proof

```bash
# Run just the principal inviolability proof
cargo kani -p proofs-kani --harness i1_principal_never_cut_by_socialize

# Run just the conservation proof
cargo kani -p proofs-kani --harness i2_conservation_holds_across_short_adversary_sequences
```

### Run with Extra Checks

```bash
cargo kani -p proofs-kani --cbmc-args "--bounds-check --pointer-check --signed-overflow-check"
```

## Understanding Results

### Success Output

```
VERIFICATION:- SUCCESSFUL
```

All properties verified exhaustively within the bounded state space.

### Failure Output

If Kani finds a counterexample, it will print:

```
VERIFICATION:- FAILED

Trace:
  Step 1: [concrete values that trigger the bug]
  Step 2: ...
```

Copy these values into a unit test in `model_safety/tests/` to create a regression test.

## Proof Harnesses

All proof harnesses are in `src/safety.rs`:

| Harness | Invariant | Description |
|---------|-----------|-------------|
| `i1_principal_never_cut_by_socialize` | I1 | Principals unchanged by socialization |
| `i2_conservation_holds_across_short_adversary_sequences` | I2 | Vault balances across 6-step sequences |
| `i3_unauthorized_cannot_mutate` | I3 | Authorization enforcement |
| `i4_socialization_hits_winners_only_and_caps` | I4 | Haircut distribution correctness |
| `i5_withdraw_throttle_safety` | I5 | Warm-up cap enforcement |
| `i6_matcher_cannot_move_funds` | I6 | Matcher immutability |
| `principal_withdrawal_reduces_principal` | Extra | Withdrawal correctness |
| `deposit_increases_principal_and_vault` | Extra | Deposit correctness |

## Bounded State Space

Kani explores **all** executions within these bounds:

- **Users**: Up to 5 users per state
- **Sequence Length**: Up to 6 transitions (configurable via `--default-unwind`)
- **Values**: Bounded to stress overflow/underflow edges (~1T for most values)

These bounds are sufficient to catch:
- Arithmetic errors (overflow, underflow, division by zero)
- Logic errors (incorrect haircut distribution, conservation violations)
- Authorization bypasses
- Off-by-one errors

## Limitations

1. **No Concurrency**: Single-threaded model (Solana programs are single-threaded anyway)
2. **Simplified Conservation**: Real vault accounting may have additional complexity
3. **Bounded Exploration**: Not full unbounded proof (but very thorough within bounds)

## Integration with CI

Add to your CI pipeline:

```yaml
- name: Kani Safety Proofs
  run: |
    cargo install --locked kani-verifier
    cargo kani setup
    cargo kani -p proofs-kani --default-unwind 8
```

## Debugging Failed Proofs

1. **Extract Counterexample**: Copy the concrete values from Kani's trace
2. **Create Regression Test**: Add to `model_safety/tests/regression.rs`
3. **Fix Logic**: Update `transitions.rs` or `helpers.rs`
4. **Re-run**: Verify fix with `cargo kani -p proofs-kani --harness <failed_harness>`
5. **Commit**: Add regression test to prevent future breakage

## Performance

Typical runtime on a modern machine:

- **Single Proof**: 5-30 seconds
- **All Proofs**: 2-5 minutes
- **With High Unwinding** (--default-unwind 16): 10-20 minutes

## References

- [Kani Documentation](https://model-checking.github.io/kani/)
- [Kani Tutorial](https://model-checking.github.io/kani/kani-tutorial.html)
- [CBMC Manual](https://www.cprover.org/cbmc/)

## Questions?

See [COVERAGE.md](./COVERAGE.md) for the verification checklist, or consult the Kani docs for troubleshooting.
