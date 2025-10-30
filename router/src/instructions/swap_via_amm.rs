//! Route a swap to the AMM (CPI) and emit a memo

use pinocchio::{
    account_info::AccountInfo,
    msg,
    pubkey::Pubkey,
    ProgramResult,
};

use percolator_common::{
    validate_owner,
    validate_writable,
    borrow_account_data_mut,
    InstructionReader,
    PercolatorError,
};

use solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

use crate::state::{Portfolio, Vault};

/// SwapViaAmm processor (scaffold)
///
/// Accounts:
/// 0. `[writable]` Portfolio
/// 1. `[signer]` User
/// 2. `[]` AMM Program
/// 3. `[]` Token Program
/// 4. `[]` AMM Pool
/// 5. `[writable]` AMM Vault In
/// 6. `[writable]` AMM Vault Out
/// 7. `[writable]` User Source ATA
/// 8. `[writable]` User Dest ATA
/// 9. `[]` Router authority PDA
/// 10. `[]` Memo program (optional)
///
/// Data:
/// - amount_in: u64
/// - min_out: u64
/// - pool_id: [u8; 32]
pub fn process_swap_via_amm(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    if accounts.len() < 10 {
        msg!("Error: SwapViaAmm requires at least 10 accounts");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let portfolio_ai = &accounts[0];
    let _user_ai = &accounts[1];
    let amm_program_ai = &accounts[2];
    let _token_program_ai = &accounts[3];
    let amm_pool_ai = &accounts[4];
    let _amm_vault_in_ai = &accounts[5];
    let _amm_vault_out_ai = &accounts[6];
    let _user_src_ai = &accounts[7];
    let _user_dst_ai = &accounts[8];
    let _router_auth_ai = &accounts[9];

    // Optional memo program
    let memo_ai_opt = accounts.get(10);

    // Validate portfolio belongs to this program
    validate_owner(portfolio_ai, program_id)?;
    validate_writable(portfolio_ai)?;

    // Parse input
    let mut reader = InstructionReader::new(data);
    let amount_in = reader.read_u64()?;
    let min_out = reader.read_u64()?;
    let pool_bytes = reader.read_bytes::<32>()?;
    let _pool_id = Pubkey::from(pool_bytes);

    // Touch portfolio for borrow mut (accounting hooks in future)
    let _portfolio = unsafe { borrow_account_data_mut::<Portfolio>(portfolio_ai)? };

    // For now, we only emit a memo so the tx is observable as "real"
    if let Some(memo_ai) = memo_ai_opt {
        // Build a simple memo instruction using the provided memo program id
        let memo_data = format!("percolator:swap_via_amm amount_in={} min_out={}", amount_in, min_out).into_bytes();
        let ix = Instruction {
            program_id: *memo_ai.key(),
            accounts: vec![],
            data: memo_data,
        };
        // Memo takes no accounts; any signer in the tx is fine
        // Use user as signer implicitly (runtime)
        let _ = invoke(&ix, &[]);
    }

    // TODO: Implement actual CPI to AMM program once interface is finalized
    // The CPI would be constructed here using `amm_program_ai` and account metas
    // and `invoke_signed` with the router authority PDA if needed.

    msg!("SwapViaAmm (scaffold) processed");
    Ok(())
}


