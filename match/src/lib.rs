#![no_std]

extern crate alloc;

pub mod passive_lp_matcher;

pub use passive_lp_matcher::*;

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
};

// =============================================================================
// Context Account Layout
// =============================================================================
// Bytes 0-63:  Matcher return data (64 bytes, written on each call) - ABI required
// Bytes 64-95: Stored LP PDA pubkey (32 bytes, set on init, verified on calls)
// Total minimum: 96 bytes (percolator requires 320 bytes minimum)

/// Offset where matcher return is written (must be 0 per ABI)
pub const CTX_RETURN_OFFSET: usize = 0;
/// Offset where LP PDA is stored (after return data)
pub const CTX_LP_PDA_OFFSET: usize = MATCHER_RETURN_LEN; // 64
/// Length of LP PDA (32 bytes for Pubkey)
pub const CTX_LP_PDA_LEN: usize = 32;
/// Minimum context account size
pub const CTX_MIN_LEN: usize = CTX_LP_PDA_OFFSET + CTX_LP_PDA_LEN; // 96 bytes

// =============================================================================
// Instruction Tags
// =============================================================================

/// Matcher call instruction tag (from percolator CPI)
pub const MATCHER_CALL_TAG: u8 = 0;
/// Initialize instruction tag (stores LP PDA)
pub const MATCHER_INIT_TAG: u8 = 1;

// =============================================================================
// Matcher Call Layout (67 bytes) - Tag 0
// =============================================================================
/// Offset  Field               Type     Size
/// 0       tag                 u8       1      Always 0
/// 1-9     req_id              u64      8
/// 9-11    lp_idx              u16      2
/// 11-19   lp_account_id       u64      8
/// 19-27   oracle_price_e6     u64      8
/// 27-43   req_size            i128     16
/// 43-67   reserved            [u8;24]  24
pub const MATCHER_CALL_LEN: usize = 67;

// =============================================================================
// Matcher Return Layout (64 bytes)
// =============================================================================

pub const MATCHER_RETURN_LEN: usize = 64;
pub const FLAG_VALID: u32 = 1;
pub const FLAG_PARTIAL_OK: u32 = 2;
pub const FLAG_REJECTED: u32 = 4;
pub const MATCHER_ABI_VERSION: u32 = 1;

/// Matcher return structure written to context account
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct MatcherReturn {
    pub abi_version: u32,
    pub flags: u32,
    pub exec_price_e6: u64,
    pub exec_size: i128,
    pub req_id: u64,
    pub lp_account_id: u64,
    pub oracle_price_e6: u64,
    pub reserved: u64,
}

impl MatcherReturn {
    /// Write to context account data at offset 0 (ABI required)
    pub fn write_to(&self, data: &mut [u8]) -> Result<(), ProgramError> {
        if data.len() < MATCHER_RETURN_LEN {
            return Err(ProgramError::AccountDataTooSmall);
        }
        data[0..4].copy_from_slice(&self.abi_version.to_le_bytes());
        data[4..8].copy_from_slice(&self.flags.to_le_bytes());
        data[8..16].copy_from_slice(&self.exec_price_e6.to_le_bytes());
        data[16..32].copy_from_slice(&self.exec_size.to_le_bytes());
        data[32..40].copy_from_slice(&self.req_id.to_le_bytes());
        data[40..48].copy_from_slice(&self.lp_account_id.to_le_bytes());
        data[48..56].copy_from_slice(&self.oracle_price_e6.to_le_bytes());
        data[56..64].copy_from_slice(&self.reserved.to_le_bytes());
        Ok(())
    }

    pub fn rejected(req_id: u64, lp_account_id: u64, oracle_price_e6: u64) -> Self {
        Self {
            abi_version: MATCHER_ABI_VERSION,
            flags: FLAG_VALID | FLAG_REJECTED,
            exec_price_e6: 1, // Non-zero to pass validation
            exec_size: 0,
            req_id,
            lp_account_id,
            oracle_price_e6,
            reserved: 0,
        }
    }

