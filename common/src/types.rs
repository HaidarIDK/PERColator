//! Common types shared between Router and Slab programs

use pinocchio::pubkey::Pubkey;

/// Maximum number of slabs in the registry
pub const MAX_SLABS: usize = 256;

/// Maximum number of instruments per slab
pub const MAX_INSTRUMENTS: usize = 32;

/// Maximum number of accounts per slab
pub const MAX_ACCOUNTS: usize = 5_000;

/// Maximum number of orders per slab
pub const MAX_ORDERS: usize = 30_000;

/// Maximum number of positions per slab
pub const MAX_POSITIONS: usize = 30_000;

/// Maximum number of reservations per slab
pub const MAX_RESERVATIONS: usize = 4_000;

/// Maximum number of slices per slab
pub const MAX_SLICES: usize = 16_000;

/// Maximum number of trades in ring buffer
pub const MAX_TRADES: usize = 10_000;

/// Maximum number of DLP accounts
pub const MAX_DLP: usize = 100;

/// Maximum TTL for capabilities (2 minutes in milliseconds)
pub const MAX_CAP_TTL_MS: u64 = 120_000;

/// Order side
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Side {
    #[default]
    Buy = 0,
    Sell = 1,
}

/// Time in force
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimeInForce {
    #[default]
    GTC = 0, // Good till cancel
    IOC = 1, // Immediate or cancel
    FOK = 2, // Fill or kill
}

/// Maker class
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MakerClass {
    #[default]
    REG = 0, // Regular - goes to pending queue
    DLP = 1, // Designated LP - posts immediately
}

/// Order state
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OrderState {
    #[default]
    LIVE = 0,    // Active in book
    PENDING = 1, // Waiting for promotion
}

/// Account state for tracking within slab
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AccountState {
    /// Account pubkey
    pub key: Pubkey,
    /// Local cash balance (signed)
    pub cash: i128,
    /// Initial margin requirement
    pub im: u128,
    /// Maintenance margin requirement
    pub mm: u128,
    /// Head of position linked list
    pub position_head: u32,
    /// Account index
    pub index: u32,
    /// Account active flag
    pub active: bool,
    /// Padding
    pub _padding: [u8; 7],
}

/// Instrument definition
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Instrument {
    /// Instrument symbol (8 bytes, e.g., "BTC-PERP")
    pub symbol: [u8; 8],
    /// Contract size (e.g., 0.001 BTC)
    pub contract_size: u64,
    /// Tick size (minimum price increment)
    pub tick: u64,
    /// Lot size (minimum quantity increment)
    pub lot: u64,
    /// Current index price (from oracle)
    pub index_price: u64,
    /// Current funding rate (basis points per hour)
    pub funding_rate: i64,
    /// Cumulative funding
    pub cum_funding: i128,
    /// Last funding timestamp
    pub last_funding_ts: u64,
    /// Bids book head
    pub bids_head: u32,
    /// Asks book head
    pub asks_head: u32,
    /// Pending bids head
    pub bids_pending_head: u32,
    /// Pending asks head
    pub asks_pending_head: u32,
    /// Current epoch
    pub epoch: u16,
    /// Instrument index
    pub index: u16,
    /// Batch open timestamp
    pub batch_open_ms: u64,
    /// Freeze until timestamp
    pub freeze_until_ms: u64,
}

/// Order in the book
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Order {
    /// Order ID (monotonic)
    pub order_id: u64,
    /// Account index
    pub account_idx: u32,
    /// Instrument index
    pub instrument_idx: u16,
    /// Order side
    pub side: Side,
    /// Time in force
    pub tif: TimeInForce,
    /// Maker class
    pub maker_class: MakerClass,
    /// Order state
    pub state: OrderState,
    /// Eligible epoch for promotion
    pub eligible_epoch: u16,
    /// Creation timestamp
    pub created_ms: u64,
    /// Price
    pub price: u64,
    /// Quantity
    pub qty: u64,
    /// Reserved quantity (locked for reservations)
    pub reserved_qty: u64,
    /// Original quantity
    pub qty_orig: u64,
    /// Next order in book
    pub next: u32,
    /// Previous order in book
    pub prev: u32,
    /// Next in freelist
    pub next_free: u32,
    /// Used flag
    pub used: bool,
    /// Padding
    pub _padding: [u8; 3],
}

