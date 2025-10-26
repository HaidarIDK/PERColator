//! Orderbook implementation - v1 array-based orderbook
//!
//! This module implements a fixed-size array-based orderbook with price-time priority.
//! Orders are stored in sorted arrays (bids descending, asks ascending) with FIFO
//! ordering at the same price level.

use pinocchio::pubkey::Pubkey;

/// Maximum number of bid orders (adjusted for actual Order size of ~80 bytes)
pub const MAX_BIDS: usize = 19;

/// Maximum number of ask orders (adjusted for actual Order size of ~80 bytes)
pub const MAX_ASKS: usize = 19;

/// Order side
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Side {
    Buy = 0,
    Sell = 1,
}

/// Individual order in the orderbook (64 bytes)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Order {
    /// Unique order ID (monotonically increasing)
    pub order_id: u64,

    /// Owner's public key
    pub owner: Pubkey,  // 32 bytes

    /// Side: 0 = Buy, 1 = Sell
    pub side: u8,

    /// Limit price (1e6 scale, i.e., $100.00 = 100_000_000)
    pub price: i64,

    /// Remaining quantity (1e6 scale)
    pub qty: i64,

    /// Timestamp for FIFO ordering at same price
    pub timestamp: u64,

    /// Reserved for future use
    pub _reserved: [u8; 7],
}

impl Order {
    /// Create a new order
    pub fn new(
        order_id: u64,
        owner: Pubkey,
        side: Side,
        price: i64,
        qty: i64,
        timestamp: u64,
    ) -> Self {
        Self {
            order_id,
            owner,
            side: side as u8,
            price,
            qty,
            timestamp,
            _reserved: [0; 7],
        }
    }
}

impl Default for Order {
    fn default() -> Self {
        Self {
            order_id: 0,
            owner: Pubkey::default(),
            side: 0,
            price: 0,
            qty: 0,
            timestamp: 0,
            _reserved: [0; 7],
        }
    }
}

/// Book area with fixed-size arrays (~3KB)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BookArea {
    /// Next order ID (monotonic counter)
    pub next_order_id: u64,

    /// Number of active buy orders
    pub num_bids: u16,

    /// Number of active sell orders
    pub num_asks: u16,

    /// Reserved for alignment
    pub _reserved: [u8; 4],

    /// Buy orders (sorted descending by price, then FIFO by timestamp)
    pub bids: [Order; MAX_BIDS],

    /// Sell orders (sorted ascending by price, then FIFO by timestamp)
    pub asks: [Order; MAX_ASKS],
}

impl BookArea {
    /// Create a new empty orderbook
    pub fn new() -> Self {
        Self {
            next_order_id: 1,  // Start at 1 (0 reserved for invalid)
            num_bids: 0,
            num_asks: 0,
            _reserved: [0; 4],
            bids: [Order::default(); MAX_BIDS],
            asks: [Order::default(); MAX_ASKS],
        }
    }

    /// Get the next order ID and increment counter
    pub fn next_order_id(&mut self) -> u64 {
        let id = self.next_order_id;
        self.next_order_id = self.next_order_id.wrapping_add(1);
        id
    }

    /// Insert an order into the book in sorted position
    pub fn insert_order(
        &mut self,
        side: Side,
        owner: Pubkey,
        price: i64,
        qty: i64,
        timestamp: u64,
    ) -> Result<u64, &'static str> {
        // Create new order with ID first (before borrowing arrays)
        let order_id = self.next_order_id();
        let order = Order::new(order_id, owner, side, price, qty, timestamp);

        // Get the appropriate array and count
        let (orders, count) = match side {
            Side::Buy => (&mut self.bids[..], &mut self.num_bids),
            Side::Sell => (&mut self.asks[..], &mut self.num_asks),
        };

        // Check capacity
        let count_usize = *count as usize;
        if count_usize >= orders.len() {
            return Err("Order book full");
        }

        // Insert in sorted position
        insert_sorted(orders, count_usize, order, side);
        *count += 1;

        Ok(order_id)
    }

    /// Remove an order from the book by order_id
    pub fn remove_order(&mut self, order_id: u64) -> Result<Order, &'static str> {
        // Search in bids
        if let Some(idx) = find_order(&self.bids[..self.num_bids as usize], order_id) {
            let order = self.bids[idx];
            remove_order(&mut self.bids, &mut self.num_bids, idx);
            return Ok(order);
        }

        // Search in asks
        if let Some(idx) = find_order(&self.asks[..self.num_asks as usize], order_id) {
            let order = self.asks[idx];
            remove_order(&mut self.asks, &mut self.num_asks, idx);
            return Ok(order);
        }

        Err("Order not found")
    }

    /// Find an order by ID and return a reference
    pub fn find_order(&self, order_id: u64) -> Option<&Order> {
        if let Some(idx) = find_order(&self.bids[..self.num_bids as usize], order_id) {
            return Some(&self.bids[idx]);
        }

        if let Some(idx) = find_order(&self.asks[..self.num_asks as usize], order_id) {
            return Some(&self.asks[idx]);
        }

        None
    }

    /// Get the best bid (highest price)
    pub fn best_bid(&self) -> Option<&Order> {
        if self.num_bids > 0 {
            Some(&self.bids[0])
        } else {
            None
        }
    }

    /// Get the best ask (lowest price)
    pub fn best_ask(&self) -> Option<&Order> {
        if self.num_asks > 0 {
            Some(&self.asks[0])
        } else {
            None
        }
    }
}

