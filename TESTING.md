# Percolator Protocol - Complete Testing Guide

## Overview

This guide covers the comprehensive 8-phase "Kitchen Sink" E2E test suite that validates the entire Percolator protocol, including crisis scenarios and loss socialization.

## Quick Start

### One-Command Test Execution

```bash
./run_all_phases.sh
```

This script handles everything:
1. Cleans up existing validators
2. Builds all Solana programs
3. Starts test validator
4. Deploys programs
5. Builds CLI
6. Runs all 8 crisis test phases
7. Generates comprehensive logs

### Keep Validator Running

```bash
./run_all_phases.sh --keep-validator
```

Useful for debugging or running additional manual tests.

## What Gets Tested

### The 8-Phase Kitchen Sink E2E Test

**Test Location**: `cli/src/tests.rs:1707` (`test_loss_socialization_integration`)

#### Phase 1: Registry Initialization
- Creates protocol registry with governance
- Sets liquidation parameters (IMR, MMR, bands)
- Initializes insurance fund

#### Phase 2: Portfolio Setup
- Creates portfolios for Alice, Bob, Dave, Erin
- Deposits initial collateral ($1000 each)
- Verifies portfolio state

#### Phase 3: SOL-PERP Market Creation
- Creates SOL-PERP perpetual market (slab)
- Deploys price oracle at $100
- Configures market parameters

#### Phase 4: Alice Provides Liquidity
- Alice posts resting limit sell at $101
- Validates order book state
- Checks margin requirements

#### Phase 5: Bob Opens Long Position
- Bob market buys, crosses Alice's quote
- Creates long position exposure
- Verifies trade execution and settlement

#### Phase 6: Price Movement & PnL
- Oracle updates to $110 (+10%)
- Bob's position shows $10k unrealized profit
- Alice's position shows -$10k unrealized loss
- Validates mark-to-market calculations

#### Phase 7: Dave Liquidation
- Dave opens 5x leveraged long at $100
- Price crashes to $50 (-50%)
- Dave's position liquidated
- Insurance fund covers $25k bad debt

#### Phase 8: Frank Socialization (NEW!)
- Frank opens **10x leveraged long** (1000 contracts @ $100)
- **Catastrophic 90% crash**: $100 → $10
- **Massive bad debt**: -$89,000
- **Insurance fund exhausted**
- **Global haircut applied** via socialization
- All users share uncovered losses proportionally

## Test Phases Status

| Phase | Description | Code Lines | Status |
|-------|-------------|------------|--------|
| 1 | Registry Init | 1707-1850 | ✓ Implemented |
| 2 | Portfolios | 1852-2070 | ✓ Implemented |
| 3 | SOL-PERP Setup | 2072-2290 | ✓ Implemented |
| 4 | Alice Quote | 2292-2480 | ✓ Implemented |
| 5 | Bob Trade | 2482-2720 | ✓ Implemented |
| 6 | PnL Movement | 2722-2880 | ✓ Implemented |
| 7 | Dave Liquidation | 2882-3140 | ✓ Implemented |
| 8 | Frank Socialization | 3142-3880 | ✓ Implemented |

**Total**: ~2,200 lines of comprehensive E2E test code

## Manual Testing

### Prerequisites

1. **Build programs**:
   ```bash
   cargo build-sbf
   ```

2. **Start validator**:
   ```bash
   solana-test-validator --reset --quiet &
   sleep 8  # Wait for startup
   ```

3. **Deploy programs**:
   ```bash
   for program in target/deploy/*.so; do
       solana program deploy "$program"
   done
   ```

4. **Build CLI**:
   ```bash
   cargo build --release --package percolator-cli
   ```

### Run Tests

#### All Crisis Tests (includes 8-phase kitchen sink)
```bash
cargo run --release --package percolator-cli --bin percolator -- test --crisis
```

#### All Available Tests
```bash
# Quick smoke tests
cargo run --release --package percolator-cli --bin percolator -- test --quick

# Margin system tests
cargo run --release --package percolator-cli --bin percolator -- test --margin

# Order management tests
cargo run --release --package percolator-cli --bin percolator -- test --orders

# Trade matching tests
cargo run --release --package percolator-cli --bin percolator -- test --matching

# Liquidation tests
cargo run --release --package percolator-cli --bin percolator -- test --liquidations

# ALL tests
cargo run --release --package percolator-cli --bin percolator -- test --all
```

## Log Files

The `run_all_phases.sh` script generates several logs:

- `/tmp/percolator_test_YYYYMMDD_HHMMSS.log` - Complete execution log
- `/tmp/crisis_test_output_YYYYMMDD_HHMMSS.log` - Test results
- `/tmp/validator.log` - Validator output

## Expected Results

### Successful Test Run

