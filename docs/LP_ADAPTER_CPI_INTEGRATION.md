# LP Adapter CPI Integration Guide

This document explains how the Router program integrates with Matcher adapter programs via Cross-Program Invocation (CPI) for liquidity operations.

## Architecture Overview

```
┌─────────────────┐
│  CLI / Client   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐         CPI          ┌──────────────────┐
│  Router Program │ ──────────────────▶  │ Matcher Adapter  │
│                 │                       │  (Slab/AMM/etc)  │
│ - RouterReserve │                       │                  │
│ - RouterLiquidity│◀──────────────────   │ - process_liquidity│
│ - RouterRelease │   LiquidityResult    │                  │
└─────────────────┘                       └──────────────────┘
         │
         ▼
┌─────────────────┐
│  State Accounts │
│ - RouterLpSeat  │
│ - VenuePnl      │
│ - Portfolio     │
└─────────────────┘
```

## CPI Flow for RouterLiquidity Instruction

### 1. Client Invokes RouterLiquidity

```rust
// Client builds instruction
let instruction = Instruction {
    program_id: router_program_id,
    accounts: vec![
        AccountMeta::new(portfolio_pda, false),
        AccountMeta::new(lp_seat_pda, false),
        AccountMeta::new(venue_pnl_pda, false),
        AccountMeta::new_readonly(matcher_program_id, false),
        // ... additional accounts for CPI
    ],
    data: instruction_data, // discriminator + RiskGuard + LiquidityIntent
};
```

### 2. Router Receives and Validates

```rust
pub fn process_router_liquidity(
    portfolio_account: &AccountInfo,
    portfolio: &mut Portfolio,
    seat_account: &AccountInfo,
    seat: &mut RouterLpSeat,
    venue_pnl_account: &AccountInfo,
    venue_pnl: &mut VenuePnl,
    matcher_program: &AccountInfo,
    guard: RiskGuard,
    intent: LiquidityIntent,
) -> ProgramResult {
    // Validate ownership and state
    validate_seat_ownership(seat, portfolio_account)?;
    validate_seat_not_frozen(seat)?;
    validate_venue_pnl_matches(venue_pnl, seat)?;

    // Make CPI to matcher adapter
    let result = invoke_matcher_liquidity(
        matcher_program,
        seat_account,
        guard,
        intent,
        remaining_accounts, // Pass through for matcher-specific accounts
    )?;

    // Apply result to router state
    apply_liquidity_result(seat, venue_pnl, result)?;

    // Verify seat credit discipline
    verify_seat_limits(seat)?;

    Ok(())
}
```

### 3. Router Makes CPI to Matcher

```rust
fn invoke_matcher_liquidity(
    matcher_program: &AccountInfo,
    seat_account: &AccountInfo,
    guard: RiskGuard,
    intent: LiquidityIntent,
    remaining_accounts: &[AccountInfo],
) -> Result<LiquidityResult, ProgramError> {
    // Serialize CPI instruction data
    let mut data = Vec::new();
    data.push(MATCHER_LIQUIDITY_DISCRIMINATOR); // e.g., 0x01
    data.extend_from_slice(&guard.try_to_vec()?);
    data.extend_from_slice(&intent.try_to_vec()?);

    // Build account metas for CPI
    let account_metas = vec![
        AccountMeta::new(*seat_account.key, false),
        // ... matcher-specific accounts from remaining_accounts
    ];

    // Create CPI instruction
    let ix = Instruction {
        program_id: *matcher_program.key,
        accounts: account_metas,
        data,
    };

    // Invoke matcher program
    invoke(
        &ix,
        &[seat_account, matcher_program],
    )?;

    // Read result from return data
    let (program_id, return_data) = get_return_data()
        .ok_or(ProgramError::InvalidInstructionData)?;

    if program_id != *matcher_program.key {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Deserialize LiquidityResult
    LiquidityResult::try_from_slice(&return_data)
        .map_err(|_| ProgramError::InvalidAccountData)
}
```

### 4. Matcher Processes Liquidity Operation

```rust
// In matcher adapter program (e.g., programs/slab/src/adapter.rs)

pub fn process_liquidity(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    guard: RiskGuard,
    intent: LiquidityIntent,
) -> ProgramResult {
    // Deserialize matcher-specific state
    let matcher_state = load_matcher_state(accounts)?;

    // Execute liquidity operation based on intent
    let result = match intent {
        LiquidityIntent::AmmAdd { .. } => {
            process_amm_add(matcher_state, guard, intent)?
        }
        LiquidityIntent::ObAdd { orders, .. } => {
            process_ob_add(matcher_state, guard, orders)?
        }
        LiquidityIntent::Remove { selector } => {
            process_remove(matcher_state, selector)?
        }
        LiquidityIntent::Modify { .. } => {
            process_modify(matcher_state, guard, intent)?
        }
        LiquidityIntent::Hook { hook_id, payload } => {
            process_hook(matcher_state, hook_id, payload)?
        }
    };

    // Validate result against risk guard
    validate_result_against_guard(&result, &guard)?;

    // Return LiquidityResult via set_return_data
    let serialized = result.try_to_vec()?;
    set_return_data(&serialized);

    Ok(())
}
```

### 5. Router Applies Result

