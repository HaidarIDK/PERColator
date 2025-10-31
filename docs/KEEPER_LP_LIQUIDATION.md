# Keeper Operations Guide: LP Liquidation

## Overview

This guide describes how to operate a Percolator DEX keeper that monitors and executes LP liquidations. LP liquidation is the final safety mechanism for restoring portfolio health when principal liquidation is insufficient.

**Target Audience**: Keeper operators, liquidation bot developers, infrastructure teams

**Prerequisites**:
- Understanding of Percolator DEX architecture
- Familiarity with Solana RPC and transaction submission
- Read `docs/LP_LIQUIDATION_ARCHITECTURE.md` for background

## Keeper Responsibilities

A keeper monitoring LP liquidations must:

1. **Monitor Portfolio Health** - Continuously track portfolio equity vs maintenance margin
2. **Detect LP Liquidation Opportunities** - Identify underwater portfolios with LP positions
3. **Check Price Freshness** - Verify AMM prices are not stale before liquidating
4. **Submit Liquidation Transactions** - Call `liquidate_user` instruction for eligible portfolios
5. **Monitor Execution** - Track liquidation results and handle failures
6. **Alert on Anomalies** - Flag stale prices, bad debt, or unusual liquidations

## Architecture Context

### Three-Tier Liquidation Priority

The router automatically handles liquidation priority:

1. **Principal Positions** (Priority 1) - Liquidated first
2. **Slab LP Positions** (Priority 2) - Liquidated if principal insufficient
3. **AMM LP Positions** (Priority 3) - Liquidated as last resort

**Keeper Action**: Submit one `liquidate_user` transaction - the router handles the priority logic.

### Keeper Role vs Router Role

**Keeper Responsibilities**:
- Monitoring portfolio health
- Detecting liquidation opportunities
- Submitting transactions
- Monitoring for stale prices

**Router Responsibilities**:
- Executing liquidation logic
- Priority ordering (principal → Slab LP → AMM LP)
- Staleness guards
- Margin recalculation
- Bad debt handling

## Detection Functions

Location: `keeper/src/health.rs`

### 1. Checking if LP Liquidation Needed

```rust
use keeper::health::{needs_lp_liquidation, Portfolio};

// After fetching portfolio account
let portfolio = parse_portfolio(&account_data)?;

if needs_lp_liquidation(&portfolio) {
    println!("Portfolio {} may need LP liquidation", portfolio_pubkey);
    // Proceed to prepare liquidation transaction
}
```

**Logic**:
- Returns `true` if portfolio has LP positions AND equity < maintenance margin
- Simple check - does not simulate principal liquidation proceeds

### 2. Getting LP Liquidation Priority

```rust
use keeper::health::get_lp_liquidation_priority;

let (slab_buckets, amm_buckets) = get_lp_liquidation_priority(&portfolio);

println!("Portfolio has:");
println!("  - {} Slab LP bucket(s)", slab_buckets.len());
println!("  - {} AMM LP bucket(s)", amm_buckets.len());

// Useful for monitoring and alerting
for bucket in slab_buckets {
    println!("  - Slab LP: venue_id={:?}, mm={}", bucket.venue_id, bucket.mm);
}

for bucket in amm_buckets {
    println!("  - AMM LP: venue_id={:?}, mm={}", bucket.venue_id, bucket.mm);
}
```

**Use Cases**:
- Monitoring dashboards
- Alerting on high-value LP liquidations
- Estimating liquidation proceeds

### 3. Checking AMM Price Staleness

```rust
use keeper::health::is_amm_price_stale;
use solana_sdk::clock::Clock;

// Get current timestamp
let clock = rpc_client.get_account(&clock::ID)?;
let clock = Clock::from_account_info(&clock)?;
let current_ts = clock.unix_timestamp as u64;

// Check staleness (max_staleness typically 60 seconds)
let max_staleness_secs = 60;

for bucket in amm_buckets {
    if is_amm_price_stale(bucket, current_ts, max_staleness_secs) {
        println!("WARNING: AMM LP price stale for venue {:?}", bucket.venue_id);
        println!("  - Consider triggering AMM price update");
        println!("  - Liquidation will skip this bucket");
    }
}
```

