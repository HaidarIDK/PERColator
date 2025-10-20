//! Router program entrypoint

use pinocchio::{
    account_info::AccountInfo,
    entrypoint,
    msg,
    pubkey::Pubkey,
    ProgramResult,
};

use crate::instructions::RouterInstruction;
use crate::state::{Vault, Escrow, UserPortfolio, SlabRegistry};
use percolator_common::*;

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
        0 => RouterInstruction::Initialize,
        1 => RouterInstruction::Deposit,
        2 => RouterInstruction::Withdraw,
        3 => RouterInstruction::MultiReserve,
        4 => RouterInstruction::MultiCommit,
        5 => RouterInstruction::Liquidate,
        _ => {
            msg!("Error: Unknown instruction: {}", discriminator);
            return Err(PercolatorError::InvalidInstruction.into());
        }
    };

    // Dispatch to instruction handler
    match instruction {
        RouterInstruction::Initialize => {
            msg!("Instruction: Initialize");
            handle_initialize(program_id, accounts, &instruction_data[1..])
        }
        RouterInstruction::Deposit => {
            msg!("Instruction: Deposit");
            handle_deposit(program_id, accounts, &instruction_data[1..])
        }
        RouterInstruction::Withdraw => {
            msg!("Instruction: Withdraw");
            handle_withdraw(program_id, accounts, &instruction_data[1..])
        }
        RouterInstruction::MultiReserve => {
            msg!("Instruction: MultiReserve");
            handle_multi_reserve(program_id, accounts, &instruction_data[1..])
        }
        RouterInstruction::MultiCommit => {
            msg!("Instruction: MultiCommit");
            handle_multi_commit(program_id, accounts, &instruction_data[1..])
        }
        RouterInstruction::Liquidate => {
            msg!("Instruction: Liquidate");
            handle_liquidate(program_id, accounts, &instruction_data[1..])
        }
    }
}

