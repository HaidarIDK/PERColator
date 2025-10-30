//! Vault account for holding collateral

use pinocchio::pubkey::Pubkey;

/// Vault account storing collateral for a specific mint
/// PDA: ["vault", router_id, mint]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Vault {
    /// Router program ID
    pub router_id: Pubkey,
    /// Mint pubkey
    pub mint: Pubkey,
    /// Token account holding the funds
    pub token_account: Pubkey,
    /// Total balance
    pub balance: u128,
    /// Total pledged to escrows
    pub total_pledged: u128,
    /// Bump seed
    pub bump: u8,
    /// Padding
    pub _padding: [u8; 7],
}

impl Vault {
    pub const LEN: usize = core::mem::size_of::<Self>();

    /// Get available balance (not pledged)
    pub fn available(&self) -> u128 {
        self.balance.saturating_sub(self.total_pledged)
    }

    /// Pledge amount to escrow
    pub fn pledge(&mut self, amount: u128) -> Result<(), ()> {
        if self.available() < amount {
            return Err(());
        }
        self.total_pledged = self.total_pledged.saturating_add(amount);
        Ok(())
    }

    /// Unpledge amount from escrow
    pub fn unpledge(&mut self, amount: u128) {
        self.total_pledged = self.total_pledged.saturating_sub(amount);
    }

    /// Deposit to vault
    pub fn deposit(&mut self, amount: u128) {
        self.balance = self.balance.saturating_add(amount);
    }

    /// Withdraw from vault
    pub fn withdraw(&mut self, amount: u128) -> Result<(), ()> {
        if self.available() < amount {
            return Err(());
        }
        self.balance = self.balance.saturating_sub(amount);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_pledge() {
        let mut vault = Vault {
            router_id: Pubkey::default(),
            mint: Pubkey::default(),
            token_account: Pubkey::default(),
            balance: 1000,
            total_pledged: 0,
            bump: 0,
            _padding: [0; 7],
        };

        assert_eq!(vault.available(), 1000);
        assert!(vault.pledge(500).is_ok());
        assert_eq!(vault.available(), 500);
        assert_eq!(vault.total_pledged, 500);

        assert!(vault.pledge(600).is_err());
        assert!(vault.pledge(500).is_ok());
        assert_eq!(vault.available(), 0);

        vault.unpledge(300);
        assert_eq!(vault.available(), 300);
    }
}