**Alert Triggers**:
- AMM price stale AND portfolio underwater → Critical alert
- Suggests triggering AMM price update before liquidation

## Monitoring Workflow

### Step 1: Fetch Portfolio Accounts

```rust
use solana_client::rpc_client::RpcClient;
use keeper::health::parse_portfolio;

let rpc_client = RpcClient::new("https://api.mainnet-beta.solana.com");

// Get all portfolio accounts (implementation-specific)
let portfolio_pubkeys = get_all_portfolios(&rpc_client)?;

for portfolio_pubkey in portfolio_pubkeys {
    let account = rpc_client.get_account(&portfolio_pubkey)?;
    let portfolio = parse_portfolio(&account.data)?;

    // Process portfolio (see Step 2)
}
```

**Optimization**:
- Use `getMultipleAccounts` RPC to fetch in batches
- Filter to only underwater portfolios (`equity < mm`)
- Use WebSocket subscriptions for real-time updates

### Step 2: Check Health and LP Positions

```rust
use keeper::health::{calculate_health, needs_lp_liquidation};
use std::collections::HashMap;

// Calculate health (equity - MM)
let oracle_prices = fetch_oracle_prices(&rpc_client)?;
let health = calculate_health(&portfolio, &oracle_prices);

if health < 0 {
    println!("Portfolio {} is underwater (health: {})", portfolio_pubkey, health);

    // Check if has LP positions
    if needs_lp_liquidation(&portfolio) {
        println!("  - Has LP positions, may need LP liquidation");

        // Proceed to Step 3
        prepare_and_submit_liquidation(
            &rpc_client,
            &portfolio_pubkey,
            &portfolio,
        )?;
    } else {
        println!("  - No LP positions, standard liquidation only");
        // Submit standard liquidation
    }
}
```

### Step 3: Check AMM Price Staleness (Critical)

```rust
use keeper::health::{get_lp_liquidation_priority, is_amm_price_stale};

let (slab_buckets, amm_buckets) = get_lp_liquidation_priority(&portfolio);
let current_ts = get_current_timestamp(&rpc_client)?;
let max_staleness_secs = 60; // From registry

let mut has_stale_amm = false;

for bucket in &amm_buckets {
    if is_amm_price_stale(bucket, current_ts, max_staleness_secs) {
        println!("WARNING: Stale AMM price for venue {:?}", bucket.venue_id);
        has_stale_amm = true;
    }
}

if has_stale_amm {
    // OPTION 1: Trigger AMM price update first
    println!("Triggering AMM price update before liquidation...");
    trigger_amm_price_update(&rpc_client, &amm_buckets)?;

    // Wait and retry
    std::thread::sleep(std::time::Duration::from_secs(5));

    // OPTION 2: Proceed anyway (router will skip stale buckets)
    println!("Proceeding with liquidation (stale buckets will be skipped)");
}
```

**Best Practice**: Always trigger AMM price updates before liquidating portfolios with AMM LP.

### Step 4: Submit Liquidation Transaction

```rust
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    transaction::Transaction,
};

// Prepare liquidate_user instruction (disc 0)
let liquidate_ix = Instruction {
    program_id: router_program_id,
    accounts: vec![
        AccountMeta::new(portfolio_pubkey, false),
        AccountMeta::new_readonly(registry_pubkey, false),
        AccountMeta::new(liquidator_portfolio_pubkey, false),
        AccountMeta::new_readonly(liquidator_authority, true),
        // ... other accounts (insurance fund, matchers, etc.)
    ],
    data: vec![0], // Discriminator 0 = liquidate_user
};

// Sign and submit
let mut transaction = Transaction::new_with_payer(
    &[liquidate_ix],
    Some(&liquidator_authority),
);

transaction.sign(&[&liquidator_keypair], recent_blockhash);

let signature = rpc_client.send_and_confirm_transaction(&transaction)?;
println!("Liquidation submitted: {}", signature);
```

**Note**: The router automatically handles the three-tier priority. No special account preparation needed for LP liquidation.

### Step 5: Monitor Execution

