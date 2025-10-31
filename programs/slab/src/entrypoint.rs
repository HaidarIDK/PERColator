//! Slab program entrypoint (v0 minimal)

use pinocchio::{
    account_info::AccountInfo,
    entrypoint,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    ProgramResult,
};

use crate::{adapter, instructions::{SlabInstruction, process_initialize_slab, process_commit_fill, process_place_order, process_cancel_order, process_update_funding, process_halt_trading, process_resume_trading}};
use crate::state::{SlabState, Side as OrderSide};
use crate::state::model_bridge::{TimeInForce, SelfTradePrevent};
use adapter_core::{LiquidityIntent, RemoveSel, RiskGuard, Side as AdapterSide, ObOrder};
use percolator_common::{PercolatorError, validate_owner, validate_writable, validate_signer, borrow_account_data_mut, InstructionReader};

extern crate alloc;
use alloc::vec::Vec;

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

    // Parse instruction discriminator
    let discriminator = instruction_data[0];
    let instruction = match discriminator {
        0 => SlabInstruction::Initialize,
        1 => SlabInstruction::CommitFill,
        2 => SlabInstruction::PlaceOrder,
        3 => SlabInstruction::CancelOrder,
        4 => {
            // Adapter liquidity - handle separately (doesn't use SlabInstruction enum)
            msg!("Instruction: AdapterLiquidity");
            return process_adapter_liquidity_inner(accounts, &instruction_data[1..]);
        }
        5 => SlabInstruction::UpdateFunding,
        6 => SlabInstruction::HaltTrading,
        7 => SlabInstruction::ResumeTrading,
        _ => {
            msg!("Error: Unknown instruction");
            return Err(PercolatorError::InvalidInstruction.into());
        }
    };

    // Dispatch to instruction handler
    match instruction {
        SlabInstruction::Initialize => {
            msg!("Instruction: Initialize");
            process_initialize_inner(program_id, accounts, &instruction_data[1..])
        }
        SlabInstruction::CommitFill => {
            msg!("Instruction: CommitFill");
            process_commit_fill_inner(program_id, accounts, &instruction_data[1..])
        }
        SlabInstruction::PlaceOrder => {
            msg!("Instruction: PlaceOrder");
            process_place_order_inner(program_id, accounts, &instruction_data[1..])
        }
        SlabInstruction::CancelOrder => {
            msg!("Instruction: CancelOrder");
            process_cancel_order_inner(program_id, accounts, &instruction_data[1..])
        }
        SlabInstruction::UpdateFunding => {
            msg!("Instruction: UpdateFunding");
            process_update_funding_inner(program_id, accounts, &instruction_data[1..])
        }
        SlabInstruction::HaltTrading => {
            msg!("Instruction: HaltTrading");
            process_halt_trading_inner(program_id, accounts, &instruction_data[1..])
        }
        SlabInstruction::ResumeTrading => {
            msg!("Instruction: ResumeTrading");
            process_resume_trading_inner(program_id, accounts, &instruction_data[1..])
        }
    }
}

// Instruction processors with account validation

