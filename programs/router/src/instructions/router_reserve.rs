//! Router Reserve Instruction
//!
//! Locks collateral from a portfolio's free collateral into an LP seat's
//! reserved amounts. This is the first step in providing liquidity.

use crate::state::{Portfolio, RouterLpSeat};
use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::Pubkey,
    ProgramResult,
};

/// Reserve collateral from portfolio into LP seat
///
/// # Arguments
/// * `portfolio_account` - Portfolio account info
/// * `portfolio` - Mutable reference to portfolio state
/// * `seat_account` - LP seat account info
/// * `seat` - Mutable reference to seat state
/// * `base_amount_q64` - Base asset amount to reserve (Q64 fixed-point)
/// * `quote_amount_q64` - Quote asset amount to reserve (Q64 fixed-point)
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(ProgramError)` on validation failure or insufficient collateral
pub fn process_router_reserve(
    portfolio_account: &AccountInfo,
    portfolio: &mut Portfolio,
    seat_account: &AccountInfo,
    seat: &mut RouterLpSeat,
    base_amount_q64: u128,
    quote_amount_q64: u128,
) -> ProgramResult {
    // Verify portfolio owns this seat
    if seat.portfolio != *portfolio_account.key() {
        return Err(ProgramError::InvalidAccountData);
    }

    // Verify seat is not frozen
    if seat.is_frozen() {
        return Err(ProgramError::InvalidAccountData);
    }

    // Check sufficient free collateral
    // free_collateral is i128, amounts are u128
    let total_needed_i128 = (base_amount_q64 as i128)
        .checked_add(quote_amount_q64 as i128)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    if portfolio.free_collateral < total_needed_i128 {
        return Err(ProgramError::InsufficientFunds);
    }

    // Reserve collateral in seat (with overflow protection)
    seat.reserve(base_amount_q64, quote_amount_q64)
        .map_err(|_| ProgramError::ArithmeticOverflow)?;

    // Deduct from portfolio free collateral
    portfolio.free_collateral = portfolio
        .free_collateral
        .checked_sub(total_needed_i128)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    Ok(())
}

#[cfg(disabled_test)] // TODO: Update tests for new Portfolio and AccountInfo APIs
mod tests {
    use super::*;
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
    fn test_reserve_success() {
        let portfolio_key = Pubkey::from([1; 32]);
        let mut portfolio_lamports = 0;
        let mut portfolio_data = vec![0u8; 256];
        let portfolio_account = create_test_account_info(
            &portfolio_key,
            &mut portfolio_lamports,
            &mut portfolio_data,
        );

        let mut portfolio = unsafe { core::mem::zeroed::<Portfolio>() };
        portfolio.router_id = Pubkey::default();
        portfolio.user = Pubkey::default();
        portfolio.free_collateral = 10000;
        portfolio.bump = 255;

        let seat_key = Pubkey::from([2; 32]);
        let mut seat_lamports = 0;
        let mut seat_data = vec![0u8; 256];
        let seat_account = create_test_account_info(&seat_key, &mut seat_lamports, &mut seat_data);

        let mut seat = unsafe { core::mem::zeroed::<RouterLpSeat>() };
        seat.initialize_in_place(
            Pubkey::default(),
            Pubkey::default(),
            portfolio_key,
            0,
            255,
        );

        let result = process_router_reserve(
            &portfolio_account,
            &mut portfolio,
            &seat_account,
            &mut seat,
            3000,
            2000,
        );

        assert!(result.is_ok());
        assert_eq!(portfolio.free_collateral, 5000); // 10000 - 3000 - 2000
        assert_eq!(seat.reserved_base_q64, 3000);
        assert_eq!(seat.reserved_quote_q64, 2000);
    }

    #[test]
    fn test_reserve_insufficient_collateral() {
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
            free_collateral: 1000,
            locked_collateral: 0,
            realized_pnl: 0,
            unrealized_pnl: 0,
            total_deposits: 1000,
            total_withdrawals: 0,
            bump: 255,
            _padding: [0; 5],
        };

        let seat_key = Pubkey::from([2; 32]);
        let mut seat_lamports = 0;
        let mut seat_data = vec![0u8; 256];
        let seat_account = create_test_account_info(&seat_key, &mut seat_lamports, &mut seat_data);

        let mut seat = unsafe { core::mem::zeroed::<RouterLpSeat>() };
        seat.initialize_in_place(
            Pubkey::default(),
            Pubkey::default(),
            portfolio_key,
            0,
            255,
        );

        let result = process_router_reserve(
            &portfolio_account,
            &mut portfolio,
            &seat_account,
            &mut seat,
            5000,
            5000,
        );

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ProgramError::InsufficientFunds);
    }

    #[test]
    fn test_reserve_wrong_portfolio() {
        let portfolio_key = Pubkey::from([1; 32]);
        let mut portfolio_lamports = 0;
        let mut portfolio_data = vec![0u8; 256];
        let portfolio_account = create_test_account_info(
            &portfolio_key,
            &mut portfolio_lamports,
            &mut portfolio_data,
        );

        let mut portfolio = unsafe { core::mem::zeroed::<Portfolio>() };
        portfolio.router_id = Pubkey::default();
        portfolio.user = Pubkey::default();
        portfolio.free_collateral = 10000;
        portfolio.bump = 255;

        let seat_key = Pubkey::from([2; 32]);
        let mut seat_lamports = 0;
        let mut seat_data = vec![0u8; 256];
        let seat_account = create_test_account_info(&seat_key, &mut seat_lamports, &mut seat_data);

        let mut seat = unsafe { core::mem::zeroed::<RouterLpSeat>() };
        seat.initialize_in_place(
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::from([99; 32]), // Different portfolio
            0,
            255,
        );

        let result = process_router_reserve(
            &portfolio_account,
            &mut portfolio,
            &seat_account,
            &mut seat,
            1000,
            1000,
        );

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ProgramError::InvalidAccountData);
    }

    #[test]
    fn test_reserve_frozen_seat() {
        let portfolio_key = Pubkey::from([1; 32]);
        let mut portfolio_lamports = 0;
        let mut portfolio_data = vec![0u8; 256];
        let portfolio_account = create_test_account_info(
            &portfolio_key,
            &mut portfolio_lamports,
            &mut portfolio_data,
        );

        let mut portfolio = unsafe { core::mem::zeroed::<Portfolio>() };
        portfolio.router_id = Pubkey::default();
        portfolio.user = Pubkey::default();
        portfolio.free_collateral = 10000;
        portfolio.bump = 255;

        let seat_key = Pubkey::from([2; 32]);
        let mut seat_lamports = 0;
        let mut seat_data = vec![0u8; 256];
        let seat_account = create_test_account_info(&seat_key, &mut seat_lamports, &mut seat_data);

        let mut seat = unsafe { core::mem::zeroed::<RouterLpSeat>() };
        seat.initialize_in_place(
            Pubkey::default(),
            Pubkey::default(),
            portfolio_key,
            0,
            255,
        );
        seat.freeze();

        let result = process_router_reserve(
            &portfolio_account,
            &mut portfolio,
            &seat_account,
            &mut seat,
            1000,
            1000,
        );

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ProgramError::InvalidAccountData);
    }
}