    pub fn filled(
        exec_price: u64,
        exec_size: i128,
        req_id: u64,
        lp_account_id: u64,
        oracle_price_e6: u64,
    ) -> Self {
        Self {
            abi_version: MATCHER_ABI_VERSION,
            flags: FLAG_VALID,
            exec_price_e6: exec_price,
            exec_size,
            req_id,
            lp_account_id,
            oracle_price_e6,
            reserved: 0,
        }
    }

    pub fn zero_fill(req_id: u64, lp_account_id: u64, oracle_price_e6: u64) -> Self {
        Self {
            abi_version: MATCHER_ABI_VERSION,
            flags: FLAG_VALID | FLAG_PARTIAL_OK,
            exec_price_e6: 1, // Non-zero to pass validation
            exec_size: 0,
            req_id,
            lp_account_id,
            oracle_price_e6,
            reserved: 0,
        }
    }
}

/// Parsed matcher call from instruction data
#[derive(Clone, Copy, Debug)]
pub struct MatcherCall {
    pub req_id: u64,
    pub lp_idx: u16,
    pub lp_account_id: u64,
    pub oracle_price_e6: u64,
    pub req_size: i128,
}

impl MatcherCall {
    pub fn parse(data: &[u8]) -> Result<Self, ProgramError> {
        if data.len() < MATCHER_CALL_LEN {
            return Err(ProgramError::InvalidInstructionData);
        }
        if data[0] != MATCHER_CALL_TAG {
            return Err(ProgramError::InvalidInstructionData);
        }

        let req_id = u64::from_le_bytes(data[1..9].try_into().unwrap());
        let lp_idx = u16::from_le_bytes(data[9..11].try_into().unwrap());
        let lp_account_id = u64::from_le_bytes(data[11..19].try_into().unwrap());
        let oracle_price_e6 = u64::from_le_bytes(data[19..27].try_into().unwrap());
        let req_size = i128::from_le_bytes(data[27..43].try_into().unwrap());

        // Verify reserved bytes are zero
        for &b in &data[43..67] {
            if b != 0 {
                return Err(ProgramError::InvalidInstructionData);
            }
        }

        Ok(Self {
            req_id,
            lp_idx,
            lp_account_id,
            oracle_price_e6,
            req_size,
        })
    }
}

// =============================================================================
// Context Account Helpers
// =============================================================================

/// Read stored LP PDA from context account
fn read_lp_pda(data: &[u8]) -> Result<Pubkey, ProgramError> {
    if data.len() < CTX_MIN_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let bytes: [u8; 32] = data[CTX_LP_PDA_OFFSET..CTX_LP_PDA_OFFSET + CTX_LP_PDA_LEN]
        .try_into()
        .unwrap();
    Ok(Pubkey::new_from_array(bytes))
}

/// Write LP PDA to context account
fn write_lp_pda(data: &mut [u8], pda: &Pubkey) -> Result<(), ProgramError> {
    if data.len() < CTX_MIN_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    data[CTX_LP_PDA_OFFSET..CTX_LP_PDA_OFFSET + CTX_LP_PDA_LEN].copy_from_slice(pda.as_ref());
    Ok(())
}

/// Check if context is initialized (LP PDA is non-zero)
fn is_initialized(data: &[u8]) -> bool {
    if data.len() < CTX_MIN_LEN {
        return false;
    }
    // Check if any byte in LP PDA slot is non-zero
    data[CTX_LP_PDA_OFFSET..CTX_LP_PDA_OFFSET + CTX_LP_PDA_LEN]
        .iter()
        .any(|&b| b != 0)
}

// =============================================================================
// Instruction Processing
// =============================================================================

