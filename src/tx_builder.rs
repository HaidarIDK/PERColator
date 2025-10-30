//! Transaction builder for liquidations

use anyhow::{Context, Result};
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
pub fn build_liquidate_instruction(
    router_program: &Pubkey,
    portfolio: &Pubkey,
    registry: &Pubkey,
    vault: &Pubkey,
    router_authority: &Pubkey,
    keeper: &Pubkey,
    is_preliq: bool,
) -> Instruction {
    // Instruction discriminator for LiquidateUser
    let discriminator = 3u8;

    // Instruction data: discriminator + is_preliq
    let mut data = vec![discriminator];
    data.push(if is_preliq { 1 } else { 0 });

    // Build account metas
    let accounts = vec![
        AccountMeta::new(*portfolio, false),
        AccountMeta::new_readonly(*registry, false),
        AccountMeta::new(*vault, false),
        AccountMeta::new_readonly(*router_authority, false),
        AccountMeta::new_readonly(*keeper, true),
        // In production, would include oracle accounts, slab accounts, etc.
    ];

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
    recent_blockhash: solana_sdk::hash::Hash,
) -> Result<Transaction> {
    let instruction = build_liquidate_instruction(
        router_program,
        portfolio,
        registry,
        vault,
        router_authority,
        &keeper.pubkey(),
        is_preliq,
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&keeper.pubkey()),
        &[keeper],
        recent_blockhash,
    );

    Ok(transaction)
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
        let keeper = Pubkey::new_unique();

        let ix = build_liquidate_instruction(
            &router_program,
            &portfolio,
            &registry,
            &vault,
            &router_authority,
            &keeper,
            false,
        );

        assert_eq!(ix.program_id, router_program);
        assert_eq!(ix.data[0], 3); // LiquidateUser discriminator
        assert_eq!(ix.data[1], 0); // is_preliq = false
        assert_eq!(ix.accounts.len(), 5);
    }

    #[test]
    fn test_build_preliq_instruction() {
        let router_program = Pubkey::new_unique();
        let portfolio = Pubkey::new_unique();
        let registry = Pubkey::new_unique();
        let vault = Pubkey::new_unique();
        let router_authority = Pubkey::new_unique();
        let keeper = Pubkey::new_unique();

        let ix = build_liquidate_instruction(
            &router_program,
            &portfolio,
            &registry,
            &vault,
            &router_authority,
            &keeper,
            true,
        );

        assert_eq!(ix.data[1], 1); // is_preliq = true
    }
}
