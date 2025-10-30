//! Create and initialize a Portfolio PDA for a user

use pinocchio::{
    account_info::AccountInfo,
    msg,
    pubkey::Pubkey,
    ProgramResult,
};

use percolator_common::{
    borrow_account_data_mut,
    validate_writable,
    PercolatorError,
    InstructionReader,
};

use solana_program::{
    program::{invoke_signed, invoke},
    system_instruction,
    sysvar::rent::Rent,
    sysvar::Sysvar,
};

use crate::pda::{derive_portfolio_pda, PORTFOLIO_SEED};
use crate::state::Portfolio;

/// CreatePortfolio instruction processor
///
/// Accounts:
/// 0. `[writable]` Portfolio PDA (will be created if not exists)
/// 1. `[signer, writable]` Payer (funds rent-exemption)
/// 2. `[]` System Program
/// 3. `[]` User (owner of portfolio)
///
/// Data:
/// - user_pubkey: [u8; 32]
pub fn process_create_portfolio(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    if accounts.len() < 4 {
        msg!("Error: CreatePortfolio requires 4 accounts");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let portfolio_ai = &accounts[0];
    let payer_ai = &accounts[1];
    let system_program_ai = &accounts[2];
    let user_ai = &accounts[3];

    validate_writable(portfolio_ai)?;

    // Parse user pubkey from data
    let mut reader = InstructionReader::new(data);
    let user_bytes = reader.read_bytes::<32>()?;
    let user = Pubkey::from(user_bytes);

    if user_ai.key() != &user {
        msg!("Error: user account mismatch");
        return Err(PercolatorError::InvalidAccount.into());
    }

    // Derive expected PDA
    let (expected_pda, bump) = derive_portfolio_pda(&user, program_id);
    if portfolio_ai.key() != &expected_pda {
        msg!("Error: portfolio PDA does not match derived address");
        return Err(PercolatorError::InvalidAccount.into());
    }

    // If not initialized (lamports == 0), create and allocate
    if **portfolio_ai.lamports.borrow() == 0 {
        let rent = Rent::get()?;
        let required_lamports = rent.minimum_balance(Portfolio::LEN);

        let create_ix = system_instruction::create_account(
            payer_ai.key(),
            portfolio_ai.key(),
            required_lamports,
            Portfolio::LEN as u64,
            program_id,
        );

        // Seeds: ["portfolio", user]
        let seeds: &[&[u8]] = &[PORTFOLIO_SEED, user.as_ref(), &[bump]];
        // First, payer signs create_account; program needs to sign to assign owner to itself using invoke_signed
        invoke(&create_ix, &[payer_ai.clone(), portfolio_ai.clone(), system_program_ai.clone()])?;

        // No additional invoke_signed needed for create_account; owner already set to program_id above
    }

    // Initialize state in-place
    unsafe {
        let portfolio = borrow_account_data_mut::<Portfolio>(portfolio_ai)?;
        portfolio.initialize_in_place(*program_id, user, bump);
    }

    msg!("CreatePortfolio: initialized");
    Ok(())
}