/// Process the matcher instruction
///
/// ## Instructions
///
/// ### Tag 0: Matcher Call (from percolator CPI)
/// Accounts:
/// 0. `[signer]` LP PDA (must match stored PDA)
/// 1. `[writable]` Matcher context account (owned by this program)
///
/// ### Tag 1: Initialize
/// Accounts:
/// 0. `[]` LP PDA (will be stored, no signature required)
/// 1. `[writable]` Matcher context account (owned by this program)
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if instruction_data.is_empty() {
        return Err(ProgramError::InvalidInstructionData);
    }

    match instruction_data[0] {
        MATCHER_CALL_TAG => process_matcher_call(program_id, accounts, instruction_data),
        MATCHER_INIT_TAG => process_init(program_id, accounts, instruction_data),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

/// Process Initialize instruction (Tag 1)
///
/// Stores the LP PDA in the context account. Does not require PDA signature.
/// Can only be called once (context must be uninitialized).
///
/// Instruction data: [tag: u8] (just the tag byte)
fn process_init(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let lp_pda = next_account_info(account_iter)?;
    let ctx_account = next_account_info(account_iter)?;

    // Verify context account is owned by this program
    if ctx_account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Verify context account is large enough
    let ctx_data = ctx_account.try_borrow_data()?;
    if ctx_data.len() < CTX_MIN_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }

    // Verify not already initialized
    if is_initialized(&ctx_data) {
        return Err(ProgramError::AccountAlreadyInitialized);
    }
    drop(ctx_data);

    // Store LP PDA
    let mut ctx_data = ctx_account.try_borrow_mut_data()?;
    write_lp_pda(&mut ctx_data, lp_pda.key)?;

    Ok(())
}

/// Process Matcher Call instruction (Tag 0)
///
/// Executes passive matching logic and writes result to context account.
/// If initialized, requires LP PDA to be a signer and match the stored PDA.
/// If uninitialized (LP PDA is zero), skips signature check.
fn process_matcher_call(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let lp_pda = next_account_info(account_iter)?;
    let ctx_account = next_account_info(account_iter)?;

    // Verify context account is owned by this program
    if ctx_account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Check if initialized and verify signature if so
    {
        let ctx_data = ctx_account.try_borrow_data()?;
        if is_initialized(&ctx_data) {
            // Initialized: require signature and match
            if !lp_pda.is_signer {
                return Err(ProgramError::MissingRequiredSignature);
            }
            let stored_pda = read_lp_pda(&ctx_data)?;
            if stored_pda != *lp_pda.key {
                return Err(ProgramError::InvalidAccountData);
            }
        }
        // Uninitialized: skip signature check (PDA is zero)
    }

    // Parse instruction
    let call = MatcherCall::parse(instruction_data)?;

    // Use default config (50 bps edge)
    let cfg = PassiveMatcherConfig::default();

    // For a stateless matcher, we don't track inventory across calls
    // The LP's actual position is tracked by percolator's RiskEngine
    let mut lp_state = PassiveLpState::default();

    let matcher = PassiveOracleBpsMatcher;
    let result = matcher.execute_match(
        &cfg,
        &mut lp_state,
        call.oracle_price_e6,
        call.req_size,
        None, // No limit price in CPI interface
    );

    let ret = match result.reason {
        Reason::Ok => {
            if result.exec.size == 0 {
                MatcherReturn::zero_fill(call.req_id, call.lp_account_id, call.oracle_price_e6)
            } else {
                MatcherReturn::filled(
                    result.exec.price,
                    result.exec.size,
                    call.req_id,
                    call.lp_account_id,
                    call.oracle_price_e6,
                )
            }
        }
        _ => MatcherReturn::rejected(call.req_id, call.lp_account_id, call.oracle_price_e6),
    };

    // Write result to context account
    let mut ctx_data = ctx_account.try_borrow_mut_data()?;
    ret.write_to(&mut ctx_data)?;

    Ok(())
}

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint {
    #[allow(unused_imports)]
    use alloc::format; // Required by entrypoint! macro in SBF builds
    use crate::process_instruction as processor;
    use solana_program::{
        account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, pubkey::Pubkey,
    };

    entrypoint!(process_instruction);

    fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        processor(program_id, accounts, instruction_data)
    }
}