```rust
use solana_client::rpc_config::RpcTransactionConfig;

// Wait for confirmation
let config = RpcTransactionConfig {
    encoding: Some(solana_sdk::commitment_config::UiTransactionEncoding::Json),
    commitment: Some(solana_sdk::commitment_config::CommitmentConfig::confirmed()),
    max_supported_transaction_version: Some(0),
};

let tx_result = rpc_client.get_transaction_with_config(&signature, config)?;

// Parse logs to check what was liquidated
for log in tx_result.transaction.meta.unwrap().log_messages {
    if log.contains("LP Liquidation") {
        println!("LP liquidation log: {}", log);
    }
}
```

**Key Log Messages**:
- `"LP Liquidation: Principal insufficient, starting LP liquidation"` - LP liquidation triggered
- `"LP Liquidation: Slab LP freed collateral"` - Slab LP liquidated
- `"LP Liquidation: Still underwater, starting AMM LP liquidation"` - AMM LP needed
- `"LP Liquidation: AMM LP freed collateral"` - AMM LP liquidated
- `"Warning: AMM price stale, skipping bucket"` - Staleness guard triggered
- `"LP Liquidation: Portfolio restored to health"` - Success
- `"LP Liquidation: Warning - still underwater"` - Bad debt scenario

## Account Preparation

### Required Accounts for `liquidate_user`

The `liquidate_user` instruction requires:

1. **Portfolio** (mut) - The underwater portfolio
2. **Registry** (immut) - Slab registry for params
3. **Liquidator Portfolio** (mut) - Liquidator's portfolio (receives profit)
4. **Liquidator Authority** (signer) - Liquidator's keypair
5. **Insurance Fund** (mut) - For bad debt
6. **Clock** (sysvar) - For timestamps

**For LP Liquidation** (additional accounts may be needed):
- Matcher accounts for each LP venue
- Token accounts for reserves (Slab LP)
- AMM accounts for share price (AMM LP)

**Note**: Account preparation is the same as standard liquidation. The router uses existing portfolio LP bucket data.

### PDA Derivation

Liquidator portfolio PDA:
```rust
let (liquidator_portfolio_pda, _bump) = Pubkey::find_program_address(
    &[
        b"portfolio",
        &liquidator_authority.to_bytes(),
        &registry_pubkey.to_bytes(),
    ],
    &router_program_id,
);
```

## Profitability Calculation

### Estimating LP Liquidation Proceeds

```rust
use keeper::health::{get_lp_liquidation_priority, LpBucketType};

fn estimate_lp_liquidation_value(portfolio: &Portfolio) -> i128 {
    let (slab_buckets, amm_buckets) = get_lp_liquidation_priority(portfolio);

    let mut total_value = 0i128;

    // Slab LP value (deterministic)
    for bucket in slab_buckets {
        if let LpBucketType::Slab { reserved_base, reserved_quote, .. } = bucket.bucket_type {
            let slab_value = (reserved_base + reserved_quote) as i128 / 1_000_000;
            total_value += slab_value;
        }
    }

    // AMM LP value (use cached price, but note staleness risk)
    for bucket in amm_buckets {
        if let LpBucketType::Amm { lp_shares, share_price_cached, .. } = bucket.bucket_type {
            let amm_value = (lp_shares as i128 * share_price_cached as i128) / 1_000_000;
            total_value += amm_value;
        }
    }

    total_value
}
```

**Warning**: AMM LP value estimates may be inaccurate if prices are stale.

### Liquidation Profit

```rust
// Liquidation profit = (MM - Equity) * liquidation_fee_bps / 10000
let deficit = portfolio.mm as i128 - portfolio.equity;
let liquidation_fee_bps = 50; // 0.5%
let profit = (deficit * liquidation_fee_bps) / 10_000;

println!("Estimated liquidation profit: ${}", profit / 1_000_000);
```

**Note**: Profit is only on the liquidated amount, not the full LP position value.

## Monitoring and Alerting

### Key Metrics to Track

1. **Underwater Portfolios with LP** - Count and total value
2. **Stale AMM Prices** - Count and age distribution
3. **LP Liquidations Executed** - Count by type (Slab vs AMM)
4. **Bad Debt from LP Liquidations** - Total USD value
5. **Liquidation Latency** - Time from underwater to liquidation

### Alert Thresholds

**Critical Alerts**:
- Portfolio underwater with AMM LP + stale price > 5 minutes
- Bad debt from LP liquidation > $10,000
- LP liquidation failed (transaction error)