/// Process initialize instruction (v0)
///
/// Expected accounts:
/// 0. `[writable]` Slab state account (PDA, uninitialized)
/// 1. `[signer]` Payer/authority
///
/// Expected data layout (121 bytes):
/// - lp_owner: Pubkey (32 bytes)
/// - router_id: Pubkey (32 bytes)
/// - instrument: Pubkey (32 bytes)
/// - mark_px: i64 (8 bytes)
/// - taker_fee_bps: i64 (8 bytes)
/// - contract_size: i64 (8 bytes)
/// - bump: u8 (1 byte)
fn process_initialize_inner(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 1 {
        msg!("Error: Initialize instruction requires at least 1 account");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let slab_account = &accounts[0];
    validate_owner(slab_account, program_id)?;
    validate_writable(slab_account)?;

    // Parse instruction data
    let mut reader = InstructionReader::new(data);
    let lp_owner_bytes = reader.read_bytes::<32>()?;
    let router_id_bytes = reader.read_bytes::<32>()?;
    let instrument_bytes = reader.read_bytes::<32>()?;
    let mark_px = reader.read_i64()?;
    let taker_fee_bps = reader.read_i64()?;
    let contract_size = reader.read_i64()?;
    let bump = reader.read_u8()?;

    let lp_owner = Pubkey::from(lp_owner_bytes);
    let router_id = Pubkey::from(router_id_bytes);
    let instrument = Pubkey::from(instrument_bytes);

    // Call the initialization logic
    process_initialize_slab(
        program_id,
        slab_account,
        lp_owner,
        router_id,
        instrument,
        mark_px,
        taker_fee_bps,
        contract_size,
        bump,
    )?;

    msg!("Slab initialized successfully");
    Ok(())
}

/// Process commit_fill instruction (v0 - atomic fill)
///
/// Expected accounts:
/// 0. `[writable]` Slab state account
/// 1. `[writable]` Fill receipt account
/// 2. `[signer]` Router signer
/// 3. Taker owner (for self-trade prevention)
///
/// Expected data layout:
/// - expected_seqno: u32 (4 bytes) - expected slab seqno (TOCTOU protection)
/// - side: u8 (1 byte) - 0 = Buy, 1 = Sell
/// - qty: i64 (8 bytes) - quantity to fill (1e6 scale)
/// - limit_px: i64 (8 bytes) - limit price (1e6 scale)
/// - time_in_force: u8 (1 byte, optional) - 0=GTC, 1=IOC, 2=FOK (default: GTC)
/// - self_trade_prevention: u8 (1 byte, optional) - 0=None, 1=CancelNewest, 2=CancelOldest, 3=DecrementAndCancel (default: None)
fn process_commit_fill_inner(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 4 {
        msg!("Error: CommitFill instruction requires at least 4 accounts");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let slab_account = &accounts[0];
    let receipt_account = &accounts[1];
    let router_signer = &accounts[2];
    let taker_owner_account = &accounts[3];

    // Validate slab account
    validate_owner(slab_account, program_id)?;
    validate_writable(slab_account)?;
    validate_writable(receipt_account)?;

    // Borrow slab state mutably
    let slab = unsafe { borrow_account_data_mut::<SlabState>(slab_account)? };

    // Parse instruction data
    let mut reader = InstructionReader::new(data);
    let expected_seqno = reader.read_u32()?;
    let side_byte = reader.read_u8()?;
    let qty = reader.read_i64()?;
    let limit_px = reader.read_i64()?;

    // Optional TIF and STP parameters (backward compatible - default to GTC and None)
    let tif_byte = reader.read_u8().unwrap_or(0);
    let stp_byte = reader.read_u8().unwrap_or(0);

    // Convert side byte to Side enum
    let side = match side_byte {
        0 => OrderSide::Buy,
        1 => OrderSide::Sell,
        _ => {
            msg!("Error: Invalid side");
            return Err(PercolatorError::InvalidSide.into());
        }
    };

    // Convert TIF byte to TimeInForce enum
    let time_in_force = match tif_byte {
        0 => TimeInForce::GTC,
        1 => TimeInForce::IOC,
        2 => TimeInForce::FOK,
        _ => {
            msg!("Error: Invalid time-in-force");
            return Err(PercolatorError::InvalidTimeInForce.into());
        }
    };

    // Convert STP byte to SelfTradePrevent enum
    let self_trade_prevention = match stp_byte {
        0 => SelfTradePrevent::None,
        1 => SelfTradePrevent::CancelNewest,
        2 => SelfTradePrevent::CancelOldest,
        3 => SelfTradePrevent::DecrementAndCancel,
        _ => {
            msg!("Error: Invalid self-trade prevention");
            return Err(PercolatorError::InvalidInstruction.into());
        }
    };

    // Call the commit_fill logic
    process_commit_fill(
        slab,
        receipt_account,
        router_signer.key(),
        expected_seqno,
        taker_owner_account.key(),
        side,
        qty,
        limit_px,
        time_in_force,
        self_trade_prevention,
    )?;

    msg!("CommitFill processed successfully");
    Ok(())
}

/// Process place_order instruction (v1)
///
/// Expected accounts:
/// 0. `[writable]` Slab state account
/// 1. `[signer]` Order owner
///
/// Expected data layout:
/// - side: u8 (1 byte) - 0 = Buy, 1 = Sell
/// - price: i64 (8 bytes) - limit price (1e6 scale)
/// - qty: i64 (8 bytes) - order quantity (1e6 scale)
/// - post_only: u8 (1 byte, optional) - 1 = true, 0 = false (default: 0)
/// - reduce_only: u8 (1 byte, optional) - 1 = true, 0 = false (default: 0)
fn process_place_order_inner(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 2 {
        msg!("Error: PlaceOrder instruction requires at least 2 accounts");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let slab_account = &accounts[0];
    let owner_account = &accounts[1];

    // Validate accounts
    validate_owner(slab_account, program_id)?;
    validate_writable(slab_account)?;
    validate_signer(owner_account)?;

    // Borrow slab state mutably
    let slab = unsafe { borrow_account_data_mut::<SlabState>(slab_account)? };

    // Parse instruction data
    let mut reader = InstructionReader::new(data);
    let side_byte = reader.read_u8()?;
    let price = reader.read_i64()?;
    let qty = reader.read_i64()?;

    // Optional flags (backward compatible - default to false if not present)
    let post_only = reader.read_u8().unwrap_or(0) != 0;
    let reduce_only = reader.read_u8().unwrap_or(0) != 0;

    // Convert side byte to Side enum
    let side = match side_byte {
        0 => OrderSide::Buy,
        1 => OrderSide::Sell,
        _ => {
            msg!("Error: Invalid side");
            return Err(PercolatorError::InvalidSide.into());
        }
    };

    // Call the place_order logic
    let _order_id = process_place_order(
        slab,
        owner_account.key(),
        side,
        price,
        qty,
        post_only,
        reduce_only,
    )?;

    msg!("PlaceOrder processed successfully");
    Ok(())
}

/// Process cancel_order instruction (v1)
///
/// Expected accounts:
/// 0. `[writable]` Slab state account
/// 1. `[signer]` Order owner
///
/// Expected data layout (8 bytes):
/// - order_id: u64 (8 bytes) - ID of the order to cancel
fn process_cancel_order_inner(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 2 {
        msg!("Error: CancelOrder instruction requires at least 2 accounts");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let slab_account = &accounts[0];
    let owner_account = &accounts[1];

    // Validate accounts
    validate_owner(slab_account, program_id)?;
    validate_writable(slab_account)?;
    validate_signer(owner_account)?;

    // Borrow slab state mutably
    let slab = unsafe { borrow_account_data_mut::<SlabState>(slab_account)? };

    // Parse instruction data
    let mut reader = InstructionReader::new(data);
    let order_id = reader.read_u64()?;

    // Call the cancel_order logic
    process_cancel_order(slab, owner_account.key(), order_id)?;

    msg!("CancelOrder processed successfully");
    Ok(())
}

/// Process update_funding instruction
///
/// Expected accounts:
/// 0. `[writable]` Slab state account
/// 1. `[signer]` LP owner (authority)
///
/// Expected data layout (8 bytes):
/// - oracle_price: i64 (8 bytes) - oracle reference price (1e6 scale)
fn process_update_funding_inner(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 2 {
        msg!("Error: UpdateFunding instruction requires at least 2 accounts");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let slab_account = &accounts[0];
    let authority_account = &accounts[1];

    // Validate accounts
    validate_owner(slab_account, program_id)?;
    validate_writable(slab_account)?;
    validate_signer(authority_account)?;

    // Borrow slab state mutably
    let slab = unsafe { borrow_account_data_mut::<SlabState>(slab_account)? };

    // Parse instruction data
    let mut reader = InstructionReader::new(data);
    let oracle_price = reader.read_i64()?;

    // Call the update_funding logic
    process_update_funding(slab, authority_account.key(), oracle_price)?;

    msg!("UpdateFunding processed successfully");
    Ok(())
}

/// Process adapter_liquidity instruction (v1)
///
/// Expected accounts:
/// 0. `[writable]` Slab state account
/// 1. `[signer]` Router signer
///
/// Expected data layout:
/// - intent_discriminator: u8 (1 byte)
/// - intent_data: variable
/// - risk_guard: RiskGuard (8 bytes)
fn process_adapter_liquidity_inner(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if data.is_empty() {
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let intent_disc = data[0];
    let mut offset = 1;

    // Parse intent based on discriminator
    let intent = match intent_disc {
        0 => {
            // AmmAdd - not supported by slab
            msg!("Error: Slab does not support AMM operations");
            return Err(PercolatorError::InvalidInstruction.into());
        }
        1 => {
            // Remove: selector_disc(1) + selector_data
            if data.len() < 1 + 1 + 8 {
                return Err(PercolatorError::InvalidInstruction.into());
            }

            let selector_disc = data[offset];
            offset += 1;

            let selector = match selector_disc {
                0 => {
                    // AmmByShares - not supported by slab
                    msg!("Error: Slab does not support AMM shares");
                    return Err(PercolatorError::InvalidInstruction.into());
                }
                1 => {
                    // ObByIds: order_ids_count(u32) + [order_id(u64); count]
                    if data.len() < offset + 4 {
                        return Err(PercolatorError::InvalidInstruction.into());
                    }
                    let count = u32::from_le_bytes(data[offset..offset+4].try_into().unwrap());
                    offset += 4;

                    let mut order_ids = Vec::new();
                    for _ in 0..count {
                        if data.len() < offset + 8 {
                            return Err(PercolatorError::InvalidInstruction.into());
                        }
                        let order_id = u64::from_le_bytes(data[offset..offset+8].try_into().unwrap());
                        offset += 8;
                        order_ids.push(order_id as u128);
                    }

                    RemoveSel::ObByIds { ids: order_ids }
                }
                2 => {
                    // ObAll - no additional data
                    RemoveSel::ObAll
                }
                _ => {
                    msg!("Error: Unsupported remove selector");
                    return Err(PercolatorError::InvalidInstruction.into());
                }
            };

            LiquidityIntent::Remove { selector }
        }
        2 => {
            // ObAdd: orders_count(4) + [orders] + post_only(1) + reduce_only(1)
            if data.len() < offset + 4 {
                return Err(PercolatorError::InvalidInstruction.into());
            }

            let orders_count = u32::from_le_bytes(data[offset..offset+4].try_into().unwrap());
            offset += 4;

            let mut orders = Vec::new();
            for _ in 0..orders_count {
                // Each order: side(1) + px_q64(16) + qty_q64(16) + tif_slots(4) = 37 bytes
                if data.len() < offset + 37 {
                    return Err(PercolatorError::InvalidInstruction.into());
                }

                let side_byte = data[offset];
                offset += 1;
                let side = match side_byte {
                    0 => AdapterSide::Bid,
                    1 => AdapterSide::Ask,
                    _ => {
                        msg!("Error: Invalid side");
                        return Err(PercolatorError::InvalidSide.into());
                    }
                };

                let px_q64 = u128::from_le_bytes(data[offset..offset+16].try_into().unwrap());
                offset += 16;
                let qty_q64 = u128::from_le_bytes(data[offset..offset+16].try_into().unwrap());
                offset += 16;
                let tif_slots = u32::from_le_bytes(data[offset..offset+4].try_into().unwrap());
                offset += 4;

                orders.push(ObOrder {
                    side,
                    px_q64,
                    qty_q64,
                    tif_slots,
                    _padding: [0; 4],
                });
            }

            if data.len() < offset + 2 {
                return Err(PercolatorError::InvalidInstruction.into());
            }

            let post_only = data[offset] != 0;
            offset += 1;
            let reduce_only = data[offset] != 0;
            offset += 1;

            LiquidityIntent::ObAdd {
                orders,
                post_only,
                reduce_only,
            }
        }
        _ => {
            msg!("Error: Unsupported liquidity intent");
            return Err(PercolatorError::InvalidInstruction.into());
        }
    };

    // Parse RiskGuard (last 8 bytes)
    if data.len() < offset + 8 {
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let guard = RiskGuard {
        max_slippage_bps: u16::from_le_bytes(data[offset..offset+2].try_into().unwrap()),
        max_fee_bps: u16::from_le_bytes(data[offset+2..offset+4].try_into().unwrap()),
        oracle_bound_bps: u16::from_le_bytes(data[offset+4..offset+6].try_into().unwrap()),
        _padding: [0; 2],
    };

    // Call adapter
    let result = adapter::process_adapter_liquidity(accounts, &intent, &guard)
        .map_err(|e: PercolatorError| Into::<ProgramError>::into(e))?;

    msg!("Adapter liquidity operation completed successfully");

    // Serialize LiquidityResult and set return_data
    let result_bytes = serialize_liquidity_result(&result);
    unsafe {
        pinocchio::syscalls::sol_set_return_data(result_bytes.as_ptr(), result_bytes.len() as u64);
    }

    Ok(())
}
/// Serialize LiquidityResult to bytes for return_data
///
/// Format: [lp_shares_delta(16)][base_q64(16)][quote_q64(16)][maker_fee_credits(16)][realized_pnl_delta(16)]
/// Total: 80 bytes
fn serialize_liquidity_result(result: &adapter_core::LiquidityResult) -> [u8; 80] {
    let mut bytes = [0u8; 80];

    // lp_shares_delta: i128 (16 bytes)
    bytes[0..16].copy_from_slice(&result.lp_shares_delta.to_le_bytes());

    // exposure_delta.base_q64: i128 (16 bytes)
    bytes[16..32].copy_from_slice(&result.exposure_delta.base_q64.to_le_bytes());

    // exposure_delta.quote_q64: i128 (16 bytes)
    bytes[32..48].copy_from_slice(&result.exposure_delta.quote_q64.to_le_bytes());

    // maker_fee_credits: i128 (16 bytes)
    bytes[48..64].copy_from_slice(&result.maker_fee_credits.to_le_bytes());

    // realized_pnl_delta: i128 (16 bytes)
    bytes[64..80].copy_from_slice(&result.realized_pnl_delta.to_le_bytes());

    bytes
}

/// Process halt_trading instruction
///
/// Accounts expected:
/// 0. `[writable]` Slab account
/// 1. `[signer]` LP owner (authority)
fn process_halt_trading_inner(_program_id: &Pubkey, accounts: &[AccountInfo], _data: &[u8]) -> ProgramResult {
    // Validate accounts
    if accounts.len() < 2 {
        msg!("Error: Missing required accounts");
        return Err(PercolatorError::InvalidAccount.into());
    }

    let slab_account = &accounts[0];
    let authority_account = &accounts[1];

    // Validate authority is signer
    validate_signer(authority_account)?;

    // Validate slab is writable
    validate_writable(slab_account)?;

    // Borrow slab state mutably
    let slab = unsafe { borrow_account_data_mut::<SlabState>(slab_account)? };

    // Process halt trading
    process_halt_trading(slab, authority_account.key())?;

    msg!("Halt trading completed successfully");
    Ok(())
}

/// Process resume_trading instruction
///
/// Accounts expected:
/// 0. `[writable]` Slab account
/// 1. `[signer]` LP owner (authority)
fn process_resume_trading_inner(_program_id: &Pubkey, accounts: &[AccountInfo], _data: &[u8]) -> ProgramResult {
    // Validate accounts
    if accounts.len() < 2 {
        msg!("Error: Missing required accounts");
        return Err(PercolatorError::InvalidAccount.into());
    }

    let slab_account = &accounts[0];
    let authority_account = &accounts[1];

    // Validate authority is signer
    validate_signer(authority_account)?;

    // Validate slab is writable
    validate_writable(slab_account)?;

    // Borrow slab state mutably
    let slab = unsafe { borrow_account_data_mut::<SlabState>(slab_account)? };

    // Process resume trading
    process_resume_trading(slab, authority_account.key())?;

    msg!("Resume trading completed successfully");
    Ok(())
}
