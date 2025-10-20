//! Slab program entrypoint

use pinocchio::{
    account_info::AccountInfo,
    entrypoint,
    msg,
    pubkey::Pubkey,
    ProgramResult,
};

use crate::instructions::{
    SlabInstruction,
    process_reserve,
    process_commit,
    process_cancel,
};
use crate::matching::funding::{update_funding, update_all_funding};
use crate::state::{SlabState, SlabHeader, Instrument};
use percolator_common::*;

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // Check minimum instruction data length (1 byte for discriminator)
    if instruction_data.is_empty() {
        msg!("Error: Instruction data is empty");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    // Parse instruction discriminator
    let discriminator = instruction_data[0];
    let instruction = match discriminator {
        0 => SlabInstruction::Reserve,
        1 => SlabInstruction::Commit,
        2 => SlabInstruction::Cancel,
        3 => SlabInstruction::BatchOpen,
        4 => SlabInstruction::Initialize,
        5 => SlabInstruction::AddInstrument,
        6 => SlabInstruction::UpdateFunding,
        7 => SlabInstruction::Liquidate,
        _ => {
            msg!("Error: Unknown instruction discriminator: {}", discriminator);
            return Err(PercolatorError::InvalidInstruction.into());
        }
    };

    // Dispatch to instruction handler
    match instruction {
        SlabInstruction::Reserve => {
            msg!("Instruction: Reserve");
            handle_reserve(program_id, accounts, &instruction_data[1..])
        }
        SlabInstruction::Commit => {
            msg!("Instruction: Commit");
            handle_commit(program_id, accounts, &instruction_data[1..])
        }
        SlabInstruction::Cancel => {
            msg!("Instruction: Cancel");
            handle_cancel(program_id, accounts, &instruction_data[1..])
        }
        SlabInstruction::BatchOpen => {
            msg!("Instruction: BatchOpen");
            handle_batch_open(program_id, accounts, &instruction_data[1..])
        }
        SlabInstruction::Initialize => {
            msg!("Instruction: Initialize");
            handle_initialize(program_id, accounts, &instruction_data[1..])
        }
        SlabInstruction::AddInstrument => {
            msg!("Instruction: AddInstrument");
            handle_add_instrument(program_id, accounts, &instruction_data[1..])
        }
        SlabInstruction::UpdateFunding => {
            msg!("Instruction: UpdateFunding");
            handle_update_funding(program_id, accounts, &instruction_data[1..])
        }
        SlabInstruction::Liquidate => {
            msg!("Instruction: Liquidate");
            handle_liquidate(program_id, accounts, &instruction_data[1..])
        }
    }
}