**Warning Alerts**:
- Portfolio underwater with LP for > 60 seconds
- AMM price stale (but not yet liquidating)
- High LP liquidation frequency (potential systemic issue)

### Example Monitoring Script

```rust
use std::time::Duration;
use tokio::time::interval;

#[tokio::main]
async fn main() -> Result<()> {
    let rpc_client = RpcClient::new("https://api.mainnet-beta.solana.com");
    let mut interval = interval(Duration::from_secs(5)); // Check every 5 seconds

    loop {
        interval.tick().await;

        // Fetch all portfolios
        let portfolios = fetch_all_portfolios(&rpc_client).await?;

        for (pubkey, portfolio) in portfolios {
            let health = calculate_health(&portfolio, &get_oracle_prices()?);

            if health < 0 && needs_lp_liquidation(&portfolio) {
                println!("ALERT: Portfolio {} underwater with LP", pubkey);

                // Check for stale AMM prices
                let (_, amm_buckets) = get_lp_liquidation_priority(&portfolio);
                let current_ts = get_current_timestamp(&rpc_client)?;

                for bucket in amm_buckets {
                    if is_amm_price_stale(bucket, current_ts, 60) {
                        println!("CRITICAL: Stale AMM price for {}", pubkey);
                        send_alert("Stale AMM price + underwater portfolio")?;
                    }
                }

                // Attempt liquidation
                execute_liquidation(&rpc_client, &pubkey, &portfolio).await?;
            }
        }
    }
}
```

## Troubleshooting

### Issue 1: Liquidation Transaction Fails

**Symptoms**: Transaction returns error, portfolio still underwater

**Possible Causes**:
1. Insufficient liquidator portfolio margin
2. Missing accounts (matcher, insurance fund)
3. Portfolio not actually underwater (race condition)
4. Rate limit reached

**Debugging**:
```bash
# Check transaction logs
solana confirm -v <SIGNATURE>

# Look for specific errors:
# - "PortfolioHealthy" (107) → Portfolio recovered before tx
# - "LiquidationCooldown" (111) → Rate limited
# - "InsufficientMargin" (400) → Liquidator lacks margin
```

**Solution**:
- Retry transaction
- Ensure liquidator portfolio has sufficient margin
- Check rate limit status
- Verify all required accounts included

### Issue 2: AMM LP Not Liquidated (Stale Price)

**Symptoms**: Portfolio still underwater, logs show "Warning: AMM price stale, skipping bucket"

**Cause**: AMM share price hasn't been updated recently

**Solution**:
```rust
// Trigger AMM price update before liquidation
let amm_pubkey = derive_amm_pubkey(&venue_id);

let update_price_ix = Instruction {
    program_id: amm_program_id,
    accounts: vec![
        AccountMeta::new(amm_pubkey, false),
        AccountMeta::new_readonly(oracle_pubkey, false),
        AccountMeta::new_readonly(clock::ID, false),
    ],
    data: vec![1], // Discriminator for update_price
};

rpc_client.send_and_confirm_transaction(&Transaction::new_with_payer(
    &[update_price_ix],
    Some(&keeper_authority),
))?;

// Now retry liquidation
```

### Issue 3: Portfolio Enters Bad Debt

**Symptoms**: Liquidation completes but portfolio still underwater

**Cause**: LP liquidation proceeds insufficient to cover deficit

