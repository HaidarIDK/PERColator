//! Slab header with metadata and anti-toxicity params

use pinocchio::pubkey::Pubkey;

/// Slab header (at start of ~70 KB account, ULTRA-CHEAP!)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SlabHeader {
    /// Magic bytes for validation
    pub magic: [u8; 8],
    /// Version
    pub version: u16,
    /// Padding
    pub _padding: [u8; 6],
    /// Slab program ID
    pub program_id: Pubkey,
    /// LP owner pubkey
    pub lp_owner: Pubkey,
    /// Router program ID
    pub router_id: Pubkey,

    // Risk parameters
    /// Initial margin ratio (basis points)
    pub imr: u64,
    /// Maintenance margin ratio (basis points)
    pub mmr: u64,
    /// Maker fee (basis points, can be negative for rebate)
    pub maker_fee: i64,
    /// Taker fee (basis points)
    pub taker_fee: u64,

    // Anti-toxicity parameters
    /// Batch window duration (milliseconds)
    pub batch_ms: u64,
    /// Number of top levels to freeze against contra queue
    pub freeze_levels: u16,
    /// Kill band (basis points) - reject if price moved too much
    pub kill_band_bps: u64,
    /// Anti-sandwich fee factor (basis points)
    pub as_fee_k: u64,
    /// JIT penalty enabled
    pub jit_penalty_on: bool,
    /// Minimum time for maker rebate (milliseconds)
    pub maker_rebate_min_ms: u64,

    // DLP configuration
    /// Maximum number of DLP accounts
    pub dlp_max: u16,
    /// Current number of DLPs
    pub dlp_count: u16,

    // Pool sizes (for offset calculations)
    pub max_accounts: u32,
    pub max_instruments: u16,
    pub max_orders: u32,
    pub max_positions: u32,
    pub max_reservations: u32,
    pub max_slices: u32,
    pub max_trades: u32,
    pub max_aggressor_entries: u32,

    // State tracking
    /// Next order ID (monotonic)
    pub next_order_id: u64,
    /// Next hold ID (monotonic)
    pub next_hold_id: u64,
    /// Book sequence number (for staleness detection)
    pub book_seqno: u64,
    /// Current timestamp (updated at batch_open)
    pub current_ts: u64,

    /// Bump seed
    pub bump: u8,
    /// Padding
    pub _padding2: [u8; 7],
}

impl SlabHeader {
    pub const MAGIC: &'static [u8; 8] = b"PERCSLB1";
    pub const VERSION: u16 = 1;
    pub const LEN: usize = core::mem::size_of::<Self>();

    /// Initialize new slab header
    pub fn new(
        program_id: Pubkey,
        lp_owner: Pubkey,
        router_id: Pubkey,
        imr: u64,
        mmr: u64,
        maker_fee: i64,
        taker_fee: u64,
        batch_ms: u64,
        bump: u8,
    ) -> Self {
        Self {
            magic: *Self::MAGIC,
            version: Self::VERSION,
            _padding: [0; 6],
            program_id,
            lp_owner,
            router_id,
            imr,
            mmr,
            maker_fee,
            taker_fee,
            batch_ms,
            freeze_levels: 3,
            kill_band_bps: 100, // 1%
            as_fee_k: 50,       // 0.5%
            jit_penalty_on: true,
            maker_rebate_min_ms: 100,
            dlp_max: 100,
            dlp_count: 0,
            max_accounts: percolator_common::MAX_ACCOUNTS as u32,
            max_instruments: percolator_common::MAX_INSTRUMENTS as u16,
            max_orders: percolator_common::MAX_ORDERS as u32,
            max_positions: percolator_common::MAX_POSITIONS as u32,
            max_reservations: percolator_common::MAX_RESERVATIONS as u32,
            max_slices: percolator_common::MAX_SLICES as u32,
            max_trades: percolator_common::MAX_TRADES as u32,
            max_aggressor_entries: percolator_common::MAX_AGGRESSOR_ENTRIES as u32,
            next_order_id: 1,
            next_hold_id: 1,
            book_seqno: 0,
            current_ts: 0,
            bump,
            _padding2: [0; 7],
        }
    }

    /// Validate magic and version
    pub fn validate(&self) -> bool {
        &self.magic == Self::MAGIC && self.version == Self::VERSION
    }

    /// Increment and get next order ID
    pub fn next_order_id(&mut self) -> u64 {
        let id = self.next_order_id;
        self.next_order_id = self.next_order_id.wrapping_add(1);
        id
    }

    /// Increment and get next hold ID
    pub fn next_hold_id(&mut self) -> u64 {
        let id = self.next_hold_id;
        self.next_hold_id = self.next_hold_id.wrapping_add(1);
        id
    }

    /// Increment book sequence number
    pub fn increment_book_seqno(&mut self) -> u64 {
        self.book_seqno = self.book_seqno.wrapping_add(1);
        self.book_seqno
    }

    /// Update current timestamp
    pub fn update_timestamp(&mut self, ts: u64) {
        self.current_ts = ts;
    }

    /// Check if JIT penalty applies
    pub fn is_jit_order(&self, order_created_ms: u64, batch_open_ms: u64) -> bool {
        self.jit_penalty_on && order_created_ms >= batch_open_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_validation() {
        let header = SlabHeader::new(
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            500,
            250,
            -5,
            20,
            100,
            0,
        );

        assert!(header.validate());
        assert_eq!(header.next_order_id, 1);
        assert_eq!(header.next_hold_id, 1);
    }

    #[test]
    fn test_header_monotonic_ids() {
        let mut header = SlabHeader::new(
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            500,
            250,
            0,
            20,
            100,
            0,
        );

        assert_eq!(header.next_order_id(), 1);
        assert_eq!(header.next_order_id(), 2);
        assert_eq!(header.next_order_id(), 3);

        assert_eq!(header.next_hold_id(), 1);
        assert_eq!(header.next_hold_id(), 2);
    }
}
