//! Router program entrypoint

use pinocchio::{
    account_info::AccountInfo,
    entrypoint,
    msg,
    pubkey::Pubkey,
    ProgramResult,
};

use crate::instructions::{RouterInstruction, process_deposit, process_withdraw, process_initialize_registry, process_initialize_portfolio, process_execute_cross_slab, process_liquidate_user, process_burn_lp_shares, process_cancel_lp_orders, process_register_slab, process_router_reserve, process_router_release, process_router_liquidity, process_router_seat_init};
use crate::state::{Vault, Portfolio, SlabRegistry, RouterLpSeat, VenuePnl};
use percolator_common::{PercolatorError, validate_owner, validate_writable, borrow_account_data, borrow_account_data_mut, InstructionReader};

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // Check minimum instruction data length
    if instruction_data.is_empty() {
        msg!("Error: Instruction data is empty");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    // Parse instruction discriminator (v0 minimal)
    let discriminator = instruction_data[0];
    let instruction = match discriminator {
        0 => RouterInstruction::Initialize,
        1 => RouterInstruction::InitializePortfolio,
        2 => RouterInstruction::Deposit,
        3 => RouterInstruction::Withdraw,
        4 => RouterInstruction::ExecuteCrossSlab,
        5 => RouterInstruction::LiquidateUser,
        6 => RouterInstruction::BurnLpShares,
        7 => RouterInstruction::CancelLpOrders,
        8 => RouterInstruction::RegisterSlab,
        9 => RouterInstruction::RouterReserve,
        10 => RouterInstruction::RouterRelease,
        11 => RouterInstruction::RouterLiquidity,
        12 => RouterInstruction::RouterSeatInit,
        _ => {
            msg!("Error: Unknown instruction");
            return Err(PercolatorError::InvalidInstruction.into());
        }
    };

    // Dispatch to instruction handler (v0 minimal)
    match instruction {
        RouterInstruction::Initialize => {
            msg!("Instruction: Initialize");
            process_initialize_inner(program_id, accounts, &instruction_data[1..])
        }
        RouterInstruction::InitializePortfolio => {
            msg!("Instruction: InitializePortfolio");
            process_initialize_portfolio_inner(program_id, accounts, &instruction_data[1..])
        }
        RouterInstruction::Deposit => {
            msg!("Instruction: Deposit");
            process_deposit_inner(program_id, accounts, &instruction_data[1..])
        }
        RouterInstruction::Withdraw => {
            msg!("Instruction: Withdraw");
            process_withdraw_inner(program_id, accounts, &instruction_data[1..])
        }
        RouterInstruction::ExecuteCrossSlab => {
            msg!("Instruction: ExecuteCrossSlab");
            process_execute_cross_slab_inner(program_id, accounts, &instruction_data[1..])
        }
        RouterInstruction::LiquidateUser => {
            msg!("Instruction: LiquidateUser");
            process_liquidate_user_inner(program_id, accounts, &instruction_data[1..])
        }
        RouterInstruction::BurnLpShares => {
            msg!("Instruction: BurnLpShares");
            process_burn_lp_shares_inner(program_id, accounts, &instruction_data[1..])
        }
        RouterInstruction::CancelLpOrders => {
            msg!("Instruction: CancelLpOrders");
            process_cancel_lp_orders_inner(program_id, accounts, &instruction_data[1..])
        }
        RouterInstruction::RegisterSlab => {
            msg!("Instruction: RegisterSlab");
            process_register_slab_inner(program_id, accounts, &instruction_data[1..])
        }
        RouterInstruction::RouterReserve => {
            msg!("Instruction: RouterReserve");
            process_router_reserve_inner(program_id, accounts, &instruction_data[1..])
        }
        RouterInstruction::RouterRelease => {
            msg!("Instruction: RouterRelease");
            process_router_release_inner(program_id, accounts, &instruction_data[1..])
        }
        RouterInstruction::RouterLiquidity => {
            msg!("Instruction: RouterLiquidity");
            process_router_liquidity_inner(program_id, accounts, &instruction_data[1..])
        }
        RouterInstruction::RouterSeatInit => {
            msg!("Instruction: RouterSeatInit");
            process_router_seat_init_inner(program_id, accounts, &instruction_data[1..])
        }
    }
}

