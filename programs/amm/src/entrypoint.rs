//! Program entrypoint

use crate::{adapter, instructions};
use adapter_core::{LiquidityIntent, RemoveSel, RiskGuard};
use percolator_common::{PercolatorError, Side};
use pinocchio::{
    account_info::AccountInfo,
    entrypoint,
    msg,
    program_error::ProgramError,
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
        2 => {
            // adapter_liquidity: intent_discriminator(1) + intent_data + risk_guard(8)
            if data.is_empty() {
                return Err(PercolatorError::InvalidInstruction.into());
            }

            let intent_disc = data[0];
            let mut offset = 1;

            // Parse intent based on discriminator
            let intent = match intent_disc {
                0 => {
                    // AmmAdd: lower_px(16) + upper_px(16) + quote_notional(16) + curve_id(4) + fee_bps(2)
                    if data.len() < 1 + 16 + 16 + 16 + 4 + 2 + 8 {
                        return Err(PercolatorError::InvalidInstruction.into());
                    }

                    let lower_px_q64 = u128::from_le_bytes(data[offset..offset+16].try_into().unwrap());
                    offset += 16;
                    let upper_px_q64 = u128::from_le_bytes(data[offset..offset+16].try_into().unwrap());
                    offset += 16;
                    let quote_notional_q64 = u128::from_le_bytes(data[offset..offset+16].try_into().unwrap());
                    offset += 16;
                    let curve_id = u32::from_le_bytes(data[offset..offset+4].try_into().unwrap());
                    offset += 4;
                    let fee_bps = u16::from_le_bytes(data[offset..offset+2].try_into().unwrap());
                    offset += 2;

                    LiquidityIntent::AmmAdd {
                        lower_px_q64,
                        upper_px_q64,
                        quote_notional_q64,
                        curve_id,
                        fee_bps,
                    }
                }
                1 => {
                    // Remove: selector_disc(1) + shares(16)
                    if data.len() < 1 + 1 + 16 + 8 {
                        return Err(PercolatorError::InvalidInstruction.into());
                    }

                    let selector_disc = data[offset];
                    offset += 1;

                    let selector = match selector_disc {
                        0 => {
                            // AmmByShares
                            let shares = u128::from_le_bytes(data[offset..offset+16].try_into().unwrap());
                            offset += 16;
                            RemoveSel::AmmByShares { shares }
                        }
                        _ => {
                            msg!("Error: Unsupported remove selector");
                            return Err(PercolatorError::InvalidInstruction.into());
                        }
                    };

                    LiquidityIntent::Remove { selector }
                }
                _ => {
                    msg!("Error: Unsupported liquidity intent");
                    return Err(PercolatorError::InvalidInstruction.into());
                }
            };

            // Parse RiskGuard (last 8 bytes)
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

            // TODO: Return LiquidityResult to caller (requires result serialization)
            let _ = result;

            Ok(())
        }
        _ => {
            msg!("Error: Unknown instruction discriminator");
            Err(PercolatorError::InvalidInstruction.into())
        }
    }
}