/// Handle Reserve instruction
///
/// Expected accounts:
/// 0. `[writable]` Slab state account
/// 1. `[signer]` User account (optional for signature verification)
///
/// Instruction data:
/// - account_idx (u32): Index of account in slab's account pool
/// - instrument_idx (u16): Index of instrument to trade
/// - side (u8): 0 = Buy, 1 = Sell
/// - qty (u64): Quantity to reserve
/// - limit_px (u64): Limit price
/// - ttl_ms (u64): Time-to-live in milliseconds
/// - commitment_hash ([u8; 32]): Hash for commit-reveal
/// - route_id (u64): Router route identifier
fn handle_reserve(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    // Validate account count
    if accounts.is_empty() {
        msg!("Error: Reserve instruction requires at least 1 account");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    // Account 0: Slab state (must be writable and owned by this program)
    let slab_account = &accounts[0];
    validate_owner(slab_account, program_id)?;
    validate_writable(slab_account)?;

    // Deserialize slab state
    let slab = unsafe { borrow_account_data_mut::<SlabState>(slab_account)? };

    // Parse instruction data
    // Total size: 4 + 2 + 1 + 8 + 8 + 8 + 32 + 8 = 71 bytes
    if data.len() < 71 {
        msg!("Error: Invalid Reserve instruction data length: {}", data.len());
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let mut offset = 0;
    let account_idx = read_u32(data, &mut offset)?;
    let instrument_idx = read_u16(data, &mut offset)?;
    let side_byte = read_u8(data, &mut offset)?;
    let qty = read_u64(data, &mut offset)?;
    let limit_px = read_u64(data, &mut offset)?;
    let ttl_ms = read_u64(data, &mut offset)?;
    let commitment_hash = read_bytes::<32>(data, &mut offset)?;
    let route_id = read_u64(data, &mut offset)?;

    // Parse side
    let side = match side_byte {
        0 => Side::Buy,
        1 => Side::Sell,
        _ => {
            msg!("Error: Invalid side value: {}", side_byte);
            return Err(PercolatorError::InvalidInstruction.into());
        }
    };

    // Call instruction handler
    let result = process_reserve(
        slab,
        account_idx,
        instrument_idx,
        side,
        qty,
        limit_px,
        ttl_ms,
        commitment_hash,
        route_id,
    )?;

    msg!("Reserve successful: hold_id={}, vwap={}, worst_px={}, max_charge={}, filled_qty={}",
        result.hold_id, result.vwap_px, result.worst_px, result.max_charge, result.filled_qty);

    Ok(())
}

/// Handle Commit instruction
///
/// Expected accounts:
/// 0. `[writable]` Slab state account
/// 1. `[signer]` User account (optional)
///
/// Instruction data:
/// - hold_id (u64): Reservation identifier
/// - current_ts (u64): Current timestamp in milliseconds
fn handle_commit(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.is_empty() {
        msg!("Error: Commit instruction requires at least 1 account");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let slab_account = &accounts[0];
    validate_owner(slab_account, program_id)?;
    validate_writable(slab_account)?;

    let slab = unsafe { borrow_account_data_mut::<SlabState>(slab_account)? };

    // Parse instruction data (16 bytes total)
    if data.len() < 16 {
        msg!("Error: Invalid Commit instruction data length: {}", data.len());
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let mut offset = 0;
    let hold_id = read_u64(data, &mut offset)?;
    let current_ts = read_u64(data, &mut offset)?;

    // Call instruction handler
    let result = process_commit(slab, hold_id, current_ts)?;

    msg!("Commit successful: executed {} fills, total_notional={}", 
        result.fills_count, result.total_notional);

    Ok(())
}

/// Handle Cancel instruction
///
/// Expected accounts:
/// 0. `[writable]` Slab state account
/// 1. `[signer]` User account (optional)
///
/// Instruction data:
/// - hold_id (u64): Reservation identifier
fn handle_cancel(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.is_empty() {
        msg!("Error: Cancel instruction requires at least 1 account");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let slab_account = &accounts[0];
    validate_owner(slab_account, program_id)?;
    validate_writable(slab_account)?;

    let slab = unsafe { borrow_account_data_mut::<SlabState>(slab_account)? };

    // Parse instruction data (8 bytes total)
    if data.len() < 8 {
        msg!("Error: Invalid Cancel instruction data length: {}", data.len());
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let mut offset = 0;
    let hold_id = read_u64(data, &mut offset)?;

    // Call instruction handler
    process_cancel(slab, hold_id)?;

    msg!("Cancel successful: hold_id={}", hold_id);

    Ok(())
}

/// Handle BatchOpen instruction
///
/// Expected accounts:
/// 0. `[writable]` Slab state account
/// 1. `[signer]` Authority (optional, for permissioned batch opening)
///
/// Instruction data:
/// - instrument_idx (u16): Instrument to open batch for
/// - current_ts (u64): Current timestamp in milliseconds
fn handle_batch_open(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.is_empty() {
        msg!("Error: BatchOpen instruction requires at least 1 account");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let slab_account = &accounts[0];
    validate_owner(slab_account, program_id)?;
    validate_writable(slab_account)?;

    let slab = unsafe { borrow_account_data_mut::<SlabState>(slab_account)? };

    // Parse instruction data (10 bytes total)
    if data.len() < 10 {
        msg!("Error: Invalid BatchOpen instruction data length: {}", data.len());
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let mut offset = 0;
    let instrument_idx = read_u16(data, &mut offset)?;
    let current_ts = read_u64(data, &mut offset)?;

    // Call batch_open logic
    crate::instructions::process_batch_open(slab, instrument_idx, current_ts)?;

    msg!("BatchOpen successful: instrument_idx={}, new_epoch={}", 
        instrument_idx, 
        slab.get_instrument(instrument_idx).map(|i| i.epoch).unwrap_or(0));

    Ok(())
}

/// Handle Initialize instruction
///
/// Expected accounts:
/// 0. `[writable]` Slab state account (must be allocated with sufficient size)
/// 1. `[signer]` Authority/payer
///
/// Instruction data:
/// - authority (Pubkey): Program authority (32 bytes)
/// - oracle (Pubkey): Oracle program ID (32 bytes)
/// - router (Pubkey): Router program ID (32 bytes)
/// - imr (u16): Initial margin ratio in basis points
/// - mmr (u16): Maintenance margin ratio in basis points
/// - maker_fee (i16): Maker fee in basis points (can be negative for rebates)
/// - taker_fee (i16): Taker fee in basis points
/// - batch_ms (u64): Batch window duration in milliseconds
/// - freeze_levels (u16): Number of top price levels to freeze
fn handle_initialize(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 2 {
        msg!("Error: Initialize instruction requires at least 2 accounts");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let slab_account = &accounts[0];
    let authority_account = &accounts[1];

    validate_owner(slab_account, program_id)?;
    validate_writable(slab_account)?;
    validate_signer(authority_account)?;

    // Parse instruction data (32 + 32 + 32 + 2 + 2 + 2 + 2 + 8 + 2 = 114 bytes minimum)
    if data.len() < 114 {
        msg!("Error: Invalid Initialize instruction data length: {}", data.len());
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let mut offset = 0;
    let authority_bytes = read_bytes::<32>(data, &mut offset)?;
    let oracle_bytes = read_bytes::<32>(data, &mut offset)?;
    let router_bytes = read_bytes::<32>(data, &mut offset)?;
    let imr = read_u16(data, &mut offset)?;
    let mmr = read_u16(data, &mut offset)?;
    let maker_fee = read_i64(data, &mut offset)? as i16;
    let taker_fee = read_i64(data, &mut offset)? as i16;
    let batch_ms = read_u64(data, &mut offset)?;
    let freeze_levels = read_u16(data, &mut offset)?;

    // Convert bytes to Pubkeys
    let authority = Pubkey::from(authority_bytes);
    let oracle = Pubkey::from(oracle_bytes);
    let router = Pubkey::from(router_bytes);

    // Initialize slab state
    let slab = unsafe { borrow_account_data_mut::<SlabState>(slab_account)? };

    // Initialize header
    slab.header = SlabHeader::new(
        authority,
        oracle,
        router,
        imr,
        mmr,
        maker_fee,
        taker_fee,
        batch_ms,
        freeze_levels,
    );

    // Initialize other fields
    slab.instrument_count = 0;

    msg!("Slab initialized successfully");

    Ok(())
}

/// Handle AddInstrument instruction
///
/// Expected accounts:
/// 0. `[writable]` Slab state account
/// 1. `[signer]` Authority
///
/// Instruction data:
/// - symbol ([u8; 8]): Instrument symbol (e.g., "BTC/USDC")
/// - contract_size (u64): Contract size (e.g., 1_000_000 for 1.0)
/// - tick (u64): Minimum price increment
/// - lot (u64): Minimum quantity increment
/// - index_price (u64): Initial index price
fn handle_add_instrument(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 2 {
        msg!("Error: AddInstrument instruction requires at least 2 accounts");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let slab_account = &accounts[0];
    let authority_account = &accounts[1];

    validate_owner(slab_account, program_id)?;
    validate_writable(slab_account)?;
    validate_signer(authority_account)?;

    let slab = unsafe { borrow_account_data_mut::<SlabState>(slab_account)? };

    // Verify authority
    if authority_account.key() != &slab.header.authority {
        msg!("Error: Invalid authority");
        return Err(PercolatorError::Unauthorized.into());
    }

    // Parse instruction data (8 + 8 + 8 + 8 + 8 = 40 bytes)
    if data.len() < 40 {
        msg!("Error: Invalid AddInstrument instruction data length: {}", data.len());
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let mut offset = 0;
    let symbol = read_bytes::<8>(data, &mut offset)?;
    let contract_size = read_u64(data, &mut offset)?;
    let tick = read_u64(data, &mut offset)?;
    let lot = read_u64(data, &mut offset)?;
    let index_price = read_u64(data, &mut offset)?;

    // Check instrument count limit
    if slab.instrument_count >= 32 {
        msg!("Error: Maximum instrument count reached (32)");
        return Err(PercolatorError::PoolFull.into());
    }

    // Add instrument
    let idx = slab.instrument_count as usize;
    slab.instruments[idx] = Instrument {
        symbol,
        contract_size,
        tick,
        lot,
        index_price,
        funding_rate: 0,
        cum_funding: 0,
        last_funding_ts: 0,
        bids_head: u32::MAX,
        asks_head: u32::MAX,
        bids_pending_head: u32::MAX,
        asks_pending_head: u32::MAX,
        epoch: 1,
        index: idx as u16,
        batch_open_ms: 0,
        freeze_until_ms: 0,
    };
    slab.instrument_count += 1;

    msg!("Instrument added: symbol={:?}, index={}", 
        core::str::from_utf8(&symbol).unwrap_or("INVALID"), idx);

    Ok(())
}

/// Handle UpdateFunding instruction
///
/// Expected accounts:
/// 0. `[writable]` Slab state account
///
/// Instruction data:
/// - update_all (u8): If 1, update all instruments; if 0, update single instrument
/// - instrument_idx (u16): Instrument to update (ignored if update_all = 1)
/// - current_ts (u64): Current timestamp in milliseconds
fn handle_update_funding(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.is_empty() {
        msg!("Error: UpdateFunding instruction requires at least 1 account");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let slab_account = &accounts[0];
    validate_owner(slab_account, program_id)?;
    validate_writable(slab_account)?;

    let slab = unsafe { borrow_account_data_mut::<SlabState>(slab_account)? };

    // Parse instruction data (1 + 2 + 8 = 11 bytes)
    if data.len() < 11 {
        msg!("Error: Invalid UpdateFunding instruction data length: {}", data.len());
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let mut offset = 0;
    let update_all = read_u8(data, &mut offset)?;
    let instrument_idx = read_u16(data, &mut offset)?;
    let current_ts = read_u64(data, &mut offset)?;

    if update_all == 1 {
        update_all_funding(slab, current_ts)?;
        msg!("Updated funding for all instruments at ts={}", current_ts);
    } else {
        update_funding(slab, instrument_idx, current_ts)?;
        msg!("Updated funding for instrument {} at ts={}", instrument_idx, current_ts);
    }

    Ok(())
}

/// Handle Liquidate instruction
///
/// Expected accounts:
/// 0. `[writable]` Slab state account
/// 1. `[]` Router program (for validation)
///
/// Instruction data:
/// - account_idx (u32): Account to liquidate
/// - deficit_target (u128): Deficit amount to cover (16 bytes)
/// - liquidation_fee_bps (u16): Liquidation fee in basis points
/// - price_band_bps (u16): Price band limit in basis points
fn handle_liquidate(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.is_empty() {
        msg!("Error: Liquidate instruction requires at least 1 account");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let slab_account = &accounts[0];
    validate_owner(slab_account, program_id)?;
    validate_writable(slab_account)?;

    let slab = unsafe { borrow_account_data_mut::<SlabState>(slab_account)? };

    // Parse instruction data (4 + 16 + 2 + 2 = 24 bytes)
    if data.len() < 24 {
        msg!("Error: Invalid Liquidate instruction data length: {}", data.len());
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let mut offset = 0;
    let account_idx = read_u32(data, &mut offset)?;
    let deficit_target = read_u128(data, &mut offset)?;
    let liquidation_fee_bps = read_u16(data, &mut offset)?;
    let price_band_bps = read_u16(data, &mut offset)?;

    // Call instruction handler
    let result = process_liquidation(
        slab,
        account_idx,
        deficit_target,
        liquidation_fee_bps,
        price_band_bps,
    )?;

    msg!("Liquidation executed: closed_qty={}, fee={}, remaining_deficit={}",
        result.closed_qty, result.liquidation_fee, result.remaining_deficit);

    Ok(())
}