// Instruction processors with account validation

/// Process initialize instruction
///
/// Expected accounts:
/// 0. `[writable]` Registry account (PDA, must be pre-created)
/// 1. `[signer, writable]` Payer account
///
/// Expected data layout (32 bytes):
/// - governance: Pubkey (32 bytes)
fn process_initialize_inner(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 2 {
        msg!("Error: Initialize instruction requires at least 2 accounts");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let registry_account = &accounts[0];
    let payer_account = &accounts[1];

    // Validate accounts
    validate_writable(registry_account)?;
    validate_writable(payer_account)?;

    // Parse instruction data - governance pubkey
    let mut reader = InstructionReader::new(data);
    let governance_bytes = reader.read_bytes::<32>()?;
    let governance = Pubkey::from(governance_bytes);

    // Call the initialization logic
    process_initialize_registry(program_id, registry_account, payer_account, &governance)?;

    msg!("Router initialized successfully");
    Ok(())
}

/// Process deposit instruction (SOL only for MVP)
///
/// Expected accounts:
/// 0. `[writable]` Portfolio account (receives SOL)
/// 1. `[signer, writable]` User account (sends SOL)
/// 2. `[]` System program
///
/// Expected data layout (8 bytes):
/// - amount: u64 (8 bytes, lamports)
fn process_deposit_inner(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 3 {
        msg!("Error: Deposit instruction requires at least 3 accounts");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let portfolio_account = &accounts[0];
    let user_account = &accounts[1];
    let system_program = &accounts[2];

    // Validate accounts
    validate_owner(portfolio_account, program_id)?;
    validate_writable(portfolio_account)?;
    validate_writable(user_account)?;

    // Borrow portfolio data
    let portfolio = unsafe { borrow_account_data_mut::<Portfolio>(portfolio_account)? };

    // Parse instruction data
    let mut reader = InstructionReader::new(data);
    let amount = reader.read_u64()?;

    // Call the instruction handler
    process_deposit(portfolio_account, portfolio, user_account, system_program, amount)?;

    msg!("Deposit processed successfully");
    Ok(())
}

/// Process withdraw instruction (SOL only for MVP)
///
/// Expected accounts:
/// 0. `[writable]` Portfolio account (sends SOL)
/// 1. `[signer, writable]` User account (receives SOL)
/// 2. `[]` System program
/// 3. `[]` Registry account (for warmup state)
///
/// Expected data layout (8 bytes):
/// - amount: u64 (8 bytes, lamports)
fn process_withdraw_inner(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 4 {
        msg!("Error: Withdraw instruction requires at least 4 accounts");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let portfolio_account = &accounts[0];
    let user_account = &accounts[1];
    let system_program = &accounts[2];
    let registry_account = &accounts[3];

    // Validate accounts
    validate_owner(portfolio_account, program_id)?;
    validate_writable(portfolio_account)?;
    validate_writable(user_account)?;
    validate_owner(registry_account, program_id)?;

    // Borrow account data
    let portfolio = unsafe { borrow_account_data_mut::<Portfolio>(portfolio_account)? };
    let registry = unsafe { borrow_account_data::<SlabRegistry>(registry_account)? };

    // Parse instruction data
    let mut reader = InstructionReader::new(data);
    let amount = reader.read_u64()?;

    // Call the instruction handler
    process_withdraw(portfolio_account, portfolio, user_account, system_program, registry, amount)?;

    msg!("Withdraw processed successfully");
    Ok(())
}

/// Process initialize portfolio instruction
///
/// Expected accounts:
/// 0. `[writable]` Portfolio account (created with seed "portfolio")
/// 1. `[signer, writable]` Payer (user funding the account)
///
/// Instruction data: user pubkey (32 bytes)
fn process_initialize_portfolio_inner(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 2 {
        msg!("Error: InitializePortfolio instruction requires at least 2 accounts");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    // Parse user pubkey from instruction data (32 bytes)
    if data.len() < 32 {
        msg!("Error: InitializePortfolio instruction requires user pubkey in data");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let user_bytes: [u8; 32] = data[0..32].try_into()
        .map_err(|_| PercolatorError::InvalidInstruction)?;
    let user = Pubkey::from(user_bytes);

    let portfolio_account = &accounts[0];
    let payer = &accounts[1];

    // Validate accounts
    if !portfolio_account.is_writable() {
        msg!("Error: Portfolio account must be writable");
        return Err(PercolatorError::InvalidAccount.into());
    }

    if !payer.is_signer() {
        msg!("Error: Payer must be a signer");
        return Err(PercolatorError::Unauthorized.into());
    }

    if !payer.is_writable() {
        msg!("Error: Payer must be writable");
        return Err(PercolatorError::InvalidAccount.into());
    }

    // Call the initialization logic
    process_initialize_portfolio(program_id, portfolio_account, payer, &user)?;

    msg!("Portfolio initialized successfully");
    Ok(())
}

/// Process execute cross-slab instruction (v0 main instruction)
///
/// Expected accounts:
/// 0. `[writable]` Portfolio account
/// 1. `[signer]` User authority
/// 2. `[writable]` Vault account
/// 3. `[writable]` Registry account
/// 4. `[]` Router authority PDA
/// 5..5+N. `[]` Oracle accounts (N = num_splits)
/// 5+N..5+2N. `[writable]` Slab accounts (N = num_splits)
/// 5+2N..5+3N. `[writable]` Receipt PDAs (N = num_splits)
///
/// Instruction data layout:
/// - num_splits: u8 (1 byte)
/// - For each split (17 bytes):
///   - side: u8 (0 = buy, 1 = sell)
///   - qty: i64 (quantity in 1e6 scale)
///   - limit_px: i64 (limit price in 1e6 scale)
///
/// Total size: 1 + (17 * num_splits) bytes
/// Maximum splits: 8 (to avoid stack overflow)
fn process_execute_cross_slab_inner(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 5 {
        msg!("Error: ExecuteCrossSlab requires at least 5 accounts");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let portfolio_account = &accounts[0];
    let user_account = &accounts[1];
    let vault_account = &accounts[2];
    let registry_account = &accounts[3];
    let router_authority = &accounts[4];

    // Validate accounts
    validate_owner(portfolio_account, program_id)?;
    validate_writable(portfolio_account)?;
    validate_owner(vault_account, program_id)?;
    validate_writable(vault_account)?;
    validate_owner(registry_account, program_id)?;
    validate_writable(registry_account)?;

    // Borrow account data mutably
    let portfolio = unsafe { borrow_account_data_mut::<Portfolio>(portfolio_account)? };
    let vault = unsafe { borrow_account_data_mut::<Vault>(vault_account)? };
    let registry = unsafe { borrow_account_data_mut::<SlabRegistry>(registry_account)? };

    // Parse instruction data: num_splits (u8) + splits (17 bytes each)
    // Layout per split: side (u8) + qty (i64) + limit_px (i64)
    if data.is_empty() {
        msg!("Error: Instruction data is empty");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let mut reader = InstructionReader::new(data);
    let num_splits = reader.read_u8()? as usize;

    if num_splits == 0 {
        msg!("Error: num_splits must be > 0");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    // Verify we have enough accounts: 5 base + num_splits oracles + num_splits slabs + num_splits receipts
    let required_accounts = 5 + (num_splits * 3);
    if accounts.len() < required_accounts {
        msg!("Error: Insufficient accounts for ExecuteCrossSlab");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    // Split accounts into oracles, slabs, and receipts
    let oracle_accounts = &accounts[5..5 + num_splits];
    let slab_accounts = &accounts[5 + num_splits..5 + (num_splits * 2)];
    let receipt_accounts = &accounts[5 + (num_splits * 2)..5 + (num_splits * 3)];

    // Parse splits from instruction data (on stack, small)
    // Use a fixed-size buffer to avoid heap allocation
    const MAX_SPLITS: usize = 8;
    if num_splits > MAX_SPLITS {
        msg!("Error: num_splits exceeds maximum");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    use crate::instructions::SlabSplit;
    let mut splits_buffer = [SlabSplit {
        slab_id: Pubkey::default(),
        qty: 0,
        side: 0,
        limit_px: 0,
    }; MAX_SPLITS];

    for i in 0..num_splits {
        let side = reader.read_u8()?;
        let qty = reader.read_i64()?;
        let limit_px = reader.read_i64()?;

        // Validate side
        if side > 1 {
            msg!("Error: Invalid side");
            return Err(PercolatorError::InvalidSide.into());
        }

        // Get slab_id from the corresponding account
        let slab_id = *slab_accounts[i].key();

        splits_buffer[i] = SlabSplit {
            slab_id,
            qty,
            side,
            limit_px,
        };
    }

    let splits = &splits_buffer[..num_splits];

    // Call the instruction handler
    process_execute_cross_slab(
        portfolio,
        user_account.key(),
        vault,
        registry,
        router_authority,
        slab_accounts,
        receipt_accounts,
        oracle_accounts,
        splits,
    )?;

    msg!("ExecuteCrossSlab processed successfully");
    Ok(())
}

/// Process liquidate user instruction
///
/// Expected accounts:
/// 0. `[writable]` Portfolio account (to be liquidated)
/// 1. `[]` Registry account
/// 2. `[writable]` Vault account
/// 3. `[]` Router authority PDA
/// 4..4+N. `[]` Oracle accounts (N = num_oracles)
/// 4+N..4+N+M. `[writable]` Slab accounts (M = num_slabs)
/// 4+N+M..4+N+2M. `[writable]` Receipt PDAs (M = num_slabs)
///
/// Instruction data layout:
/// - num_oracles: u8 (1 byte)
/// - num_slabs: u8 (1 byte)
/// - is_preliq: u8 (1 byte, 0 = auto, 1 = force pre-liq)
/// - current_ts: u64 (8 bytes, Unix timestamp)
///
/// Total size: 11 bytes
fn process_liquidate_user_inner(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 4 {
        msg!("Error: LiquidateUser requires at least 4 accounts");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let portfolio_account = &accounts[0];
    let registry_account = &accounts[1];
    let vault_account = &accounts[2];
    let router_authority = &accounts[3];

    // Validate accounts
    validate_owner(portfolio_account, program_id)?;
    validate_writable(portfolio_account)?;
    validate_owner(registry_account, program_id)?;
    validate_writable(registry_account)?;
    validate_owner(vault_account, program_id)?;
    validate_writable(vault_account)?;

    // Borrow account data mutably
    let portfolio = unsafe { borrow_account_data_mut::<Portfolio>(portfolio_account)? };
    let registry = unsafe { borrow_account_data_mut::<SlabRegistry>(registry_account)? };
    let vault = unsafe { borrow_account_data_mut::<Vault>(vault_account)? };

    // Parse instruction data
    if data.len() < 11 {
        msg!("Error: Instruction data too short");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let mut reader = InstructionReader::new(data);
    let num_oracles = reader.read_u8()? as usize;
    let num_slabs = reader.read_u8()? as usize;
    let is_preliq = reader.read_u8()? != 0;
    let current_ts = reader.read_u64()?;

    // Verify we have enough accounts
    let required_accounts = 4 + num_oracles + num_slabs * 2;
    if accounts.len() < required_accounts {
        msg!("Error: Insufficient accounts for LiquidateUser");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    // Split accounts
    let oracle_accounts = &accounts[4..4 + num_oracles];
    let slab_accounts = &accounts[4 + num_oracles..4 + num_oracles + num_slabs];
    let receipt_accounts = &accounts[4 + num_oracles + num_slabs..4 + num_oracles + num_slabs * 2];

    // Call the instruction handler
    process_liquidate_user(
        portfolio,
        registry,
        vault,
        router_authority,
        oracle_accounts,
        slab_accounts,
        receipt_accounts,
        is_preliq,
        current_ts,
    )?;

    msg!("LiquidateUser processed successfully");
    Ok(())
}

/// Process burn LP shares instruction
///
/// Expected accounts:
/// 0. `[writable]` Portfolio account
/// 1. `[signer]` User authority
///
/// Instruction data layout:
/// - market_id: Pubkey (32 bytes)
/// - shares_to_burn: u64 (8 bytes)
/// - current_share_price: i64 (8 bytes)
/// - current_ts: u64 (8 bytes)
/// - max_staleness_seconds: u64 (8 bytes)
///
/// Total size: 64 bytes
fn process_burn_lp_shares_inner(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 2 {
        msg!("Error: BurnLpShares requires at least 2 accounts");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let portfolio_account = &accounts[0];
    let _user_account = &accounts[1];

    // Validate accounts
    validate_owner(portfolio_account, program_id)?;
    validate_writable(portfolio_account)?;

    // Borrow account data mutably
    let portfolio = unsafe { borrow_account_data_mut::<Portfolio>(portfolio_account)? };

    // Parse instruction data
    if data.len() < 64 {
        msg!("Error: Instruction data too short");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let mut reader = InstructionReader::new(data);
    let market_id_bytes = reader.read_bytes::<32>()?;
    let market_id = Pubkey::from(market_id_bytes);
    let shares_to_burn = reader.read_u64()?;
    let current_share_price = reader.read_i64()?;
    let current_ts = reader.read_u64()?;
    let max_staleness_seconds = reader.read_u64()?;

    // Call the instruction handler
    process_burn_lp_shares(
        portfolio,
        market_id,
        shares_to_burn,
        current_share_price,
        current_ts,
        max_staleness_seconds,
    )?;

    msg!("BurnLpShares processed successfully");
    Ok(())
}

/// Process cancel LP orders instruction
///
/// Expected accounts:
/// 0. `[writable]` Portfolio account
/// 1. `[signer]` User authority
///
/// Instruction data layout:
/// - market_id: Pubkey (32 bytes)
/// - order_count: u8 (1 byte)
/// - order_ids: [u64; order_count] (8 * order_count bytes)
/// - freed_quote: u128 (16 bytes)
/// - freed_base: u128 (16 bytes)
///
/// Total size: 65 + (8 * order_count) bytes
fn process_cancel_lp_orders_inner(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 2 {
        msg!("Error: CancelLpOrders requires at least 2 accounts");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let portfolio_account = &accounts[0];
    let _user_account = &accounts[1];

    // Validate accounts
    validate_owner(portfolio_account, program_id)?;
    validate_writable(portfolio_account)?;

    // Borrow account data mutably
    let portfolio = unsafe { borrow_account_data_mut::<Portfolio>(portfolio_account)? };

    // Parse instruction data
    if data.len() < 65 {
        msg!("Error: Instruction data too short");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let mut reader = InstructionReader::new(data);
    let market_id_bytes = reader.read_bytes::<32>()?;
    let market_id = Pubkey::from(market_id_bytes);
    let order_count = reader.read_u8()? as usize;

    // Read order IDs (up to 16 max for stack safety)
    const MAX_ORDERS: usize = 16;
    if order_count > MAX_ORDERS {
        msg!("Error: order_count exceeds maximum");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let mut order_ids_buffer = [0u64; MAX_ORDERS];
    for i in 0..order_count {
        order_ids_buffer[i] = reader.read_u64()?;
    }
    let order_ids = &order_ids_buffer[..order_count];

    let freed_quote = reader.read_u128()?;
    let freed_base = reader.read_u128()?;

    // Call the instruction handler
    process_cancel_lp_orders(
        portfolio,
        market_id,
        order_ids,
        order_count,
        freed_quote,
        freed_base,
    )?;

    msg!("CancelLpOrders processed successfully");
    Ok(())
}

/// Process register_slab instruction (governance-only)
///
/// Expected accounts:
/// 0. `[writable]` Registry account
/// 1. `[signer]` Governance account
///
/// Expected data layout (152 bytes):
/// - slab_id: Pubkey (32 bytes)
/// - version_hash: [u8; 32] (32 bytes)
/// - oracle_id: Pubkey (32 bytes)
/// - imr: u64 (8 bytes)
/// - mmr: u64 (8 bytes)
/// - maker_fee_cap: u64 (8 bytes)
/// - taker_fee_cap: u64 (8 bytes)
/// - latency_sla_ms: u64 (8 bytes)
/// - max_exposure: u128 (16 bytes)
fn process_register_slab_inner(_program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 2 {
        msg!("Error: RegisterSlab instruction requires at least 2 accounts");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let registry_account = &accounts[0];
    let governance_account = &accounts[1];

    // Validate accounts
    validate_writable(registry_account)?;

    // Parse instruction data
    if data.len() < 152 {
        msg!("Error: Instruction data too short");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let mut reader = InstructionReader::new(data);
    let slab_id_bytes = reader.read_bytes::<32>()?;
    let slab_id = Pubkey::from(slab_id_bytes);
    let version_hash = reader.read_bytes::<32>()?;
    let oracle_id_bytes = reader.read_bytes::<32>()?;
    let oracle_id = Pubkey::from(oracle_id_bytes);
    let imr = reader.read_u64()?;
    let mmr = reader.read_u64()?;
    let maker_fee_cap = reader.read_u64()?;
    let taker_fee_cap = reader.read_u64()?;
    let latency_sla_ms = reader.read_u64()?;
    let max_exposure = reader.read_u128()?;

    // Call the instruction handler
    process_register_slab(
        registry_account,
        governance_account,
        slab_id,
        version_hash,
        oracle_id,
        imr,
        mmr,
        maker_fee_cap,
        taker_fee_cap,
        latency_sla_ms,
        max_exposure,
    )?;

    msg!("RegisterSlab processed successfully");
    Ok(())
}

/// Process router_reserve instruction
///
/// Expected accounts:
/// 0. `[writable]` Portfolio account
/// 1. `[writable]` LP seat account
///
/// Expected data layout (32 bytes):
/// - base_amount_q64: u128 (16 bytes)
/// - quote_amount_q64: u128 (16 bytes)
fn process_router_reserve_inner(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 2 {
        msg!("Error: RouterReserve instruction requires at least 2 accounts");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let portfolio_account = &accounts[0];
    let seat_account = &accounts[1];

    // Validate accounts
    validate_owner(portfolio_account, program_id)?;
    validate_writable(portfolio_account)?;
    validate_owner(seat_account, program_id)?;
    validate_writable(seat_account)?;

    // Borrow account data
    let portfolio = unsafe { borrow_account_data_mut::<Portfolio>(portfolio_account)? };
    let seat = unsafe { borrow_account_data_mut::<RouterLpSeat>(seat_account)? };

    // Parse instruction data
    if data.len() < 32 {
        msg!("Error: Instruction data too short");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let mut reader = InstructionReader::new(data);
    let base_amount_q64 = reader.read_u128()?;
    let quote_amount_q64 = reader.read_u128()?;

    // Call the instruction handler
    process_router_reserve(
        portfolio_account,
        portfolio,
        seat_account,
        seat,
        base_amount_q64,
        quote_amount_q64,
    )?;

    msg!("RouterReserve processed successfully");
    Ok(())
}

/// Process router_release instruction
///
/// Expected accounts:
/// 0. `[writable]` Portfolio account
/// 1. `[writable]` LP seat account
///
/// Expected data layout (32 bytes):
/// - base_amount_q64: u128 (16 bytes)
/// - quote_amount_q64: u128 (16 bytes)
fn process_router_release_inner(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 2 {
        msg!("Error: RouterRelease instruction requires at least 2 accounts");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let portfolio_account = &accounts[0];
    let seat_account = &accounts[1];

    // Validate accounts
    validate_owner(portfolio_account, program_id)?;
    validate_writable(portfolio_account)?;
    validate_owner(seat_account, program_id)?;
    validate_writable(seat_account)?;

    // Borrow account data
    let portfolio = unsafe { borrow_account_data_mut::<Portfolio>(portfolio_account)? };
    let seat = unsafe { borrow_account_data_mut::<RouterLpSeat>(seat_account)? };

    // Parse instruction data
    if data.len() < 32 {
        msg!("Error: Instruction data too short");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let mut reader = InstructionReader::new(data);
    let base_amount_q64 = reader.read_u128()?;
    let quote_amount_q64 = reader.read_u128()?;

    // Call the instruction handler
    process_router_release(
        portfolio_account,
        portfolio,
        seat_account,
        seat,
        base_amount_q64,
        quote_amount_q64,
    )?;

    msg!("RouterRelease processed successfully");
    Ok(())
}

/// Process router_liquidity instruction
///
/// Expected accounts:
/// 0. `[writable]` Portfolio account
/// 1. `[writable]` LP seat account
/// 2. `[writable]` Venue PnL account
/// 3. `[]` Matcher program account
///
/// Expected data layout:
/// - guard: RiskGuard (6 bytes: max_slippage_bps u16, max_fee_bps u16, oracle_bound_bps u16)
/// - intent: LiquidityIntent (variable length, for now we'll use a placeholder)
fn process_router_liquidity_inner(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 4 {
        msg!("Error: RouterLiquidity instruction requires at least 4 accounts");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let portfolio_account = &accounts[0];
    let seat_account = &accounts[1];
    let venue_pnl_account = &accounts[2];
    let matcher_program = &accounts[3];

    // Validate accounts
    validate_owner(portfolio_account, program_id)?;
    validate_writable(portfolio_account)?;
    validate_owner(seat_account, program_id)?;
    validate_writable(seat_account)?;
    validate_owner(venue_pnl_account, program_id)?;
    validate_writable(venue_pnl_account)?;

    // Borrow account data
    let portfolio = unsafe { borrow_account_data_mut::<Portfolio>(portfolio_account)? };
    let seat = unsafe { borrow_account_data_mut::<RouterLpSeat>(seat_account)? };
    let venue_pnl = unsafe { borrow_account_data_mut::<VenuePnl>(venue_pnl_account)? };

    // Zero-copy deserialization using hand-rolled bindings
    // Layout: [RiskGuard: 8 bytes][LiquidityIntent: variable]
    if data.len() < 8 {
        msg!("Error: Instruction data too short for RiskGuard");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    // Deserialize RiskGuard (8 bytes fixed size)
    let guard = crate::lp_adapter_serde::deserialize_risk_guard(data)?;

    // Deserialize LiquidityIntent (variable size)
    let (intent, _bytes_read) = crate::lp_adapter_serde::deserialize_liquidity_intent(&data[8..])?;

    // Call the instruction handler
    process_router_liquidity(
        portfolio_account,
        portfolio,
        seat_account,
        seat,
        venue_pnl_account,
        venue_pnl,
        matcher_program,
        guard,
        intent,
    )?;

    msg!("RouterLiquidity processed successfully");
    Ok(())
}

/// Process router_seat_init instruction
///
/// Expected accounts:
/// 0. `[writable]` LP seat account (PDA to be initialized)
/// 1. `[writable]` Portfolio account
/// 2. `[signer]` Signer (portfolio owner)
///
/// Expected data layout (36 bytes):
/// - matcher_state: Pubkey (32 bytes)
/// - context_id: u32 (4 bytes)
fn process_router_seat_init_inner(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 3 {
        msg!("Error: RouterSeatInit instruction requires at least 3 accounts");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let seat_account = &accounts[0];
    let portfolio_account = &accounts[1];
    let signer = &accounts[2];

    // Validate accounts
    validate_writable(seat_account)?;
    validate_writable(portfolio_account)?;

    // Parse instruction data
    if data.len() < 36 {
        msg!("Error: Instruction data too short");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let mut reader = InstructionReader::new(data);
    let matcher_state_bytes = reader.read_bytes::<32>()?;
    let matcher_state = Pubkey::from(matcher_state_bytes);
    let context_id = reader.read_u32()?;

    // Call the instruction handler
    process_router_seat_init(
        program_id,
        seat_account,
        portfolio_account,
        &matcher_state,
        signer,
        context_id,
    )?;

    msg!("RouterSeatInit processed successfully");
    Ok(())
}