/// Handle Initialize instruction
///
/// Expected accounts:
/// 0. `[writable]` Registry account
/// 1. `[signer]` Authority/payer
///
/// Instruction data:
/// - authority (Pubkey): Program authority (32 bytes)
fn handle_initialize(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 2 {
        msg!("Error: Initialize instruction requires at least 2 accounts");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let registry_account = &accounts[0];
    let authority_account = &accounts[1];

    validate_owner(registry_account, program_id)?;
    validate_writable(registry_account)?;
    validate_signer(authority_account)?;

    // Parse instruction data (32 bytes for authority pubkey)
    if data.len() < 32 {
        msg!("Error: Invalid Initialize instruction data length: {}", data.len());
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let mut offset = 0;
    let authority_bytes = read_bytes::<32>(data, &mut offset)?;
    let authority = Pubkey::from(authority_bytes);

    // Initialize registry
    let registry = unsafe { borrow_account_data_mut::<SlabRegistry>(registry_account)? };
    *registry = SlabRegistry::new(authority);

    msg!("Router initialized successfully");
    Ok(())
}

/// Handle Deposit instruction
///
/// Expected accounts:
/// 0. `[writable]` Vault account
/// 1. `[writable]` User token account (source)
/// 2. `[writable]` User portfolio account
/// 3. `[signer]` User authority
/// 4. `[]` Token program
/// 5. `[]` Vault authority PDA
///
/// Instruction data:
/// - mint (Pubkey): Token mint (32 bytes)
/// - amount (u128): Amount to deposit (16 bytes)
fn handle_deposit(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 4 {
        msg!("Error: Deposit instruction requires at least 4 accounts");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let vault_account = &accounts[0];
    let user_token_account = &accounts[1];
    let portfolio_account = &accounts[2];
    let user_authority = &accounts[3];

    validate_owner(vault_account, program_id)?;
    validate_writable(vault_account)?;
    validate_writable(user_token_account)?;
    validate_writable(portfolio_account)?;
    validate_signer(user_authority)?;

    // Parse instruction data (32 + 16 = 48 bytes)
    if data.len() < 48 {
        msg!("Error: Invalid Deposit instruction data length: {}", data.len());
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let mut offset = 0;
    let _mint_bytes = read_bytes::<32>(data, &mut offset)?;
    let amount = read_u128(data, &mut offset)?;

    // Get vault state
    let vault = unsafe { borrow_account_data_mut::<Vault>(vault_account)? };

    // Process deposit
    crate::instructions::process_deposit(vault, amount)?;

    // Update user portfolio (add to equity)
    let portfolio = unsafe { borrow_account_data_mut::<UserPortfolio>(portfolio_account)? };
    portfolio.equity = portfolio.equity.saturating_add(amount as i128);

    // TODO: Actual token transfer via CPI to SPL Token program
    // For now, just log the operation
    msg!("Deposit successful: amount={}, new_vault_balance={}", amount, vault.balance);

    Ok(())
}

/// Handle Withdraw instruction
///
/// Expected accounts:
/// 0. `[writable]` Vault account
/// 1. `[writable]` User token account (destination)
/// 2. `[writable]` User portfolio account
/// 3. `[signer]` User authority
/// 4. `[]` Token program
/// 5. `[]` Vault authority PDA
///
/// Instruction data:
/// - mint (Pubkey): Token mint (32 bytes)
/// - amount (u128): Amount to withdraw (16 bytes)
fn handle_withdraw(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 4 {
        msg!("Error: Withdraw instruction requires at least 4 accounts");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let vault_account = &accounts[0];
    let user_token_account = &accounts[1];
    let portfolio_account = &accounts[2];
    let user_authority = &accounts[3];

    validate_owner(vault_account, program_id)?;
    validate_writable(vault_account)?;
    validate_writable(user_token_account)?;
    validate_writable(portfolio_account)?;
    validate_signer(user_authority)?;

    // Parse instruction data (32 + 16 = 48 bytes)
    if data.len() < 48 {
        msg!("Error: Invalid Withdraw instruction data length: {}", data.len());
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let mut offset = 0;
    let _mint_bytes = read_bytes::<32>(data, &mut offset)?;
    let amount = read_u128(data, &mut offset)?;

    // Get portfolio and check free collateral
    let portfolio = unsafe { borrow_account_data_mut::<UserPortfolio>(portfolio_account)? };
    
    // Check if user has sufficient free collateral
    let free_collateral = portfolio.equity.saturating_sub(portfolio.im as i128);
    if free_collateral < amount as i128 {
        msg!("Error: Insufficient free collateral: free={}, requested={}", 
            free_collateral, amount);
        return Err(PercolatorError::InsufficientFunds.into());
    }

    // Get vault state and withdraw
    let vault = unsafe { borrow_account_data_mut::<Vault>(vault_account)? };
    crate::instructions::process_withdraw(vault, amount)?;

    // Update portfolio
    portfolio.equity = portfolio.equity.saturating_sub(amount as i128);

    // TODO: Actual token transfer via CPI to SPL Token program
    msg!("Withdraw successful: amount={}, new_vault_balance={}", amount, vault.balance);

    Ok(())
}

/// Handle MultiReserve instruction
///
/// This instruction coordinates reserve operations across multiple slabs
///
/// Expected accounts:
/// 0. `[writable]` User portfolio account
/// 1. `[signer]` User authority
/// 2..N. `[writable]` Escrow accounts (one per slab)
/// N+1..M. `[]` Slab programs to invoke
///
/// Instruction data:
/// - slab_count (u8): Number of slabs to reserve from
/// - For each slab:
///   - slab_id (Pubkey): Slab program ID (32 bytes)
///   - instrument_idx (u16): Instrument index
///   - side (u8): 0 = Buy, 1 = Sell
///   - qty (u64): Quantity to reserve
///   - limit_px (u64): Limit price
fn handle_multi_reserve(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 2 {
        msg!("Error: MultiReserve instruction requires at least 2 accounts");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let portfolio_account = &accounts[0];
    let user_authority = &accounts[1];

    validate_owner(portfolio_account, program_id)?;
    validate_writable(portfolio_account)?;
    validate_signer(user_authority)?;

    // Parse instruction data
    if data.is_empty() {
        msg!("Error: Invalid MultiReserve instruction data length");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let mut offset = 0;
    let slab_count = read_u8(data, &mut offset)?;

    msg!("MultiReserve: {} slabs", slab_count);

    // TODO: Implement actual multi-slab reserve coordination
    // For now, validate structure
    for i in 0..slab_count {
        if offset + 32 + 2 + 1 + 8 + 8 > data.len() {
            msg!("Error: Incomplete reserve data for slab {}", i);
            return Err(PercolatorError::InvalidInstruction.into());
        }

        let _slab_id_bytes = read_bytes::<32>(data, &mut offset)?;
        let instrument_idx = read_u16(data, &mut offset)?;
        let side = read_u8(data, &mut offset)?;
        let qty = read_u64(data, &mut offset)?;
        let limit_px = read_u64(data, &mut offset)?;

        msg!("Reserve {}: instrument={}, side={}, qty={}, limit_px={}", 
            i, instrument_idx, side, qty, limit_px);
    }

    msg!("MultiReserve validated - implementation pending");
    Ok(())
}

/// Handle MultiCommit instruction
///
/// This instruction coordinates commit operations across multiple slabs
///
/// Expected accounts:
/// 0. `[writable]` User portfolio account
/// 1. `[signer]` User authority
/// 2..N. `[writable]` Cap accounts (one per slab)
/// N+1..M. `[]` Slab programs to invoke
///
/// Instruction data:
/// - slab_count (u8): Number of slabs to commit to
/// - current_ts (u64): Current timestamp
/// - For each slab:
///   - hold_id (u64): Reservation identifier
fn handle_multi_commit(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 2 {
        msg!("Error: MultiCommit instruction requires at least 2 accounts");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let portfolio_account = &accounts[0];
    let user_authority = &accounts[1];

    validate_owner(portfolio_account, program_id)?;
    validate_writable(portfolio_account)?;
    validate_signer(user_authority)?;

    // Parse instruction data
    if data.len() < 9 {
        msg!("Error: Invalid MultiCommit instruction data length");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let mut offset = 0;
    let slab_count = read_u8(data, &mut offset)?;
    let current_ts = read_u64(data, &mut offset)?;

    msg!("MultiCommit: {} slabs at ts={}", slab_count, current_ts);

    // TODO: Implement actual multi-slab commit coordination
    // For now, validate structure
    for i in 0..slab_count {
        if offset + 8 > data.len() {
            msg!("Error: Incomplete commit data for slab {}", i);
            return Err(PercolatorError::InvalidInstruction.into());
        }

        let hold_id = read_u64(data, &mut offset)?;
        msg!("Commit {}: hold_id={}", i, hold_id);
    }

    msg!("MultiCommit validated - implementation pending");
    Ok(())
}

/// Handle Liquidate instruction
///
/// Coordinates liquidation of an underwater position across multiple slabs
///
/// Expected accounts:
/// 0. `[writable]` Liquidatee portfolio account
/// 1. `[writable]` Liquidator portfolio account
/// 2. `[signer]` Liquidator authority
/// 3..N. `[writable]` Slab accounts involved in liquidation
///
/// Instruction data:
/// - liquidatee (Pubkey): User to liquidate (32 bytes)
/// - max_debt (u128): Maximum debt to cover (16 bytes)
fn handle_liquidate(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 3 {
        msg!("Error: Liquidate instruction requires at least 3 accounts");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let liquidatee_portfolio = &accounts[0];
    let liquidator_portfolio = &accounts[1];
    let liquidator_authority = &accounts[2];

    validate_owner(liquidatee_portfolio, program_id)?;
    validate_writable(liquidatee_portfolio)?;
    validate_owner(liquidator_portfolio, program_id)?;
    validate_writable(liquidator_portfolio)?;
    validate_signer(liquidator_authority)?;

    // Parse instruction data (32 + 16 = 48 bytes)
    if data.len() < 48 {
        msg!("Error: Invalid Liquidate instruction data length: {}", data.len());
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let mut offset = 0;
    let liquidatee_bytes = read_bytes::<32>(data, &mut offset)?;
    let max_debt = read_u128(data, &mut offset)?;

    let liquidatee_pubkey = Pubkey::from(liquidatee_bytes);

    // Verify liquidatee account matches
    if liquidatee_portfolio.key() != &liquidatee_pubkey {
        msg!("Error: Liquidatee account mismatch");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    // Get liquidatee portfolio and check if underwater
    let portfolio = unsafe { borrow_account_data_mut::<UserPortfolio>(liquidatee_portfolio)? };
    
    // Check if account is eligible for liquidation (equity < maintenance margin)
    if portfolio.equity >= portfolio.mm as i128 {
        msg!("Error: Account is not underwater: equity={}, mm={}", 
            portfolio.equity, portfolio.mm);
        return Err(PercolatorError::NotLiquidatable.into());
    }

    msg!("Liquidation eligible: equity={}, mm={}, deficit={}, max_debt={}", 
        portfolio.equity, portfolio.mm, portfolio.mm as i128 - portfolio.equity, max_debt);

    // TODO: Implement actual liquidation logic
    // - Calculate deficit
    // - Close positions across slabs
    // - Reward liquidator
    // - Update both portfolios

    msg!("Liquidate validated - implementation pending");
    Ok(())
}
