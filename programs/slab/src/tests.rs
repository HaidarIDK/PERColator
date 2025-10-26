//! Unit tests for slab operations
//!
//! Basic unit tests for v1 orderbook functionality

#[cfg(test)]
mod slab_orderbook_tests {
    use crate::state::{SlabState, Side};
    use crate::instructions::{process_place_order, process_cancel_order};
    use pinocchio::pubkey::Pubkey;
    use std::mem;

    /// Helper to create a test SlabState with initialized book
    fn create_test_slab() -> SlabState {
        let mut slab = unsafe { mem::zeroed::<SlabState>() };

        // Initialize header
        slab.header.lp_owner = Pubkey::default();
        slab.header.router_id = Pubkey::from([1u8; 32]);
        slab.header.instrument = Pubkey::default();
        slab.header.seqno = 100;
        slab.header.mark_px = 1_000_000;  // 1.0 in 1e6 scale
        slab.header.taker_fee_bps = 10;   // 0.1% fee
        slab.header.contract_size = 1_000_000;
        slab.header.bump = 255;

        // Initialize book
        slab.book.next_order_id = 1;
        slab.book.num_bids = 0;
        slab.book.num_asks = 0;

        slab
    }

    #[test]
    fn test_orderbook_insert_ask() {
        let mut slab = create_test_slab();

        // Place a single ask at 1.5
        let ask_owner = Pubkey::from([2u8; 32]);
        let order_id = slab.book.insert_order(Side::Sell, ask_owner, 1_500_000, 10_000_000, 1000).unwrap();

        assert_eq!(slab.book.num_asks, 1);
        assert_eq!(slab.book.asks[0].price, 1_500_000);
        assert_eq!(slab.book.asks[0].qty, 10_000_000);
        assert_eq!(slab.book.asks[0].owner, ask_owner);
        assert_eq!(slab.book.asks[0].order_id, order_id);
    }

    #[test]
    fn test_orderbook_insert_bid() {
        let mut slab = create_test_slab();

        // Place a single bid at 1.2
        let bid_owner = Pubkey::from([3u8; 32]);
        let order_id = slab.book.insert_order(Side::Buy, bid_owner, 1_200_000, 8_000_000, 2000).unwrap();

        assert_eq!(slab.book.num_bids, 1);
        assert_eq!(slab.book.bids[0].price, 1_200_000);
        assert_eq!(slab.book.bids[0].qty, 8_000_000);
        assert_eq!(slab.book.bids[0].owner, bid_owner);
        assert_eq!(slab.book.bids[0].order_id, order_id);
    }

    #[test]
    fn test_orderbook_multiple_asks_sorted() {
        let mut slab = create_test_slab();

        // Place asks at different prices (should be sorted ascending)
        let owner = Pubkey::from([4u8; 32]);
        slab.book.insert_order(Side::Sell, owner, 1_400_000, 3_000_000, 1000).unwrap();
        slab.book.insert_order(Side::Sell, owner, 1_500_000, 2_000_000, 1001).unwrap();
        slab.book.insert_order(Side::Sell, owner, 1_300_000, 1_000_000, 1002).unwrap();

        assert_eq!(slab.book.num_asks, 3);
        // Should be sorted by price ascending
        assert_eq!(slab.book.asks[0].price, 1_300_000);
        assert_eq!(slab.book.asks[1].price, 1_400_000);
        assert_eq!(slab.book.asks[2].price, 1_500_000);
    }

    #[test]
    fn test_orderbook_multiple_bids_sorted() {
        let mut slab = create_test_slab();

        // Place bids at different prices (should be sorted descending)
        let owner = Pubkey::from([5u8; 32]);
        slab.book.insert_order(Side::Buy, owner, 1_200_000, 3_000_000, 1000).unwrap();
        slab.book.insert_order(Side::Buy, owner, 1_100_000, 2_000_000, 1001).unwrap();
        slab.book.insert_order(Side::Buy, owner, 1_300_000, 1_000_000, 1002).unwrap();

        assert_eq!(slab.book.num_bids, 3);
        // Should be sorted by price descending
        assert_eq!(slab.book.bids[0].price, 1_300_000);
        assert_eq!(slab.book.bids[1].price, 1_200_000);
        assert_eq!(slab.book.bids[2].price, 1_100_000);
    }

    #[test]
    fn test_orderbook_remove_order() {
        let mut slab = create_test_slab();

        let owner = Pubkey::from([6u8; 32]);
        let order_id = slab.book.insert_order(Side::Sell, owner, 1_500_000, 10_000_000, 1000).unwrap();

        assert_eq!(slab.book.num_asks, 1);

        // Remove the order
        slab.book.remove_order(order_id).unwrap();

        assert_eq!(slab.book.num_asks, 0);
    }

    #[test]
    fn test_orderbook_full_capacity_asks() {
        let mut slab = create_test_slab();

        // Fill orderbook to capacity (19 asks)
        let owner = Pubkey::from([7u8; 32]);
        for i in 0..19 {
            let price = 1_000_000 + (i as i64 * 100_000);
            let result = slab.book.insert_order(Side::Sell, owner, price, 1_000_000, 1000 + i);
            assert!(result.is_ok());
        }

        assert_eq!(slab.book.num_asks, 19);

        // Try to add one more (should fail)
        let result = slab.book.insert_order(Side::Sell, owner, 3_000_000, 1_000_000, 1019);
        assert!(result.is_err());
    }

    #[test]
    fn test_place_cancel_order_integration() {
        let mut slab = create_test_slab();
        let owner = Pubkey::from([8u8; 32]);

        // Place an order
        let order_id = process_place_order(
            &mut slab,
            &owner,
            Side::Buy,
            1_200_000,
            5_000_000,
        ).unwrap();

        assert_eq!(slab.book.num_bids, 1);
        let seqno_after_place = slab.header.seqno;

        // Cancel the order
        let result = process_cancel_order(&mut slab, &owner, order_id);
        assert!(result.is_ok());

        assert_eq!(slab.book.num_bids, 0);
        assert_eq!(slab.header.seqno, seqno_after_place + 1);
    }

    #[test]
    fn test_cancel_nonexistent_order() {
        let mut slab = create_test_slab();
        let owner = Pubkey::from([9u8; 32]);

        // Try to cancel an order that doesn't exist
        let result = process_cancel_order(&mut slab, &owner, 999);
        assert!(result.is_err());
    }

    #[test]
    fn test_cancel_order_wrong_owner() {
        let mut slab = create_test_slab();
        let owner1 = Pubkey::from([10u8; 32]);
        let owner2 = Pubkey::from([11u8; 32]);

        // Place an order with owner1
        let order_id = process_place_order(
            &mut slab,
            &owner1,
            Side::Sell,
            1_500_000,
            3_000_000,
        ).unwrap();

        // Try to cancel with owner2 (should fail)
        let result = process_cancel_order(&mut slab, &owner2, order_id);
        assert!(result.is_err());

        // Order should still be there
        assert_eq!(slab.book.num_asks, 1);
    }
}
