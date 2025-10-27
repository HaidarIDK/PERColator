//! Router Liquidity Instruction
//!
//! Processes liquidity operations (add/remove/modify) by coordinating with
//! a matcher adapter via CPI. Updates seat exposure, LP shares, and venue PnL.

use crate::state::{Portfolio, RouterLpSeat, VenuePnl};
use adapter_core::{LiquidityIntent, LiquidityResult, RiskGuard};
use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::Pubkey,
    ProgramResult,
};

/// Process liquidity operation via matcher adapter
///
/// # Arguments
/// * `portfolio_account` - Portfolio account info
/// * `portfolio` - Mutable reference to portfolio state
/// * `seat_account` - LP seat account info
/// * `seat` - Mutable reference to seat state
/// * `venue_pnl_account` - Venue PnL account info
/// * `venue_pnl` - Mutable reference to venue PnL state
/// * `matcher_program` - Matcher adapter program account
/// * `guard` - Risk guard parameters (slippage, fees, oracle bounds)
/// * `intent` - Liquidity operation intent (add/remove/modify)
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(ProgramError)` on validation failure or matcher rejection
///
/// # Note
/// This instruction will invoke the matcher adapter via CPI to execute the
/// liquidity operation and return a normalized result. For now, this is
/// a simplified version that applies deltas directly.
pub fn process_router_liquidity(
    portfolio_account: &AccountInfo,
    portfolio: &mut Portfolio,
    seat_account: &AccountInfo,
    seat: &mut RouterLpSeat,
    venue_pnl_account: &AccountInfo,
    venue_pnl: &mut VenuePnl,
    _matcher_program: &AccountInfo,
    _guard: RiskGuard,
    _intent: LiquidityIntent,
) -> ProgramResult {
    // Verify portfolio owns this seat
    if seat.portfolio != *portfolio_account.key() {
        return Err(ProgramError::InvalidAccountData);
    }

    // Verify seat is not frozen
    if seat.is_frozen() {
        return Err(ProgramError::InvalidAccountData);
    }

    // Verify venue_pnl matches the seat's matcher
    if venue_pnl.matcher_state != seat.matcher_state {
        return Err(ProgramError::InvalidAccountData);
    }

    // DEFERRED: CPI to matcher adapter program (Phase 4 - Matcher Integration)
    //   This is intentionally placeholder until matcher adapters are implemented.
    //   Production implementation will:
    //     1. Build CPI instruction with guard + intent (Borsh-serialized)
    //     2. Invoke matcher program with seat + matcher-specific accounts
    //     3. Read LiquidityResult from return_data
    //   See docs/LP_ADAPTER_CPI_INTEGRATION.md Section 3 for full CPI flow

    // Placeholder result for testing router-side infrastructure
    let result = LiquidityResult {
        lp_shares_delta: 0,
        exposure_delta: adapter_core::Exposure {
            base_q64: 0,
            quote_q64: 0,
        },
        maker_fee_credits: 0,
        realized_pnl_delta: 0,
    };

    // Apply LP shares delta
    seat.lp_shares = apply_shares_delta(seat.lp_shares, result.lp_shares_delta)
        .map_err(|_| ProgramError::ArithmeticOverflow)?;

    // Apply exposure delta
    seat.exposure.base_q64 = seat
        .exposure
        .base_q64
        .checked_add(result.exposure_delta.base_q64)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    seat.exposure.quote_q64 = seat
        .exposure
        .quote_q64
        .checked_add(result.exposure_delta.quote_q64)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    // Apply venue PnL deltas
    // Note: venue_fees are tracked separately from LiquidityResult
    // For now, we pass 0 as placeholder - in production this would be calculated
    // based on the actual venue fee structure
    venue_pnl
        .apply_deltas(
            result.maker_fee_credits,
            0, // venue_fees_delta placeholder
            result.realized_pnl_delta,
        )
        .map_err(|_| ProgramError::ArithmeticOverflow)?;

    // Verify seat credit discipline (exposure within reserved limits)
    // This uses the haircut values from the seat's risk class
    // For now, using conservative 10% haircuts
    let haircut_base_bps = 1000; // 10%
    let haircut_quote_bps = 1000; // 10%

    if !seat.check_limits(haircut_base_bps, haircut_quote_bps) {
        return Err(ProgramError::Custom(0x1001)); // Seat credit limit exceeded
    }

    Ok(())
}

/// Apply shares delta to current shares
///
/// This is equivalent to `adapter_core::add_shares_checked` but operates
/// on u128 current value and i128 delta.
fn apply_shares_delta(current: u128, delta: i128) -> Result<u128, ()> {
    if delta >= 0 {
        current.checked_add(delta as u128).ok_or(())
    } else {
        let abs_delta = delta.abs() as u128;
        if current < abs_delta {
            return Err(());
        }
        Ok(current - abs_delta)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Exposure;
    use pinocchio::pubkey::Pubkey;

    fn create_test_account_info<'a>(
        key: &'a Pubkey,
        lamports: &'a mut u64,
        data: &'a mut [u8],
    ) -> AccountInfo<'a> {
        AccountInfo {
            key,
            is_signer: false,
            is_writable: true,
            lamports,
            data,
            owner: &Pubkey::default(),
            rent_epoch: 0,
            #[cfg(feature = "bpf-entrypoint")]
            executable: false,
        }
    }

    #[test]
    fn test_apply_shares_delta_positive() {
        assert_eq!(apply_shares_delta(100, 50), Ok(150));
        assert_eq!(apply_shares_delta(0, 100), Ok(100));
    }

    #[test]
    fn test_apply_shares_delta_negative() {
        assert_eq!(apply_shares_delta(100, -50), Ok(50));
        assert_eq!(apply_shares_delta(100, -100), Ok(0));
    }

    #[test]
    fn test_apply_shares_delta_underflow() {
        assert!(apply_shares_delta(50, -100).is_err());
        assert!(apply_shares_delta(0, -1).is_err());
    }

    #[test]
    fn test_apply_shares_delta_overflow() {
        assert!(apply_shares_delta(u128::MAX, 1).is_err());
    }

    #[test]
    fn test_liquidity_wrong_portfolio() {
        let portfolio_key = Pubkey::from([1; 32]);
        let mut portfolio_lamports = 0;
        let mut portfolio_data = vec![0u8; 256];
        let portfolio_account = create_test_account_info(
            &portfolio_key,
            &mut portfolio_lamports,
            &mut portfolio_data,
        );

        let mut portfolio = Portfolio {
            owner: Pubkey::default(),
            vault: Pubkey::default(),
            free_collateral: 10000,
            locked_collateral: 0,
            realized_pnl: 0,
            unrealized_pnl: 0,
            total_deposits: 10000,
            total_withdrawals: 0,
            bump: 255,
            _padding: [0; 5],
        };

        let seat_key = Pubkey::from([2; 32]);
        let mut seat_lamports = 0;
        let mut seat_data = vec![0u8; 256];
        let seat_account = create_test_account_info(&seat_key, &mut seat_lamports, &mut seat_data);

        let matcher_key = Pubkey::from([3; 32]);

        let mut seat = unsafe { core::mem::zeroed::<RouterLpSeat>() };
        seat.initialize_in_place(
            Pubkey::default(),
            matcher_key,
            Pubkey::from([99; 32]), // Different portfolio
            0,
            255,
        );

        let venue_pnl_key = Pubkey::from([4; 32]);
        let mut venue_pnl_lamports = 0;
        let mut venue_pnl_data = vec![0u8; 256];
        let venue_pnl_account = create_test_account_info(
            &venue_pnl_key,
            &mut venue_pnl_lamports,
            &mut venue_pnl_data,
        );

        let mut venue_pnl = unsafe { core::mem::zeroed::<VenuePnl>() };
        venue_pnl.initialize_in_place(Pubkey::default(), matcher_key, 255);

        let matcher_program_key = Pubkey::from([5; 32]);
        let mut matcher_lamports = 0;
        let mut matcher_data = vec![0u8; 256];
        let matcher_program = create_test_account_info(
            &matcher_program_key,
            &mut matcher_lamports,
            &mut matcher_data,
        );

        let guard = RiskGuard {
            max_slippage_bps: 100,
            max_fee_bps: 50,
            oracle_bound_bps: 200,
        };

        let intent = LiquidityIntent::Remove {
            selector: adapter_core::RemoveSel::ObAll,
        };

        let result = process_router_liquidity(
            &portfolio_account,
            &mut portfolio,
            &seat_account,
            &mut seat,
            &venue_pnl_account,
            &mut venue_pnl,
            &matcher_program,
            guard,
            intent,
        );

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ProgramError::InvalidAccountData);
    }

    #[test]
    fn test_liquidity_frozen_seat() {
        let portfolio_key = Pubkey::from([1; 32]);
        let mut portfolio_lamports = 0;
        let mut portfolio_data = vec![0u8; 256];
        let portfolio_account = create_test_account_info(
            &portfolio_key,
            &mut portfolio_lamports,
            &mut portfolio_data,
        );

        let mut portfolio = Portfolio {
            owner: Pubkey::default(),
            vault: Pubkey::default(),
            free_collateral: 10000,
            locked_collateral: 0,
            realized_pnl: 0,
            unrealized_pnl: 0,
            total_deposits: 10000,
            total_withdrawals: 0,
            bump: 255,
            _padding: [0; 5],
        };

        let seat_key = Pubkey::from([2; 32]);
        let mut seat_lamports = 0;
        let mut seat_data = vec![0u8; 256];
        let seat_account = create_test_account_info(&seat_key, &mut seat_lamports, &mut seat_data);

        let matcher_key = Pubkey::from([3; 32]);

        let mut seat = unsafe { core::mem::zeroed::<RouterLpSeat>() };
        seat.initialize_in_place(Pubkey::default(), matcher_key, portfolio_key, 0, 255);
        seat.freeze(); // Freeze the seat

        let venue_pnl_key = Pubkey::from([4; 32]);
        let mut venue_pnl_lamports = 0;
        let mut venue_pnl_data = vec![0u8; 256];
        let venue_pnl_account = create_test_account_info(
            &venue_pnl_key,
            &mut venue_pnl_lamports,
            &mut venue_pnl_data,
        );

        let mut venue_pnl = unsafe { core::mem::zeroed::<VenuePnl>() };
        venue_pnl.initialize_in_place(Pubkey::default(), matcher_key, 255);

        let matcher_program_key = Pubkey::from([5; 32]);
        let mut matcher_lamports = 0;
        let mut matcher_data = vec![0u8; 256];
        let matcher_program = create_test_account_info(
            &matcher_program_key,
            &mut matcher_lamports,
            &mut matcher_data,
        );

        let guard = RiskGuard {
            max_slippage_bps: 100,
            max_fee_bps: 50,
            oracle_bound_bps: 200,
        };

        let intent = LiquidityIntent::Remove {
            selector: adapter_core::RemoveSel::ObAll,
        };

        let result = process_router_liquidity(
            &portfolio_account,
            &mut portfolio,
            &seat_account,
            &mut seat,
            &venue_pnl_account,
            &mut venue_pnl,
            &matcher_program,
            guard,
            intent,
        );

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ProgramError::InvalidAccountData);
    }

    #[test]
    fn test_liquidity_mismatched_venue() {
        let portfolio_key = Pubkey::from([1; 32]);
        let mut portfolio_lamports = 0;
        let mut portfolio_data = vec![0u8; 256];
        let portfolio_account = create_test_account_info(
            &portfolio_key,
            &mut portfolio_lamports,
            &mut portfolio_data,
        );

        let mut portfolio = Portfolio {
            owner: Pubkey::default(),
            vault: Pubkey::default(),
            free_collateral: 10000,
            locked_collateral: 0,
            realized_pnl: 0,
            unrealized_pnl: 0,
            total_deposits: 10000,
            total_withdrawals: 0,
            bump: 255,
            _padding: [0; 5],
        };

        let seat_key = Pubkey::from([2; 32]);
        let mut seat_lamports = 0;
        let mut seat_data = vec![0u8; 256];
        let seat_account = create_test_account_info(&seat_key, &mut seat_lamports, &mut seat_data);

        let matcher_key = Pubkey::from([3; 32]);
        let wrong_matcher_key = Pubkey::from([99; 32]);

        let mut seat = unsafe { core::mem::zeroed::<RouterLpSeat>() };
        seat.initialize_in_place(Pubkey::default(), matcher_key, portfolio_key, 0, 255);

        let venue_pnl_key = Pubkey::from([4; 32]);
        let mut venue_pnl_lamports = 0;
        let mut venue_pnl_data = vec![0u8; 256];
        let venue_pnl_account = create_test_account_info(
            &venue_pnl_key,
            &mut venue_pnl_lamports,
            &mut venue_pnl_data,
        );

        let mut venue_pnl = unsafe { core::mem::zeroed::<VenuePnl>() };
        venue_pnl.initialize_in_place(Pubkey::default(), wrong_matcher_key, 255);

        let matcher_program_key = Pubkey::from([5; 32]);
        let mut matcher_lamports = 0;
        let mut matcher_data = vec![0u8; 256];
        let matcher_program = create_test_account_info(
            &matcher_program_key,
            &mut matcher_lamports,
            &mut matcher_data,
        );

        let guard = RiskGuard {
            max_slippage_bps: 100,
            max_fee_bps: 50,
            oracle_bound_bps: 200,
        };

        let intent = LiquidityIntent::Remove {
            selector: adapter_core::RemoveSel::ObAll,
        };

        let result = process_router_liquidity(
            &portfolio_account,
            &mut portfolio,
            &seat_account,
            &mut seat,
            &venue_pnl_account,
            &mut venue_pnl,
            &matcher_program,
            guard,
            intent,
        );

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ProgramError::InvalidAccountData);
    }
}