**Action**:
- Verify insurance fund has sufficient balance
- Monitor for loss socialization events
- Alert operations team
- Review liquidation delay (why wasn't liquidated earlier?)

**Log Evidence**:
```
"LP Liquidation: Warning - still underwater"
"Bad debt recorded: 5000000000" (recorded in subsequent logs)
```

### Issue 4: Slab LP Not Liquidated (Open Orders)

**Symptoms**: Slab LP bucket has reserves but not liquidated

**Possible Cause**: Bucket may have locked reserves in open orders

**Investigation**:
```rust
// Check Slab LP bucket state
if let LpBucketType::Slab { reserved_base, reserved_quote, open_order_count } = bucket.bucket_type {
    println!("Slab LP: base={}, quote={}, orders={}",
        reserved_base, reserved_quote, open_order_count);
}
```

**Note**: Current implementation liquidates entire bucket (including locked reserves). If this is the issue, it's likely a bug - report to dev team.

## Best Practices

### 1. Monitor AMM Price Freshness Proactively

Don't wait for portfolios to go underwater:

```rust
// Separate monitoring loop for AMM prices
loop {
    let amm_accounts = fetch_all_amm_accounts(&rpc_client).await?;
    let current_ts = get_current_timestamp(&rpc_client)?;

    for (pubkey, amm) in amm_accounts {
        let age = current_ts - amm.last_update_ts;
        if age > 45 { // Warn at 45s (before 60s staleness limit)
            println!("WARNING: AMM {} price aging ({}s)", pubkey, age);
            trigger_price_update(&rpc_client, &pubkey).await?;
        }
    }

    tokio::time::sleep(Duration::from_secs(30)).await;
}
```

### 2. Batch Account Fetches

Use `getMultipleAccounts` RPC method:

```rust
use solana_client::rpc_request::RpcRequest;

let portfolio_pubkeys: Vec<Pubkey> = /* ... */;

// Batch fetch (up to 100 accounts per call)
let accounts = rpc_client.get_multiple_accounts(&portfolio_pubkeys)?;

for (pubkey, account) in portfolio_pubkeys.iter().zip(accounts.iter()) {
    if let Some(account) = account {
        let portfolio = parse_portfolio(&account.data)?;
        // Process portfolio
    }
}
```

### 3. Use WebSocket Subscriptions for Real-Time Monitoring

```rust
use solana_client::pubsub_client::PubsubClient;

let (mut account_subscription, _unsubscribe) = PubsubClient::account_subscribe(
    "wss://api.mainnet-beta.solana.com",
    &portfolio_pubkey,
    None,
)?;

loop {
    match account_subscription.recv() {
        Ok(response) => {
            let portfolio = parse_portfolio(&response.value.data)?;

            if needs_lp_liquidation(&portfolio) {
                println!("Real-time alert: Portfolio underwater with LP");
                execute_liquidation_async(&portfolio_pubkey, &portfolio).await?;
            }
        }
        Err(e) => eprintln!("WebSocket error: {}", e),
    }
}
```

### 4. Implement Exponential Backoff for Failed Transactions

```rust
use std::time::Duration;

let mut retry_count = 0;
let max_retries = 3;

loop {
    match execute_liquidation(&rpc_client, &portfolio_pubkey, &portfolio) {
        Ok(signature) => {
            println!("Liquidation successful: {}", signature);
            break;
        }
        Err(e) if retry_count < max_retries => {
            retry_count += 1;
            let backoff = Duration::from_millis(100 * 2u64.pow(retry_count));
            println!("Liquidation failed (attempt {}), retrying in {:?}", retry_count, backoff);
            std::thread::sleep(backoff);
        }
        Err(e) => {
            println!("Liquidation failed after {} attempts: {}", retry_count, e);
            send_alert(&format!("Liquidation failed for {}", portfolio_pubkey))?;
            break;
        }
    }
}
```

### 5. Log All LP Liquidation Events

```rust
use chrono::Utc;

struct LpLiquidationEvent {
    timestamp: i64,
    portfolio: Pubkey,
    equity_before: i128,
    mm_before: u128,
    slab_buckets_liquidated: usize,
    amm_buckets_liquidated: usize,
    total_value_freed: i128,
    equity_after: i128,
    signature: String,
}

fn log_liquidation_event(event: &LpLiquidationEvent) {
    // Write to database, CloudWatch, Datadog, etc.
    println!("{}", serde_json::to_string(event).unwrap());
}
```

## Performance Optimization

### Reduce RPC Calls

**Before**:
```rust
for portfolio_pubkey in all_portfolios {
    let account = rpc_client.get_account(&portfolio_pubkey)?; // N RPC calls
    // ...
}
```

**After**:
```rust
// Batch into groups of 100
for chunk in all_portfolios.chunks(100) {
    let accounts = rpc_client.get_multiple_accounts(chunk)?; // N/100 RPC calls
    // ...
}
```

### Cache Oracle Prices

```rust
use std::time::{Instant, Duration};

struct PriceCache {
    prices: HashMap<u16, i64>,
    last_update: Instant,
    ttl: Duration,
}

impl PriceCache {
    fn get_prices(&mut self, rpc_client: &RpcClient) -> Result<&HashMap<u16, i64>> {
        if self.last_update.elapsed() > self.ttl {
            self.prices = fetch_oracle_prices(rpc_client)?;
            self.last_update = Instant::now();
        }
        Ok(&self.prices)
    }
}
```

### Parallel Processing

```rust
use rayon::prelude::*;

let liquidation_opportunities: Vec<_> = portfolios
    .par_iter()
    .filter_map(|(pubkey, portfolio)| {
        let health = calculate_health(portfolio, &oracle_prices);
        if health < 0 && needs_lp_liquidation(portfolio) {
            Some((pubkey, portfolio))
        } else {
            None
        }
    })
    .collect();

// Execute liquidations sequentially (to avoid nonce conflicts)
for (pubkey, portfolio) in liquidation_opportunities {
    execute_liquidation(&rpc_client, pubkey, portfolio)?;
}
```

## Security Considerations

### 1. Protect Liquidator Private Keys

- Use hardware wallets or secure enclaves
- Rotate keys periodically
- Limit liquidator portfolio exposure
- Monitor for unauthorized transactions

### 2. Validate AMM Price Updates

Before trusting an AMM price:

```rust
// Check oracle price against AMM share price
let oracle_price = fetch_oracle_price(&rpc_client, &amm.oracle)?;
let share_price = amm.share_price_cached;

let deviation = ((share_price - oracle_price).abs() as f64 / oracle_price as f64) * 100.0;

if deviation > 5.0 { // 5% deviation threshold
    println!("WARNING: AMM price deviates {}% from oracle", deviation);
    send_alert("Suspicious AMM price")?;
}
```

### 3. Rate Limiting Protection

The protocol enforces rate limits per portfolio. Respect them:

```rust
// Track last liquidation timestamp per portfolio
let mut last_liquidation: HashMap<Pubkey, u64> = HashMap::new();

// Before liquidating
if let Some(&last_ts) = last_liquidation.get(&portfolio_pubkey) {
    let cooldown = 60; // seconds
    if current_ts - last_ts < cooldown {
        println!("Skipping {}: cooldown active", portfolio_pubkey);
        continue;
    }
}

// After successful liquidation
last_liquidation.insert(portfolio_pubkey, current_ts);
```

## Testing

### Devnet Testing

```bash
# Set up devnet environment
export SOLANA_RPC_URL=https://api.devnet.solana.com
export ROUTER_PROGRAM_ID=<devnet_program_id>

# Run keeper with devnet config
cargo run --bin keeper -- \
    --rpc-url $SOLANA_RPC_URL \
    --program-id $ROUTER_PROGRAM_ID \
    --keypair ~/.config/solana/devnet.json
```

### Simulation Mode

```rust
// Dry-run mode: detect but don't execute
let dry_run = true;

if needs_lp_liquidation(&portfolio) {
    if dry_run {
        println!("[DRY RUN] Would liquidate portfolio {}", portfolio_pubkey);
        println!("[DRY RUN] Estimated value: {}", estimate_lp_liquidation_value(&portfolio));
    } else {
        execute_liquidation(&rpc_client, &portfolio_pubkey, &portfolio)?;
    }
}
```

## References

- **Architecture**: `docs/LP_LIQUIDATION_ARCHITECTURE.md`
- **Keeper Code**: `keeper/src/health.rs`
- **Router Implementation**: `programs/router/src/instructions/liquidate_user.rs`
- **Test Scripts**: `test_router_lp_slab.sh`, `test_router_lp_amm.sh`

## Support

For issues or questions:
- GitHub Issues: `https://github.com/percolator-dex/issues`
- Discord: `#keeper-operations` channel
- Email: `ops@percolator.xyz`

## Summary

Operating a keeper for LP liquidations requires:

1. **Continuous monitoring** of portfolio health and LP positions
2. **Proactive price management** for AMM venues
3. **Fast execution** when liquidation opportunities arise
4. **Robust error handling** for failed transactions
5. **Comprehensive logging** for post-mortem analysis

The detection functions in `keeper/src/health.rs` provide the building blocks. The router handles the complex liquidation logic automatically. Your job is to monitor, detect, and submit transactions efficiently and reliably.