/// Insert an order into a sorted array
///
/// Orders are sorted by:
/// - Bids: Descending price (highest first), then FIFO by timestamp
/// - Asks: Ascending price (lowest first), then FIFO by timestamp
fn insert_sorted(orders: &mut [Order], count: usize, order: Order, side: Side) {
    // Find insertion position
    let pos = match side {
        Side::Buy => {
            // Descending price, then FIFO timestamp
            orders[..count]
                .iter()
                .position(|o| {
                    order.price > o.price
                        || (order.price == o.price && order.timestamp < o.timestamp)
                })
                .unwrap_or(count)
        }
        Side::Sell => {
            // Ascending price, then FIFO timestamp
            orders[..count]
                .iter()
                .position(|o| {
                    order.price < o.price
                        || (order.price == o.price && order.timestamp < o.timestamp)
                })
                .unwrap_or(count)
        }
    };

    // Shift orders to make room
    if pos < count {
        unsafe {
            core::ptr::copy(
                &orders[pos] as *const Order,
                &mut orders[pos + 1] as *mut Order,
                count - pos,
            );
        }
    }

    // Insert new order
    orders[pos] = order;
}

/// Remove an order from an array at a given index
fn remove_order(orders: &mut [Order], count: &mut u16, idx: usize) {
    let count_usize = *count as usize;

    // Shift remaining orders
    if idx < count_usize - 1 {
        unsafe {
            core::ptr::copy(
                &orders[idx + 1] as *const Order,
                &mut orders[idx] as *mut Order,
                count_usize - idx - 1,
            );
        }
    }

    // Clear last slot
    orders[count_usize - 1] = Order::default();
    *count -= 1;
}

