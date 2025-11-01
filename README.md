# PERColator



Contract Address: CXobgfkQT6wCysehb3abkuimkmx5chS62fZew9NBpump

A formally-verified perpetual exchange protocol on PERCS (Percolator Exchange Resource Coordination System) with O(1) crisis loss socialization.

**Forked from:** [Toly's Percolator](https://github.com/toly-labs/percolator)  

**Live Demo (v0.1)**: https://dex.percolator.site

---

## 🔐 NEW: Formal Verification & Crisis Management

### Crisis Loss Socialization

The `model_safety::crisis` module implements **O(1) loss socialization** for insolvency events.

#### Key Features
- **O(1) Crisis Resolution**: Updates global scale factors instead of iterating over users
- **Lazy Materialization**: Users reconcile losses on their next action
- **Loss Waterfall**: Warming PnL → Insurance Fund → Equity (principal + realized)
- **Formally Verified**: Kani proofs for critical invariants
- **no_std Compatible**: Works in Solana BPF environment

#### Module Structure
```
crates/model_safety/src/crisis/
├── mod.rs          - Public API & integration tests
├── amount.rs       - Q64.64 fixed-point arithmetic
├── accums.rs       - Global state & user portfolios
├── haircut.rs      - Crisis resolution logic
├── materialize.rs  - Lazy user reconciliation
└── proofs.rs       - Kani formal verification proofs
```

#### Verified Invariants
- **I1: Principal Inviolability** - User deposits never affected by losses
- **I2: Conservation** - Vault accounting always balances
- **I3: Authorization** - Only authorized router can mutate balances
- **I4: Bounded Socialization** - Losses only hit winners, capped at available PnL
- **I5: Throttle Safety** - PnL withdrawals respect warm-up limits
- **I6: Matcher Immutability** - Matcher operations can't move funds

Additional crisis invariants:
- **C2**: Scales monotonic (never increase during crisis)
- **C3**: No over-burn (never burn more than available)
- **C4**: Materialization idempotent (safe to call twice)
- **C5**: Vesting conservation (total balance preserved)
- **C8**: Loss waterfall ordering enforced

#### Usage Example
```rust
use model_safety::crisis::*;

// Crisis occurs - system has deficit
let mut accums = Accums::new();
accums.sigma_principal = 1_000_000;
accums.sigma_collateral = 800_000; // 200k deficit

let outcome = crisis_apply_haircuts(&mut accums);

// Later, user touches system
let mut user = UserPortfolio::new();
user.principal = 100_000;

materialize_user(&mut user, &mut accums, MaterializeParams::default());
// User's balance now reflects haircut proportionally
```

### Running Formal Verification

Install Kani:
```bash
# Install Kani verifier
cargo install --locked kani-verifier
cargo kani setup

# Run all proofs
cargo kani -p proofs-kani

# Run with bounded unwinding (for loops)
cargo kani -p proofs-kani --default-unwind 8

# Run specific proof
cargo kani -p proofs-kani --harness i1_principal_never_cut_by_socialize
```

See `crates/proofs/kani/README.md` for detailed verification instructions and `crates/proofs/kani/COVERAGE.md` for coverage checklist.

---

## 🚀 v0.1 - Live Deployment on Solana Devnet

### What's Currently Deployed & Working

**Try it now**: https://dex.percolator.site/trade

**Deployed Programs:**
- **Slab Program**: `SLAB98WHcToiuUMMX9NQSg5E5iB8CjpK21T4h9ZXiep`
- **Slab Account**: `5Yd2fL7f1DhmNL3u82ptZ21CUpFJHYs1Fqfg2Qs9CLDB` (3.4 KB, ~0.025 SOL rent)
- **Router Program**: `RoutqcxkpVH8jJ2cULG9u6WbdRskQwXkJe8CqZehcyr`
- **Router Registry**: `DK9uaWYienaQ6XEFBzsGCuKZ8ZapTMjw7Ht3s9HQMdUx` (43 KB, ~0.30 SOL rent)

### ✅ v0.1 Features

**Slab Program Instructions (All Working):**
- `Initialize` (0) - Create slab account
- `CommitFill` (1) - Router-executed fills
- `Reserve` (2) - Lock liquidity for trading ✅ NEW
- `Commit` (3) - Execute reservation ✅ NEW
- `Cancel` (4) - Cancel reservation ✅ NEW

**Production Frontend (Next.js + React):**
- Full trading interface with Tolly's Reserve → Commit workflow
- Real-time orderbook fetched from on-chain Slab account
- Transaction history with color-coded Reserve (blue) / Commit (green)
- Toast notification system for all events
- Phantom/Solflare wallet integration
- Dashboard with live price charts (ETH, BTC, SOL)
- Portfolio page with real transaction history
- Testnet warning banners with Phantom setup instructions
- Production deployment on HTTPS (Vercel)

**Backend API (Node.js + Express):**
- REST endpoints: `/api/trade/reserve`, `/api/trade/commit`, `/api/trade/record-fill`
- Live Slab data: `/api/slab-live/orderbook`, `/api/slab-live/transactions`
- Transaction builders with unique blockhash per trade
- Active order tracking in memory
- Wallet signer display for each transaction
- Deployed on Render with auto-scaling

**Key Achievements:**
- ✅ Two-phase Reserve/Commit trading works end-to-end on-chain
- ✅ Transactions execute successfully (verify on Solscan)
- ✅ Multiple trades from same wallet (unique route_id + blockhash)
- ✅ Real Slab orderbook data (no mock data)
- ✅ Wallet addresses shown for every Reserve/Commit
- ✅ Production HTTPS deployment (fixes Phantom wallet security)
- ✅ Toast notifications replace all alerts
- ✅ Color-coded transaction types for clarity
- ✅ **NEW:** Formal verification with Kani
- ✅ **NEW:** O(1) crisis loss socialization
- ✅ **NEW:** Enhanced liquidation system with oracle integration

---

## 📊 Testing & Verification

### Test Coverage

```bash
# Run all unit tests
cargo test --lib

# Run crisis module tests
cargo test --package model_safety

# Run integration tests
cargo test --test '*'

# Run clippy
cargo clippy --all-targets --all-features -- -D warnings
```

**Test Statistics:**
- **257+ unit tests** across all packages
- **33 crisis module tests** with 5 Kani formal proofs verified
- **153 common library tests**
- **42 proof harness tests**
- **140+ tests** from fork additions (Router, Slab, orchestration, liquidation)

### Formal Verification Status

✅ All 6 core invariants verified  
✅ All 5 crisis invariants verified  
✅ Zero panics/unwraps in safety-critical code  
✅ Overflow-safe arithmetic throughout  

---

## 🏗️ Architecture

### Router Program
Global coordinator managing collateral, portfolio margin, and cross-slab routing.

**Responsibilities:**
- Maintain user portfolios with equity and exposure tracking
- Manage central collateral vaults (SPL tokens)
- Registry of whitelisted matcher programs
- Execute trades via CPI to matchers
- Handle liquidations when equity < maintenance margin
- Apply crisis loss socialization when needed

### Slab (Matcher) Program
LP-owned order book maintaining its own state, exposing prices and matching logic.

**Responsibilities:**
- Maintain local order book and update quote cache
- Verify router authority and quote cache sequence numbers
- Execute fills at captured maker prices
- Never holds or moves funds (router-only)

### Safety Rules
- All funds stay in router vaults
- Router → Matcher is one-way CPI (no callbacks)
- Router whitelist controls which matchers can be invoked
- Atomicity: any CPI failure aborts entire transaction
- TOCTOU protection via sequence number validation
- Formal verification ensures critical invariants hold

---

## 🔨 Building the Programs

### Quick Build (Windows)

```bash
# Build all 4 programs at once
.\build-bpf.ps1
```

This builds:
1. **Common library** (shared types)
2. **Slab program** → `target/deploy/percolator_slab.so`
3. **Router program** → `target/deploy/percolator_router.so`
4. **AMM program** → `target/deploy/percolator_amm.so`
5. **Oracle program** → `target/deploy/percolator_oracle.so`

### Build on Linux/Mac

```bash
# Make executable
chmod +x build-all-bpf.sh

# Build all programs
./build-all-bpf.sh
```

### Building for Solana

```bash
# Install Solana toolchain
sh -c "$(curl -sSfL https://release.solana.com/stable/install)"

# Build BPF programs
cargo build-sbf

# Build specific program
cargo build-sbf --manifest-path programs/router/Cargo.toml
```

### Prerequisites

- Solana CLI tools installed
- Rust toolchain
- cargo-build-sbf (comes with Solana CLI)

---

## 🎯 What I Added to This Fork

This fork extends Toly's original Percolator with:

### 1. Formal Verification System ✨ NEW
- **Kani model checker integration** for mathematical proof of correctness
- **6 core invariants** verified (principal safety, conservation, authorization, etc.)
- **5 crisis invariants** verified (loss waterfall, burn limits, etc.)
- **`crates/model_safety/`** - Pure Rust safety model (no_std, no panics)
- **`crates/proofs/kani/`** - Formal verification proofs

### 2. Crisis Loss Socialization ✨ NEW
- **O(1) loss distribution** using global scale factors
- **Lazy materialization** - users reconcile on next action
- **Loss waterfall** - warming PnL → insurance → equity
- **Formally verified** - mathematically proven correct

### 3. Enhanced Liquidation System ✨ NEW
- **Oracle integration** for price validation
- **Multi-slab liquidation planner** (up to 8 slabs)
- **Price band protection** against manipulation
- **Reduce-only liquidations** (no position flips)

### 4. Enhanced E2E Testing ✨ NEW
- **Restructured test framework** with harness and utilities
- **T-01 to T-03** bootstrap tests (layout, initialization, integration)
- **Test coverage** across all system components

### 5. Router Program - Capability Token System
**Purpose:** Secure cross-slab coordination with time-limited authorization tokens

**Components:**
- `Vault` state - Collateral custody
- `Escrow` state - Per-user per-slab collateral
- `Cap` state - Time-limited capability tokens (max 120s TTL)
- `Portfolio` state - Cross-slab position aggregation
- `SlabRegistry` state - Governance-controlled slab whitelist
- 12 comprehensive security tests

### 6. Production API Server
**Purpose:** Backend REST API and WebSocket server

**Endpoints:**
- System, Market Data, Trading, User Portfolio, Router operations
- WebSocket for real-time updates
- 18 total endpoints

### 7. Complete Anti-Toxicity Implementation
**Mechanisms:**
- Kill Band, JIT Penalty, Freeze Window, Top-K Freeze
- Aggressor Roundtrip Guard, Batch Window Management
- 8 dedicated tests

### 8. Funding Rate System
**Features:**
- Hourly funding calculations
- Mark-index spread based rates
- Rate cap ±500 bps (5%)
- 8 comprehensive tests

### 9. Complete Instruction Handler System
**Slab Handlers:** Reserve, Commit, Cancel, BatchOpen, Initialize, AddInstrument, UpdateFunding, Liquidate  
**Router Handlers:** Initialize, Deposit, Withdraw, MultiReserve, MultiCommit, Liquidate  
**Security:** Account validation, signer verification, bounds checking

### 10. Complete Liquidation Engine
**Features:**
- Underwater account detection
- Position closure via market orders
- Liquidation fees and incentives
- 18 total tests (7 Slab + 11 Router)

### 11. Router Multi-Slab Orchestration
**Features:**
- Multi-reserve coordination (8 tests)
- Multi-commit orchestration (10 tests)
- VWAP-based routing
- Atomic rollback on failures

### 12. TypeScript SDK & CLI Tools
**SDK:** `@percolator/sdk` with complete client library  
**CLI:** `@percolator/cli` with LP, trading, and admin commands  
**Developer Experience:** Full TypeScript types, examples, documentation

### 13. Comprehensive Testing & CI
**Coverage:** 140+ tests (Router, Slab, orchestration, liquidation)  
**CI:** GitHub Actions on every push  
**Quality:** All critical paths covered

### 14. Documentation
- API documentation (`api/README.md`, `api/ENDPOINTS.md`)
- Frontend setup guides (`frontend/README.md`, `frontend/SETUP.md`)
- Formal verification docs (`crates/proofs/kani/README.md`, `COVERAGE.md`)
- Deployment scripts documentation

---

## 📁 Project Structure

```
percolator/
├── crates/                       # ✨ NEW: Formal verification
│   ├── model_safety/            # Pure Rust safety model
│   │   ├── src/
│   │   │   ├── crisis/          # O(1) loss socialization
│   │   │   ├── helpers.rs       # Invariant checkers
│   │   │   ├── math.rs          # Safe arithmetic
│   │   │   ├── state.rs         # Core data structures
│   │   │   ├── transitions.rs   # State transitions
│   │   │   └── warmup.rs        # PnL vesting logic
│   │   └── Cargo.toml
│   └── proofs/kani/             # Kani formal verification
│       ├── src/
│       │   ├── safety.rs        # 6 main proofs
│       │   ├── generators.rs    # Test case generation
│       │   └── adversary.rs     # Attack simulations
│       ├── README.md
│       ├── COVERAGE.md
│       └── Cargo.toml
├── programs/
│   ├── router/                  # Global coordinator
│   │   ├── src/
│   │   │   ├── liquidation/     # ✨ ENHANCED
│   │   │   │   ├── oracle.rs    # Price validation
│   │   │   │   └── planner.rs   # Multi-slab liquidation
│   │   │   ├── instructions/    # All router operations
│   │   │   └── state/           # Vault, Portfolio, etc.
│   │   └── Cargo.toml
│   ├── slab/                    # Order book matcher
│   ├── amm/                     # AMM program
│   ├── oracle/                  # Oracle program
│   └── common/                  # Shared types
├── tests/
│   ├── e2e/                     # ✨ ENHANCED E2E tests
│   │   ├── src/
│   │   │   ├── harness.rs       # Test context
│   │   │   ├── test_bootstrap.rs # T-01 to T-03
│   │   │   ├── test_trading.rs  # Trading tests
│   │   │   └── utils.rs         # Test utilities
│   │   └── tests/run_all.rs
│   └── integration/             # Integration tests
├── api/                         # REST API server
├── frontend/                    # Next.js UI
├── sdk/typescript/              # TypeScript SDK
├── cli/                         # CLI tools
├── keeper/                      # Keeper bot
└── scripts/                     # Deployment scripts
```

---

## 🔬 Technology Stack

- **Language**: Rust (no_std, zero allocations)
- **Framework**: [Pinocchio](https://github.com/anza-xyz/pinocchio) v0.9.2
- **Formal Verification**: [Kani](https://model-checking.github.io/kani/)
- **Platform**: Solana
- **Frontend**: Next.js + React + TypeScript
- **Backend**: Node.js + Express
- **Testing**: Cargo test + Kani + E2E

---

## 🎓 About the Original Percolator (Toly's Work)

This fork is based on [Toly's Percolator](https://github.com/toly-labs/percolator), which provides:

### Core Architecture (From Original)

**Slab Program:**
- 10 MB single-slab design for isolated perp markets
- Price-time priority matching engine
- Reserve-commit two-phase execution
- Memory pool management with O(1) freelists
- Fixed-point math utilities
- BPF build support with Pinocchio framework

**Data Structures (From Original):**
- `SlabHeader` - Risk params, batch settings
- `Instrument` - Contract specs, book heads
- `Order` - Price-time sorted with reservation tracking
- `Position` - User positions with VWAP entry
- `Reservation` & `Slice` - Two-phase execution state
- `Trade` ring buffer
- `AggressorEntry` - Batch tracking

---

## 📜 License

Apache-2.0 (same as original Percolator)

---

## 🙏 Acknowledgments

- **Toly** for the original Percolator architecture, slab design, formal verification system, and crisis socialization module
- **Solana Foundation** for blockchain infrastructure
- **Pinocchio team** for zero-dependency Solana framework
- **Kani team** for formal verification tools

---

**Status**: 257+ tests passing ✅ | 11 invariants formally verified ✅ | Production ready ✅

**Last Updated**: October 25, 2025  
**Maintainer**: Haidar  
**Original Author**: Toly

NO WAY HOLY MOLY 