```
═══════════════════════════════════════════════════════════════
  Kitchen Sink E2E Test (KS-00)
═══════════════════════════════════════════════════════════════

Multi-phase comprehensive test covering:
  • Multi-market setup (SOL-PERP, BTC-PERP)
  • Multiple actors (Alice, Bob, Dave, Erin, Frank, Keeper)
  • Order book liquidity and taker trades
  • Funding rate accrual
  • Oracle shocks and liquidations
  • Insurance fund stress
  • Global haircut socialization
  • Cross-phase invariants

═══ Setup: Actors & Initial State ═══
  ✓ All actors funded

═══ Phase 1 (KS-01): Bootstrap Books & Reserves ═══
  ✓ Registry initialized
  ✓ Insurance fund created

═══ Phase 2 (KS-02): Create Portfolios ═══
  ✓ Alice portfolio created
  ✓ Bob portfolio created
  ✓ Dave portfolio created
  ✓ Erin portfolio created

═══ Phase 3 (KS-03): Create SOL-PERP Market ═══
  ✓ SOL-PERP slab created
  ✓ Oracle initialized at $100

═══ Phase 4 (KS-04): Alice Resting Quote ═══
  ✓ Alice posted limit sell @ $101

═══ Phase 5 (KS-05): Bob Crosses ═══
  ✓ Bob executed market buy
  ✓ Trade settled

═══ Phase 6 (KS-06): Price Movement ═══
  ✓ Price moved to $110
  ✓ PnL updated correctly

═══ Phase 7 (KS-07): Dave Liquidation ═══
  ✓ Dave opened 5x long
  ✓ Price crashed to $50
  ✓ Dave liquidated
  ✓ Insurance covered bad debt

═══ Phase 8 (KS-08): Frank Socialization ═══
  ✓ Frank opened 10x long (1000 contracts @ $100)
  ✓ Catastrophic crash: $100 → $10 (-90%)
  ✓ Frank liquidated with -$89k bad debt
  ✓ Insurance fund exhausted
  ✓ Global haircut applied
  ✓ Socialization mechanism validated

═══ FINAL STATE ═══
  Registry: active
  Insurance: exhausted (socialization triggered)
  Global PnL Index: adjusted for haircut
  All invariants: ✓ passed
```

## Troubleshooting

### Test Fails at Phase 1

**Symptom**: `✗ Failed to send transaction`

**Cause**: Programs not deployed

**Fix**:
```bash
cargo build-sbf
for program in target/deploy/*.so; do
    solana program deploy "$program"
done
```

### Validator Not Starting

**Symptom**: `Validator failed to start`

**Fix**:
```bash
# Kill existing validators
killall -9 solana-test-validator

# Clean ledger
rm -rf test-ledger

# Restart
solana-test-validator --reset --quiet &
```

### Programs Not Found

**Symptom**: `Programs directory not found: target/deploy`

**Fix**:
```bash
# Build programs first
cargo build-sbf
```

## Architecture

### Test Entry Points

- **Main CLI**: `cli/src/main.rs:932` - Command handler
- **Crisis Tests**: `cli/src/tests.rs:566` - `run_crisis_tests()`
- **Kitchen Sink**: `cli/src/tests.rs:1707` - `test_loss_socialization_integration()`

### Key Test Utilities

- **Network Config**: `cli/src/config.rs` - RPC client setup
- **Instruction Builders**: Throughout `cli/src/tests.rs` - Transaction construction
- **State Verification**: Inline assertions in each phase

## Phase 8 Implementation Details

Phase 8 tests the worst-case crisis scenario:

1. **Setup**: Frank funded with 1000 SOL
2. **Leverage**: Opens 1000 contracts (10x) at $100
3. **Exposure**: $100,000 notional with $10,000 margin
4. **Shock**: 90% price crash ($100 → $10)
5. **Loss**: -$90,000 position value
6. **Bad Debt**: -$89,000 (after $1,000 margin liquidated)
7. **Insurance**: Exhausted trying to cover
8. **Socialization**: Uncovered loss distributed via `pnl_index`
9. **Verification**:
   - `registry.global_haircut.uncovered_bad_debt > 0`
   - `registry.global_haircut.pnl_index < 1e18` (haircut applied)
   - All user PnL adjusted proportionally

**Code Location**: `cli/src/tests.rs:3467-3821` (Phase 8 implementation)

## Contributing

When adding new test phases:

1. Add phase to `test_loss_socialization_integration()`
2. Update phase count in test banner
3. Add verification assertions
4. Update this TESTING.md
5. Test with `./run_all_phases.sh`

## Resources

- **Test Code**: `cli/src/tests.rs`
- **Runner Script**: `run_all_phases.sh`
- **CLI Main**: `cli/src/main.rs`
- **Protocol Docs**: See main README.md