/// Position
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Position {
    /// Account index
    pub account_idx: u32,
    /// Instrument index
    pub instrument_idx: u16,
    /// Padding
    pub _padding: u16,
    /// Position quantity (signed: positive = long, negative = short)
    pub qty: i64,
    /// Entry VWAP price
    pub entry_px: u64,
    /// Last funding snapshot
    pub last_funding: i128,
    /// Next position for this account
    pub next_in_account: u32,
    /// Position index in pool
    pub index: u32,
    /// Used flag
    pub used: bool,
    /// Padding
    pub _padding2: [u8; 7],
}

/// Slice in a reservation
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Slice {
    /// Order index being reserved
    pub order_idx: u32,
    /// Quantity reserved from this order
    pub qty: u64,
    /// Next slice in reservation
    pub next: u32,
    /// Slice index
    pub index: u32,
    /// Used flag
    pub used: bool,
    /// Padding
    pub _padding: [u8; 7],
}

/// Reservation hold
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Reservation {
    /// Unique hold ID
    pub hold_id: u64,
    /// Route ID from router
    pub route_id: u64,
    /// Account index
    pub account_idx: u32,
    /// Instrument index
    pub instrument_idx: u16,
    /// Side
    pub side: Side,
    /// Padding
    pub _padding: u8,
    /// Quantity to fill
    pub qty: u64,
    /// VWAP price of reserved slices
    pub vwap_px: u64,
    /// Worst price in reservation
    pub worst_px: u64,
    /// Maximum charge (fees + notional)
    pub max_charge: u128,
    /// Commitment hash for commit-reveal
    pub commitment_hash: [u8; 32],
    /// Salt for commitment
    pub salt: [u8; 16],
    /// Book sequence number at hold time
    pub book_seqno: u64,
    /// Expiry timestamp
    pub expiry_ms: u64,
    /// Head of slice linked list
    pub slice_head: u32,
    /// Reservation index
    pub index: u32,
    /// Used flag
    pub used: bool,
    /// Committed flag
    pub committed: bool,
    /// Padding
    pub _padding2: [u8; 6],
}

/// Trade record in ring buffer
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Trade {
    /// Timestamp
    pub ts: u64,
    /// Maker order ID
    pub order_id_maker: u64,
    /// Taker order ID / route ID
    pub order_id_taker: u64,
    /// Instrument index
    pub instrument_idx: u16,
    /// Side (from taker perspective)
    pub side: Side,
    /// Padding
    pub _padding: [u8; 5],
    /// Price
    pub price: u64,
    /// Quantity
    pub qty: u64,
    /// Optional hash for delayed reveal
    pub hash: [u8; 32],
    /// Reveal timestamp
    pub reveal_ms: u64,
}

/// Aggressor ledger entry for anti-sandwich
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct AggressorEntry {
    /// Account index
    pub account_idx: u32,
    /// Instrument index
    pub instrument_idx: u16,
    /// Current epoch
    pub epoch: u16,
    /// Buy quantity this batch
    pub buy_qty: u64,
    /// Buy notional this batch
    pub buy_notional: u128,
    /// Sell quantity this batch
    pub sell_qty: u64,
    /// Sell notional this batch
    pub sell_notional: u128,
    /// Used flag
    pub used: bool,
    /// Padding
    pub _padding: [u8; 7],
}

/// Maximum aggressor ledger entries (shared pool, not per account-instrument)
pub const MAX_AGGRESSOR_ENTRIES: usize = 4_000;

// Size checks to ensure we're within 10 MB for slab
const _: () = {
    const fn check_size() {
        let total = 0
            + (MAX_ACCOUNTS * core::mem::size_of::<AccountState>())
            + (MAX_INSTRUMENTS * core::mem::size_of::<Instrument>())
            + (MAX_ORDERS * core::mem::size_of::<Order>())
            + (MAX_POSITIONS * core::mem::size_of::<Position>())
            + (MAX_RESERVATIONS * core::mem::size_of::<Reservation>())
            + (MAX_SLICES * core::mem::size_of::<Slice>())
            + (MAX_TRADES * core::mem::size_of::<Trade>())
            + (MAX_AGGRESSOR_ENTRIES * core::mem::size_of::<AggressorEntry>());

        // Should be under 10 MB
        const MAX_SLAB_SIZE: usize = 10 * 1024 * 1024;
        if total > MAX_SLAB_SIZE {
            panic!("Slab size exceeds 10 MB");
        }
    }
    check_size();
};
