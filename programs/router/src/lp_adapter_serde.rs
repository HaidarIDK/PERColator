//! Zero-copy serialization/deserialization for LP adapter types
//!
//! Hand-rolled bindings for efficient BPF execution without Borsh dependency.
//! Uses direct memory access with Pinocchio's zero-copy philosophy.

use adapter_core::{LiquidityIntent, RemoveSel, RiskGuard};
use pinocchio::program_error::ProgramError;

/// Deserialize RiskGuard from bytes (zero-copy, 8 bytes fixed size)
///
/// Layout: [max_slippage_bps: u16][max_fee_bps: u16][oracle_bound_bps: u16][_padding: 2]
pub fn deserialize_risk_guard(data: &[u8]) -> Result<RiskGuard, ProgramError> {
    if data.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }

    Ok(RiskGuard {
        max_slippage_bps: u16::from_le_bytes([data[0], data[1]]),
        max_fee_bps: u16::from_le_bytes([data[2], data[3]]),
        oracle_bound_bps: u16::from_le_bytes([data[4], data[5]]),
        _padding: [data[6], data[7]],
    })
}

/// Deserialize LiquidityIntent from bytes
///
/// Layout: [variant_discriminator: u8][variant_fields...]
///
/// Variants:
/// - 0: AmmAdd { lower_px_q64: u128, upper_px_q64: u128, quote_notional_q64: u128, curve_id: u32, fee_bps: u16 }
/// - 1: ObAdd { ... } (not implemented yet)
/// - 2: Hook { ... } (not implemented yet)
/// - 3: Remove { selector: RemoveSel }
/// - 4: Modify { ... } (not implemented yet)
pub fn deserialize_liquidity_intent(data: &[u8]) -> Result<(LiquidityIntent, usize), ProgramError> {
    if data.is_empty() {
        return Err(ProgramError::InvalidInstructionData);
    }

    let variant = data[0];
    let mut offset = 1;

    match variant {
        // AmmAdd
        0 => {
            if data.len() < offset + 16 + 16 + 16 + 4 + 2 {
                return Err(ProgramError::InvalidInstructionData);
            }

            let lower_px_q64 = read_u128(&data[offset..offset + 16])?;
            offset += 16;

            let upper_px_q64 = read_u128(&data[offset..offset + 16])?;
            offset += 16;

            let quote_notional_q64 = read_u128(&data[offset..offset + 16])?;
            offset += 16;

            let curve_id = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            offset += 4;

            let fee_bps = u16::from_le_bytes([data[offset], data[offset + 1]]);
            offset += 2;

            Ok((
                LiquidityIntent::AmmAdd {
                    lower_px_q64,
                    upper_px_q64,
                    quote_notional_q64,
                    curve_id,
                    fee_bps,
                },
                offset,
            ))
        }

        // Remove
        3 => {
            if data.len() < offset + 1 {
                return Err(ProgramError::InvalidInstructionData);
            }

            let selector_variant = data[offset];
            offset += 1;

            let selector = match selector_variant {
                // AmmByShares
                0 => {
                    if data.len() < offset + 16 {
                        return Err(ProgramError::InvalidInstructionData);
                    }
                    let shares = read_u128(&data[offset..offset + 16])?;
                    offset += 16;
                    RemoveSel::AmmByShares { shares }
                }
                // ObByIds - not yet implemented
                1 => return Err(ProgramError::InvalidInstructionData),
                // ObAll
                2 => RemoveSel::ObAll,
                _ => return Err(ProgramError::InvalidInstructionData),
            };

            Ok((LiquidityIntent::Remove { selector }, offset))
        }

        // Other variants not yet implemented
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

/// Helper to read u128 from little-endian bytes
#[inline]
fn read_u128(data: &[u8]) -> Result<u128, ProgramError> {
    if data.len() < 16 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&data[0..16]);
    Ok(u128::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_risk_guard() {
        let data = [
            100, 0, // max_slippage_bps = 100
            50, 0, // max_fee_bps = 50
            200, 0, // oracle_bound_bps = 200
            0, 0, // padding
        ];

        let guard = deserialize_risk_guard(&data).unwrap();
        assert_eq!(guard.max_slippage_bps, 100);
        assert_eq!(guard.max_fee_bps, 50);
        assert_eq!(guard.oracle_bound_bps, 200);
    }

    #[test]
    fn test_deserialize_risk_guard_too_short() {
        let data = [100, 0, 50, 0];
        assert!(deserialize_risk_guard(&data).is_err());
    }

    #[test]
    fn test_deserialize_amm_add() {
        let mut data = vec![0u8]; // AmmAdd variant

        // lower_px_q64 = 1000
        data.extend_from_slice(&1000u128.to_le_bytes());
        // upper_px_q64 = 2000
        data.extend_from_slice(&2000u128.to_le_bytes());
        // quote_notional_q64 = 1_000_000
        data.extend_from_slice(&1_000_000u128.to_le_bytes());
        // curve_id = 0
        data.extend_from_slice(&0u32.to_le_bytes());
        // fee_bps = 30
        data.extend_from_slice(&30u16.to_le_bytes());

        let (intent, offset) = deserialize_liquidity_intent(&data).unwrap();

        match intent {
            LiquidityIntent::AmmAdd {
                lower_px_q64,
                upper_px_q64,
                quote_notional_q64,
                curve_id,
                fee_bps,
            } => {
                assert_eq!(lower_px_q64, 1000);
                assert_eq!(upper_px_q64, 2000);
                assert_eq!(quote_notional_q64, 1_000_000);
                assert_eq!(curve_id, 0);
                assert_eq!(fee_bps, 30);
            }
            _ => panic!("Wrong variant"),
        }

        assert_eq!(offset, data.len());
    }

    #[test]
    fn test_deserialize_remove_all() {
        let data = [
            3, // Remove variant
            2, // ObAll selector
        ];

        let (intent, offset) = deserialize_liquidity_intent(&data).unwrap();

        match intent {
            LiquidityIntent::Remove { selector } => match selector {
                RemoveSel::ObAll => {}
                _ => panic!("Wrong selector"),
            },
            _ => panic!("Wrong variant"),
        }

        assert_eq!(offset, 2);
    }

    #[test]
    fn test_deserialize_remove_by_shares() {
        let mut data = vec![
            3, // Remove variant
            0, // AmmByShares selector
        ];
        data.extend_from_slice(&500u128.to_le_bytes());

        let (intent, offset) = deserialize_liquidity_intent(&data).unwrap();

        match intent {
            LiquidityIntent::Remove { selector } => match selector {
                RemoveSel::AmmByShares { shares } => {
                    assert_eq!(shares, 500);
                }
                _ => panic!("Wrong selector"),
            },
            _ => panic!("Wrong variant"),
        }

        assert_eq!(offset, data.len());
    }
}