/// Find an order by ID in a slice
fn find_order(orders: &[Order], order_id: u64) -> Option<usize> {
    orders.iter().position(|o| o.order_id == order_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_size() {
        use core::mem::size_of;

        let order_size = size_of::<Order>();
        // Note: Actual size is ~80 bytes due to Pubkey alignment
        println!("Order size: {} bytes", order_size);
        assert!(order_size <= 96, "Order should be <= 96 bytes");
    }

    #[test]
    fn test_book_area_size() {
        use core::mem::size_of;

        let book_size = size_of::<BookArea>();

        // With Order size of ~80 bytes:
        // Header: 8 + 2 + 2 + 4 = 16 bytes
        // Bids: 19 * 80 = 1,520 bytes
        // Asks: 19 * 80 = 1,520 bytes
        // Total: ~3,056 bytes
        println!("BookArea size: {} bytes", book_size);
        assert!(book_size <= 3072, "BookArea should fit in 3KB (3,072 bytes), got {} bytes", book_size);
        assert!(book_size >= 2000, "BookArea should be at least 2KB");
    }

    #[test]
    fn test_insert_bid_sorted() {
        let mut book = BookArea::new();
        let owner = Pubkey::default();

        // Insert bids at different prices
        let id1 = book.insert_order(Side::Buy, owner, 100_000_000, 1_000_000, 1000).unwrap();
        let id2 = book.insert_order(Side::Buy, owner, 105_000_000, 2_000_000, 1001).unwrap();
        let id3 = book.insert_order(Side::Buy, owner, 95_000_000, 1_500_000, 1002).unwrap();

        assert_eq!(book.num_bids, 3);

        // Should be sorted descending by price
        assert_eq!(book.bids[0].order_id, id2); // $105
        assert_eq!(book.bids[0].price, 105_000_000);
        assert_eq!(book.bids[1].order_id, id1); // $100
        assert_eq!(book.bids[1].price, 100_000_000);
        assert_eq!(book.bids[2].order_id, id3); // $95
        assert_eq!(book.bids[2].price, 95_000_000);
    }

    #[test]
    fn test_insert_ask_sorted() {
        let mut book = BookArea::new();
        let owner = Pubkey::default();

        // Insert asks at different prices
        let id1 = book.insert_order(Side::Sell, owner, 100_000_000, 1_000_000, 1000).unwrap();
        let id2 = book.insert_order(Side::Sell, owner, 95_000_000, 2_000_000, 1001).unwrap();
        let id3 = book.insert_order(Side::Sell, owner, 105_000_000, 1_500_000, 1002).unwrap();

        assert_eq!(book.num_asks, 3);

        // Should be sorted ascending by price
        assert_eq!(book.asks[0].order_id, id2); // $95
        assert_eq!(book.asks[0].price, 95_000_000);
        assert_eq!(book.asks[1].order_id, id1); // $100
        assert_eq!(book.asks[1].price, 100_000_000);
        assert_eq!(book.asks[2].order_id, id3); // $105
        assert_eq!(book.asks[2].price, 105_000_000);
    }

    #[test]
    fn test_fifo_ordering_same_price() {
        let mut book = BookArea::new();
        let owner = Pubkey::default();

        // Insert three bids at same price with different timestamps
        let id1 = book.insert_order(Side::Buy, owner, 100_000_000, 1_000_000, 1000).unwrap();
        let id2 = book.insert_order(Side::Buy, owner, 100_000_000, 2_000_000, 1001).unwrap();
        let id3 = book.insert_order(Side::Buy, owner, 100_000_000, 1_500_000, 999).unwrap();

        assert_eq!(book.num_bids, 3);

        // Should be sorted by timestamp (FIFO) at same price
        assert_eq!(book.bids[0].order_id, id3); // timestamp 999
        assert_eq!(book.bids[1].order_id, id1); // timestamp 1000
        assert_eq!(book.bids[2].order_id, id2); // timestamp 1001
    }

    #[test]
    fn test_remove_order() {
        let mut book = BookArea::new();
        let owner = Pubkey::default();

        // Insert orders
        let id1 = book.insert_order(Side::Buy, owner, 100_000_000, 1_000_000, 1000).unwrap();
        let id2 = book.insert_order(Side::Buy, owner, 105_000_000, 2_000_000, 1001).unwrap();
        let id3 = book.insert_order(Side::Buy, owner, 95_000_000, 1_500_000, 1002).unwrap();

        assert_eq!(book.num_bids, 3);

        // Remove middle order
        let removed = book.remove_order(id1).unwrap();
        assert_eq!(removed.order_id, id1);
        assert_eq!(book.num_bids, 2);

        // Check remaining orders are still sorted
        assert_eq!(book.bids[0].order_id, id2);
        assert_eq!(book.bids[1].order_id, id3);
    }

    #[test]
    fn test_find_order() {
        let mut book = BookArea::new();
        let owner = Pubkey::default();

        let id1 = book.insert_order(Side::Buy, owner, 100_000_000, 1_000_000, 1000).unwrap();
        let id2 = book.insert_order(Side::Sell, owner, 105_000_000, 2_000_000, 1001).unwrap();

        // Find existing orders
        assert!(book.find_order(id1).is_some());
        assert!(book.find_order(id2).is_some());

        // Try to find non-existent order
        assert!(book.find_order(999).is_none());
    }

    #[test]
    fn test_best_bid_ask() {
        let mut book = BookArea::new();
        let owner = Pubkey::default();

        // Empty book
        assert!(book.best_bid().is_none());
        assert!(book.best_ask().is_none());

        // Add orders
        book.insert_order(Side::Buy, owner, 100_000_000, 1_000_000, 1000).unwrap();
        book.insert_order(Side::Buy, owner, 105_000_000, 2_000_000, 1001).unwrap();
        book.insert_order(Side::Sell, owner, 110_000_000, 1_500_000, 1002).unwrap();
        book.insert_order(Side::Sell, owner, 115_000_000, 1_000_000, 1003).unwrap();

        // Best bid should be highest price
        let best_bid = book.best_bid().unwrap();
        assert_eq!(best_bid.price, 105_000_000);

        // Best ask should be lowest price
        let best_ask = book.best_ask().unwrap();
        assert_eq!(best_ask.price, 110_000_000);
    }

    #[test]
    fn test_capacity_limit() {
        let mut book = BookArea::new();
        let owner = Pubkey::default();

        // Fill up the bid side
        for i in 0..MAX_BIDS {
            let result = book.insert_order(
                Side::Buy,
                owner,
                100_000_000 - (i as i64 * 1_000),
                1_000_000,
                1000 + i as u64,
            );
            assert!(result.is_ok());
        }

        assert_eq!(book.num_bids, MAX_BIDS as u16);

        // Try to insert one more - should fail
        let result = book.insert_order(Side::Buy, owner, 50_000_000, 1_000_000, 2000);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Order book full");
    }

    #[test]
    fn test_order_id_monotonic() {
        let mut book = BookArea::new();
        let owner = Pubkey::default();

        let id1 = book.insert_order(Side::Buy, owner, 100_000_000, 1_000_000, 1000).unwrap();
        let id2 = book.insert_order(Side::Sell, owner, 105_000_000, 2_000_000, 1001).unwrap();
        let id3 = book.insert_order(Side::Buy, owner, 95_000_000, 1_500_000, 1002).unwrap();

        // Order IDs should be monotonically increasing
        assert!(id1 < id2);
        assert!(id2 < id3);
    }
}
