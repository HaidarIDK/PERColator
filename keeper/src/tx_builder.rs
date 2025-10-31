//! Transaction builder for liquidations

use anyhow::Result;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    transaction::Transaction,
};

/// Build liquidate_user instruction
///
/// This constructs the liquidate_user instruction that the keeper
/// will submit to liquidate undercollateralized portfolios.
///
/// Expected accounts:
/// 0. Portfolio (writable)
/// 1. Registry
/// 2. Vault (writable)
/// 3. Router authority PDA
/// 4..4+N. Oracle accounts (N = num_oracles)
/// 4+N..4+N+M. Slab accounts (M = num_slabs, writable)
/// 4+N+M..4+N+2M. Receipt PDAs (M = num_slabs, writable)
/// 4+N+2M..4+N+2M+K. AMM accounts (K = num_amms, writable)
///
/// Instruction data:
/// - num_oracles: u8 (1 byte)
/// - num_slabs: u8 (1 byte)
/// - num_amms: u8 (1 byte)
/// - is_preliq: u8 (1 byte)
/// - current_ts: u64 (8 bytes)
pub fn build_liquidate_instruction(
    router_program: &Pubkey,
    portfolio: &Pubkey,
    registry: &Pubkey,
    vault: &Pubkey,
    router_authority: &Pubkey,
    is_preliq: bool,
    current_ts: u64,
    oracle_accounts: &[Pubkey],
    slab_accounts: &[Pubkey],
    receipt_accounts: &[Pubkey],
    amm_accounts: &[Pubkey],
) -> Instruction {
    // Instruction discriminator for LiquidateUser (from RouterInstruction enum)
    let discriminator = 5u8;

    // Build instruction data (12 bytes total)
    let mut data = vec![discriminator];
    data.push(oracle_accounts.len() as u8);
    data.push(slab_accounts.len() as u8);
    data.push(amm_accounts.len() as u8);
    data.push(if is_preliq { 1 } else { 0 });
    data.extend_from_slice(&current_ts.to_le_bytes());

    // Build account metas
    let mut accounts = vec![
        AccountMeta::new(*portfolio, false),
        AccountMeta::new_readonly(*registry, false),
        AccountMeta::new(*vault, false),
        AccountMeta::new_readonly(*router_authority, false),
    ];

    // Add oracle accounts
    for oracle in oracle_accounts {
        accounts.push(AccountMeta::new_readonly(*oracle, false));
    }

    // Add slab accounts
    for slab in slab_accounts {
        accounts.push(AccountMeta::new(*slab, false));
    }

    // Add receipt PDAs
    for receipt in receipt_accounts {
        accounts.push(AccountMeta::new(*receipt, false));
    }

    // Add AMM accounts
    for amm in amm_accounts {
        accounts.push(AccountMeta::new(*amm, false));
    }

    Instruction {
        program_id: *router_program,
        accounts,
        data,
    }
}

/// Build transaction for liquidation
pub fn build_liquidation_transaction(
    router_program: &Pubkey,
    portfolio: &Pubkey,
    registry: &Pubkey,
    vault: &Pubkey,
    router_authority: &Pubkey,
    keeper: &Keypair,
    is_preliq: bool,
    current_ts: u64,
    oracle_accounts: &[Pubkey],
    slab_accounts: &[Pubkey],
    receipt_accounts: &[Pubkey],
    amm_accounts: &[Pubkey],
    recent_blockhash: solana_sdk::hash::Hash,
) -> Result<Transaction> {
    let instruction = build_liquidate_instruction(
        router_program,
        portfolio,
        registry,
        vault,
        router_authority,
        is_preliq,
        current_ts,
        oracle_accounts,
        slab_accounts,
        receipt_accounts,
        amm_accounts,
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&keeper.pubkey()),
        &[keeper],
        recent_blockhash,
    );

    Ok(transaction)
}

/// Derive router authority PDA
pub fn derive_router_authority(router_program: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"authority"], router_program)
}

/// Derive registry PDA
pub fn derive_registry(router_program: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"registry", router_program.as_ref()], router_program)
}

/// Derive vault PDA for a given mint
pub fn derive_vault(mint: &Pubkey, router_program: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"vault", mint.as_ref()], router_program)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_liquidate_instruction() {
        let router_program = Pubkey::new_unique();
        let portfolio = Pubkey::new_unique();
        let registry = Pubkey::new_unique();
        let vault = Pubkey::new_unique();
        let router_authority = Pubkey::new_unique();

        let ix = build_liquidate_instruction(
            &router_program,
            &portfolio,
            &registry,
            &vault,
            &router_authority,
            false,
            1234567890,
            &[],
            &[],
            &[],
            &[],
        );

        assert_eq!(ix.program_id, router_program);
        assert_eq!(ix.data[0], 5); // LiquidateUser discriminator
        assert_eq!(ix.data[1], 0); // num_oracles
        assert_eq!(ix.data[2], 0); // num_slabs
        assert_eq!(ix.data[3], 0); // num_amms
        assert_eq!(ix.data[4], 0); // is_preliq = false
        assert_eq!(ix.accounts.len(), 4); // Portfolio, registry, vault, router_authority
    }

    #[test]
    fn test_build_preliq_instruction() {
        let router_program = Pubkey::new_unique();
        let portfolio = Pubkey::new_unique();
        let registry = Pubkey::new_unique();
        let vault = Pubkey::new_unique();
        let router_authority = Pubkey::new_unique();

        let ix = build_liquidate_instruction(
            &router_program,
            &portfolio,
            &registry,
            &vault,
            &router_authority,
            true,
            1234567890,
            &[],
            &[],
            &[],
            &[],
        );

        assert_eq!(ix.data[4], 1); // is_preliq = true
    }

    #[test]
    fn test_build_instruction_with_accounts() {
        let router_program = Pubkey::new_unique();
        let portfolio = Pubkey::new_unique();
        let registry = Pubkey::new_unique();
        let vault = Pubkey::new_unique();
        let router_authority = Pubkey::new_unique();

        let oracle1 = Pubkey::new_unique();
        let oracle2 = Pubkey::new_unique();
        let slab1 = Pubkey::new_unique();
        let receipt1 = Pubkey::new_unique();
        let amm1 = Pubkey::new_unique();

        let ix = build_liquidate_instruction(
            &router_program,
            &portfolio,
            &registry,
            &vault,
            &router_authority,
            false,
            1234567890,
            &[oracle1, oracle2],
            &[slab1],
            &[receipt1],
            &[amm1],
        );

        assert_eq!(ix.data[1], 2); // num_oracles
        assert_eq!(ix.data[2], 1); // num_slabs
        assert_eq!(ix.data[3], 1); // num_amms
        // 4 base + 2 oracles + 1 slab + 1 receipt + 1 amm = 9
        assert_eq!(ix.accounts.len(), 9);
    }

    #[test]
    fn test_derive_router_authority() {
        let router_program = Pubkey::new_unique();
        let (authority, _bump) = derive_router_authority(&router_program);

        // Authority should be deterministic
        let (authority2, _bump2) = derive_router_authority(&router_program);
        assert_eq!(authority, authority2);
    }

    #[test]
    fn test_derive_registry() {
        let router_program = Pubkey::new_unique();
        let (registry, _bump) = derive_registry(&router_program);

        // Registry should be deterministic
        let (registry2, _bump2) = derive_registry(&router_program);
        assert_eq!(registry, registry2);
    }
}
