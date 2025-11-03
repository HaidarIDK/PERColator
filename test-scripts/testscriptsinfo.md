# Test Scripts

This folder contains all test shell scripts for running Percolator tests.

## Contents

### Main Test Runners
- `run_all_tests.sh` - Run all tests
- `run_all_orderbook_tests.sh` - Run all orderbook tests

### Router LP Tests
- `test_router_lp_amm.sh` - Router LP with AMM
- `test_router_lp_mixed.sh` - Router LP with mixed venues
- `test_router_lp_slab.sh` - Router LP with Slab

### Orderbook Tests
- `test_orderbook_comprehensive.sh` - Comprehensive orderbook tests
- `test_orderbook_extended.sh` - Extended orderbook tests
- `test_orderbook_simple.sh` - Simple orderbook tests
- `test_orderbook_working.sh` - Working orderbook tests

### Matching Engine Tests
- `test_matching_engine.sh` - Matching engine tests
- `test_matching_scenarios.sh` - Matching scenario tests
- `test_modify_order.sh` - Order modification tests

### Funding Tests
- `test_funding_e2e.sh` - End-to-end funding tests
- `test_funding_simple.sh` - Simple funding tests
- `test_funding_working.sh` - Working funding tests

### Crisis Tests
- `test_comprehensive_crisis.sh` - Comprehensive crisis tests
- `test_insurance_crisis.sh` - Insurance crisis tests

### Other Tests
- `test_core_scenarios.sh` - Core scenario tests
- `test_halt_resume.sh` - Halt/resume tests
- `test_kitchen_sink.sh` - Kitchen sink tests

## Usage

Run scripts from the repository root:

```bash
# Run a specific test
./test-scripts/test_router_lp_slab.sh

# Run all tests
./test-scripts/run_all_tests.sh

# Or from within the folder
cd test-scripts
./test_router_lp_slab.sh  # Note: when running from within folder, use ./
```

## Adding New Test Scripts

When Toly (upstream) adds new test scripts, place them in this folder to keep organized.

## Note

Some test scripts may reference paths relative to the repository root. If you encounter issues, run scripts from the repository root directory.