```rust
fn apply_liquidity_result(
    seat: &mut RouterLpSeat,
    venue_pnl: &mut VenuePnl,
    result: LiquidityResult,
) -> Result<(), ()> {
    // Update LP shares
    seat.lp_shares = apply_shares_delta(seat.lp_shares, result.lp_shares_delta)?;

    // Update exposure
    seat.exposure.base_q64 = seat.exposure.base_q64
        .checked_add(result.exposure_delta.base_q64)
        .ok_or(())?;

    seat.exposure.quote_q64 = seat.exposure.quote_q64
        .checked_add(result.exposure_delta.quote_q64)
        .ok_or(())?;

    // Update venue PnL
    venue_pnl.apply_deltas(
        result.maker_fee_credits,
        0, // venue_fees calculated separately
        result.realized_pnl_delta,
    )?;

    Ok(())
}
```

## Matcher Adapter Interface Requirements

Any matcher adapter program MUST implement:

1. **Liquidity Instruction Handler**
   - Discriminator: Matcher-specific (recommended 0x01 for liquidity ops)
   - Input: `RiskGuard` + `LiquidityIntent` (Borsh serialized)
   - Output: `LiquidityResult` (via `set_return_data`)

2. **State Validation**
   - Verify seat ownership/permissions
   - Validate operation against current matcher state
   - Enforce risk guards (slippage, fees, oracle bounds)

3. **Atomic Operations**
   - Execute liquidity changes atomically
   - Update matcher-specific state (AMM pools, orderbook, etc.)
   - Return accurate deltas in `LiquidityResult`

4. **Error Handling**
   - Return specific ProgramError codes
   - Fail transaction if risk guards violated
   - Prevent partial state updates

## Security Considerations

### 1. Custody Isolation
- Router NEVER passes writable vault accounts to matcher
- Matcher NEVER receives direct token transfer authority
- All fund movements go through Router's authority

### 2. State Validation
- Router validates seat ownership before CPI
- Router validates seat not frozen before CPI
- Router verifies venue_pnl matches seat's matcher

### 3. Credit Discipline
- Router checks seat credit limits AFTER applying result
- Reverting transaction if exposure exceeds reserved collateral
- Haircut applied to reserved amounts for safety margin

### 4. CPI Return Data
- Router validates return data program ID matches matcher
- Router deserializes LiquidityResult with error handling
- Router applies deltas with overflow checking

## Example: AMM Liquidity Add

```rust
// 1. Client calls RouterReserve
RouterReserve {
    base_amount_q64: 1_000_000 << 64,  // 1M base
    quote_amount_q64: 1_000_000 << 64, // 1M quote
}
// → Portfolio.free_collateral -= 2M
// → LpSeat.reserved_base_q64 += 1M
// → LpSeat.reserved_quote_q64 += 1M

// 2. Client calls RouterLiquidity
RouterLiquidity {
    guard: RiskGuard {
        max_slippage_bps: 100,  // 1%
        max_fee_bps: 30,        // 0.3%
        oracle_bound_bps: 200,   // 2%
    },
    intent: LiquidityIntent::AmmAdd {
        lower_px_q64: 0,
        upper_px_q64: u128::MAX,
        quote_notional_q64: 1_000_000 << 64,
        curve_id: 0,
        fee_bps: 30,
    },
}

// 3. Router → Matcher CPI
// Matcher returns:
LiquidityResult {
    lp_shares_delta: 1_000_000,  // Minted 1M LP shares
    exposure_delta: Exposure {
        base_q64: 500_000 << 64,  // Used 500K base
        quote_q64: -500_000 << 64, // Sold 500K quote for base
    },
    maker_fee_credits: 0,
    realized_pnl_delta: 0,
}

// 4. Router applies result
// → LpSeat.lp_shares = 1_000_000
// → LpSeat.exposure.base_q64 = 500_000 << 64
// → LpSeat.exposure.quote_q64 = -500_000 << 64
// → VenuePnl updated with deltas

// 5. Router validates credit discipline
// |exposure.base| = 500K ≤ reserved_base * (1 - haircut) = 1M * 0.9 = 900K ✓
// |exposure.quote| = 500K ≤ reserved_quote * (1 - haircut) = 1M * 0.9 = 900K ✓
```

## Testing Strategy

### Unit Tests
- Test CPI serialization/deserialization
- Test result application with various deltas
- Test credit discipline validation
- Test error cases (frozen seat, mismatched venue, etc.)

### Integration Tests
- Deploy mock matcher adapter
- Test full Reserve → Liquidity → Release flow
- Test various LiquidityIntent variants
- Test risk guard enforcement

### Fuzz Tests
- Random exposure deltas
- Edge case share amounts
- Overflow/underflow scenarios

## Next Steps for Production

1. **Implement Borsh Serialization**
   - Add `#[derive(BorshSerialize, BorshDeserialize)]` to adapter-core types
   - Test serialization roundtrip for all LiquidityIntent variants

2. **Implement Actual CPI in router_liquidity.rs**
   - Replace placeholder with invoke_matcher_liquidity()
   - Add proper error handling and validation
   - Test with real matcher programs

3. **Create Reference Matcher Adapter**
   - Implement minimal AMM or orderbook matcher
   - Demonstrate proper LiquidityResult calculation
   - Provide as template for third-party matchers

4. **Add Initialization Instructions**
   - InitializeRouterLpSeat (creates LP seat PDA)
   - InitializeVenuePnl (creates venue PnL PDA)
   - Wire into entrypoint dispatch

5. **CLI Borsh Serialization**
   - Replace simplified serialization in liquidity.rs
   - Use proper Borsh encoding for RiskGuard and LiquidityIntent
   - Add tests for instruction data format
