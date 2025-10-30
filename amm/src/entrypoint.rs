//! Program entrypoint

use crate::instructions;
use percolator_common::{PercolatorError, Side};
use pinocchio::{
    account_info::AccountInfo,
    entrypoint,
    msg,
    pubkey::Pubkey,
    ProgramResult,
};

entrypoint!(process_instruction);

/// Main entrypoint
pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if instruction_data.is_empty() {
        msg!("Error: No instruction data provided");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let discriminator = instruction_data[0];
    let data = &instruction_data[1..];

    match discriminator {
        0 => {
            // Initialize: lp_owner(32) + router_id(32) + instrument(32) +
            //             mark_px(8) + taker_fee_bps(8) + contract_size(8) + bump(1) +
            //             x_reserve(8) + y_reserve(8)
            if data.len() < 137 {
                return Err(PercolatorError::InvalidInstruction.into());
            }

            let lp_owner = Pubkey::from(<[u8; 32]>::try_from(&data[0..32]).unwrap());
            let router_id = Pubkey::from(<[u8; 32]>::try_from(&data[32..64]).unwrap());
            let instrument = Pubkey::from(<[u8; 32]>::try_from(&data[64..96]).unwrap());
            let mark_px = i64::from_le_bytes(data[96..104].try_into().unwrap());
            let taker_fee_bps = i64::from_le_bytes(data[104..112].try_into().unwrap());
            let contract_size = i64::from_le_bytes(data[112..120].try_into().unwrap());
            let bump = data[120];
            let x_reserve = i64::from_le_bytes(data[121..129].try_into().unwrap());
            let y_reserve = i64::from_le_bytes(data[129..137].try_into().unwrap());

            instructions::process_initialize(
                accounts,
                lp_owner,
                router_id,
                instrument,
                mark_px,
                taker_fee_bps,
                contract_size,
                bump,
                x_reserve,
                y_reserve,
            )
        }
        1 => {
            // commit_fill: side(1) + qty(8) + limit_px(8)
            if data.len() < 17 {
                return Err(PercolatorError::InvalidInstruction.into());
            }

            let side = if data[0] == 0 { Side::Buy } else { Side::Sell };
            let qty = i64::from_le_bytes(data[1..9].try_into().unwrap());
            let limit_px = i64::from_le_bytes(data[9..17].try_into().unwrap());

            instructions::process_commit_fill(accounts, side, qty, limit_px)
        }
        _ => {
            msg!("Error: Unknown instruction discriminator");
            Err(PercolatorError::InvalidInstruction.into())
        }
    }
}
