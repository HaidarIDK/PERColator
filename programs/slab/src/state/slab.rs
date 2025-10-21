//! Main slab state structure

use super::header::SlabHeader;
use super::pools::Pool;
use percolator_common::*;

/// Main Slab state (~70 KB contiguous memory, ULTRA-CHEAP: ~0.5 SOL rent!)
/// Layout: Header + Pools + Data
#[repr(C)]
pub struct SlabState {
    /// Header with metadata
    pub header: SlabHeader,

    /// Account pool
    pub accounts: [AccountState; MAX_ACCOUNTS],

    /// Instrument pool (small, fixed size)
    pub instruments: [Instrument; MAX_INSTRUMENTS],
    pub instrument_count: u16,

    /// DLP bitset/list
    pub dlp_accounts: [u32; MAX_DLP],

    /// Order pool
    pub orders: Pool<Order, MAX_ORDERS>,

    /// Position pool
    pub positions: Pool<Position, MAX_POSITIONS>,

    /// Reservation pool
    pub reservations: Pool<Reservation, MAX_RESERVATIONS>,

    /// Slice pool
    pub slices: Pool<Slice, MAX_SLICES>,

    /// Trade ring buffer
    pub trades: [Trade; MAX_TRADES],
    pub trade_head: u32,
    pub trade_count: u32,

    /// Aggressor ledger pool (shared, not per account)
    pub aggressor_ledger: Pool<AggressorEntry, MAX_AGGRESSOR_ENTRIES>,
}

impl SlabState {
    /// Get instrument by index
    pub fn get_instrument(&self, idx: u16) -> Option<&Instrument> {
        if idx < self.instrument_count {
            Some(&self.instruments[idx as usize])
        } else {
            None
        }
    }

    /// Get mutable instrument by index
    pub fn get_instrument_mut(&mut self, idx: u16) -> Option<&mut Instrument> {
        if idx < self.instrument_count {
            Some(&mut self.instruments[idx as usize])
        } else {
            None
        }
    }

    /// Add a new instrument
    pub fn add_instrument(&mut self, instrument: Instrument) -> Result<u16, ()> {
        if (self.instrument_count as usize) >= MAX_INSTRUMENTS {
            return Err(());
        }

        let idx = self.instrument_count;
        self.instruments[idx as usize] = instrument;
        self.instrument_count += 1;

        Ok(idx)
    }

    /// Record trade in ring buffer
    pub fn record_trade(&mut self, trade: Trade) {
        let idx = self.trade_head as usize;
        self.trades[idx] = trade;
        self.trade_head = (self.trade_head + 1) % (MAX_TRADES as u32);
        if (self.trade_count as usize) < MAX_TRADES {
            self.trade_count += 1;
        }
    }

    /// Check if account is DLP
    pub fn is_dlp(&self, account_idx: u32) -> bool {
        for i in 0..self.header.dlp_count as usize {
            if self.dlp_accounts[i] == account_idx {
                return true;
            }
        }
        false
    }

    /// Add DLP account
    pub fn add_dlp(&mut self, account_idx: u32) -> Result<(), ()> {
        if (self.header.dlp_count as usize) >= MAX_DLP {
            return Err(());
        }

        // Check if already exists
        if self.is_dlp(account_idx) {
            return Ok(());
        }

        self.dlp_accounts[self.header.dlp_count as usize] = account_idx;
        self.header.dlp_count += 1;
        Ok(())
    }

    /// Get account by index
    pub fn get_account(&self, idx: u32) -> Option<&AccountState> {
        if (idx as usize) < MAX_ACCOUNTS && self.accounts[idx as usize].active {
            Some(&self.accounts[idx as usize])
        } else {
            None
        }
    }

    /// Get mutable account by index
    pub fn get_account_mut(&mut self, idx: u32) -> Option<&mut AccountState> {
        if (idx as usize) < MAX_ACCOUNTS && self.accounts[idx as usize].active {
            Some(&mut self.accounts[idx as usize])
        } else {
            None
        }
    }

    /// Find or create account
    pub fn find_or_create_account(&mut self, pubkey: &pinocchio::pubkey::Pubkey) -> Result<u32, ()> {
        // First try to find existing
        for i in 0..MAX_ACCOUNTS {
            if self.accounts[i].active && &self.accounts[i].key == pubkey {
                return Ok(i as u32);
            }
        }

        // Find first inactive slot
        for i in 0..MAX_ACCOUNTS {
            if !self.accounts[i].active {
                self.accounts[i] = AccountState {
                    key: *pubkey,
                    cash: 0,
                    im: 0,
                    mm: 0,
                    position_head: u32::MAX,
                    index: i as u32,
                    active: true,
                    _padding: [0; 7],
                };
                return Ok(i as u32);
            }
        }

        Err(())
    }
}

// Size validation
const _: () = {
    const SLAB_SIZE: usize = core::mem::size_of::<SlabState>();
    const MAX_SIZE: usize = 128 * 1024; // 128 KB

    // This will fail to compile if SlabState exceeds 128 KB
    if SLAB_SIZE > MAX_SIZE {
        panic!("SlabState exceeds 128 KB limit");
    }
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slab_size() {
        let size = core::mem::size_of::<SlabState>();
        // Size should be ~50-80 KB (ULTRA-cheap: ~0.5 SOL rent!)
        assert!(size > 40_000, "Size too small: {} bytes", size);
        assert!(size < 100_000, "Size too large: {} bytes", size);
    }
}
