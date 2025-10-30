//! Account validation helpers for Solana programs
//!
//! Provides utilities for:
//! - Account ownership validation
//! - Signer and writable checks
//! - Safe account data deserialization
//! - Common account validation patterns

use crate::error::PercolatorError;
use pinocchio::{account_info::AccountInfo, pubkey::Pubkey};

/// Validate that an account is owned by the expected program
///
/// # Arguments
/// * `account` - The account to validate
/// * `expected_owner` - The expected program owner
///
/// # Returns
/// * `Ok(())` if the account is owned by the expected program
/// * `Err(PercolatorError::InvalidAccountOwner)` otherwise
#[inline]
pub fn validate_owner(account: &AccountInfo, expected_owner: &Pubkey) -> Result<(), PercolatorError> {
    if account.owner() != expected_owner {
        return Err(PercolatorError::InvalidAccountOwner);
    }
    Ok(())
}

/// Validate that an account is a signer
///
/// # Arguments
/// * `account` - The account to validate
///
/// # Returns
/// * `Ok(())` if the account is a signer
/// * `Err(PercolatorError::InvalidAccount)` otherwise
#[inline]
pub fn validate_signer(account: &AccountInfo) -> Result<(), PercolatorError> {
    if !account.is_signer() {
        return Err(PercolatorError::InvalidAccount);
    }
    Ok(())
}

/// Validate that an account is writable
///
/// # Arguments
/// * `account` - The account to validate
///
/// # Returns
/// * `Ok(())` if the account is writable
/// * `Err(PercolatorError::InvalidAccount)` otherwise
#[inline]
pub fn validate_writable(account: &AccountInfo) -> Result<(), PercolatorError> {
    if !account.is_writable() {
        return Err(PercolatorError::InvalidAccount);
    }
    Ok(())
}

/// Validate that an account has the expected key
///
/// # Arguments
/// * `account` - The account to validate
/// * `expected_key` - The expected pubkey
///
/// # Returns
/// * `Ok(())` if the account key matches
/// * `Err(PercolatorError::InvalidAccount)` otherwise
#[inline]
pub fn validate_key(account: &AccountInfo, expected_key: &Pubkey) -> Result<(), PercolatorError> {
    if account.key() != expected_key {
        return Err(PercolatorError::InvalidAccount);
    }
    Ok(())
}

/// Validate that an account is initialized (has non-zero data)
///
/// # Arguments
/// * `account` - The account to validate
///
/// # Returns
/// * `Ok(())` if the account appears initialized
/// * `Err(PercolatorError::InvalidAccount)` otherwise
#[inline]
pub fn validate_initialized(account: &AccountInfo) -> Result<(), PercolatorError> {
    let data = account.try_borrow_data().map_err(|_| PercolatorError::InvalidAccount)?;

    if data.is_empty() {
        return Err(PercolatorError::InvalidAccount);
    }

    // Check if the first bytes are non-zero (simple initialization check)
    if data[0] == 0 && data.len() > 1 && data[1] == 0 {
        return Err(PercolatorError::InvalidAccount);
    }

    Ok(())
}

/// Safely borrow account data as a reference to type T
///
/// # Safety
/// This function performs basic alignment and size checks but cannot fully
/// guarantee memory safety. The caller must ensure the account data represents
/// a valid instance of type T.
///
/// # Arguments
/// * `account` - The account whose data to borrow
///
/// # Returns
/// * `Ok(&T)` if the account data can be safely cast to &T
/// * `Err(PercolatorError)` if validation fails
pub unsafe fn borrow_account_data<T>(account: &AccountInfo) -> Result<&T, PercolatorError> {
    let data = account.try_borrow_data().map_err(|_| PercolatorError::InvalidAccount)?;

    // Check size
    if data.len() < core::mem::size_of::<T>() {
        return Err(PercolatorError::InvalidAccount);
    }

    // Check alignment
    let ptr = data.as_ptr();
    if (ptr as usize) % core::mem::align_of::<T>() != 0 {
        return Err(PercolatorError::InvalidAccount);
    }

    // SAFETY: Caller must ensure T is valid for this account
    Ok(&*(ptr as *const T))
}

/// Safely borrow account data as a mutable reference to type T
///
/// # Safety
/// This function performs basic alignment and size checks but cannot fully
/// guarantee memory safety. The caller must ensure the account data represents
/// a valid instance of type T.
///
/// # Arguments
/// * `account` - The account whose data to borrow mutably
///
/// # Returns
/// * `Ok(&mut T)` if the account data can be safely cast to &mut T
/// * `Err(PercolatorError)` if validation fails
pub unsafe fn borrow_account_data_mut<T>(account: &AccountInfo) -> Result<&mut T, PercolatorError> {
    let mut data = account.try_borrow_mut_data().map_err(|_| PercolatorError::InvalidAccount)?;

    // Check size
    if data.len() < core::mem::size_of::<T>() {
        return Err(PercolatorError::InvalidAccount);
    }

    // Check alignment
    let ptr = data.as_mut_ptr();
    if (ptr as usize) % core::mem::align_of::<T>() != 0 {
        return Err(PercolatorError::InvalidAccount);
    }

    // SAFETY: Caller must ensure T is valid for this account
    Ok(&mut *(ptr as *mut T))
}

/// Combined validation: owner, signer, and writable
///
/// # Arguments
/// * `account` - The account to validate
/// * `expected_owner` - The expected program owner
///
/// # Returns
/// * `Ok(())` if all validations pass
/// * `Err(PercolatorError)` if any validation fails
#[inline]
pub fn validate_account_full(
    account: &AccountInfo,
    expected_owner: &Pubkey,
    require_signer: bool,
    require_writable: bool,
) -> Result<(), PercolatorError> {
    validate_owner(account, expected_owner)?;

    if require_signer {
        validate_signer(account)?;
    }

    if require_writable {
        validate_writable(account)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    // Note: Full account validation tests require Solana runtime
    // These are placeholder tests for compilation

    #[test]
    fn test_size_checks() {
        use crate::types::*;

        // Verify sizes for our main types
        assert!(core::mem::size_of::<AccountState>() > 0);
        assert!(core::mem::size_of::<Order>() > 0);
        assert!(core::mem::size_of::<Position>() > 0);
    }
}
