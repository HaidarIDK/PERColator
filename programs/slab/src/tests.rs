//! Unit tests for slab operations

#[cfg(test)]
mod pool_tests {
    use crate::state::pools::*;
    use percolator_common::Order;

    #[test]
    fn test_pool_alloc_free() {
        let mut pool: Pool<Order, 10> = Pool::new();

        assert_eq!(pool.used(), 0);
        assert!(!pool.is_full());

        let idx1 = pool.alloc().unwrap();
        assert_eq!(idx1, 0);
        assert_eq!(pool.used(), 1);

        let idx2 = pool.alloc().unwrap();
        assert_eq!(idx2, 1);
        assert_eq!(pool.used(), 2);

        pool.free(idx1);
        assert_eq!(pool.used(), 1);

        let idx3 = pool.alloc().unwrap();
        assert_eq!(idx3, 0); // Reuses freed slot
        assert_eq!(pool.used(), 2);
    }

    #[test]
    fn test_pool_full() {
        let mut pool: Pool<Order, 3> = Pool::new();

        assert!(pool.alloc().is_some());
        assert!(pool.alloc().is_some());
        assert!(pool.alloc().is_some());
        assert!(pool.is_full());
        assert!(pool.alloc().is_none());
    }

    #[test]
    fn test_pool_get_after_alloc() {
        let mut pool: Pool<Order, 10> = Pool::new();

        let idx = pool.alloc().unwrap();

        // Should be able to get the allocated item
        assert!(pool.get(idx).is_some());

        // Should be able to mutate it
        if let Some(order) = pool.get_mut(idx) {
            order.order_id = 123;
            order.price = 50_000;
        }

        assert_eq!(pool.get(idx).unwrap().order_id, 123);
        assert_eq!(pool.get(idx).unwrap().price, 50_000);
    }

    #[test]
    fn test_pool_get_after_free() {
        let mut pool: Pool<Order, 10> = Pool::new();

        let idx = pool.alloc().unwrap();
        pool.free(idx);

        // Should not be able to get freed item
        assert!(pool.get(idx).is_none());
    }

    #[test]
    fn test_pool_double_free() {
        let mut pool: Pool<Order, 10> = Pool::new();

        let idx = pool.alloc().unwrap();
        assert_eq!(pool.used(), 1);

        pool.free(idx);
        assert_eq!(pool.used(), 0);

        // Double free should be idempotent
        pool.free(idx);
        assert_eq!(pool.used(), 0);
    }

    #[test]
    fn test_pool_alloc_all_then_free_all() {
        let mut pool: Pool<Order, 5> = Pool::new();
        let mut indices = [0u32; 5];

        // Allocate all
        for i in 0..5 {
            indices[i] = pool.alloc().unwrap();
        }
        assert!(pool.is_full());

        // Free all
        for i in 0..5 {
            pool.free(indices[i]);
        }
        assert_eq!(pool.used(), 0);

        // Should be able to allocate again
        for _ in 0..5 {
            assert!(pool.alloc().is_some());
        }
        assert!(pool.is_full());
    }
}

#[cfg(test)]
mod header_tests {
    use crate::state::header::SlabHeader;
    use pinocchio::pubkey::Pubkey;

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
        assert_eq!(header.book_seqno, 0);
    }

    #[test]
    fn test_header_monotonic_order_ids() {
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
        assert_eq!(header.next_order_id, 4);
    }

    #[test]
    fn test_header_monotonic_hold_ids() {
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

        assert_eq!(header.next_hold_id(), 1);
        assert_eq!(header.next_hold_id(), 2);
        assert_eq!(header.next_hold_id, 3);
    }

    #[test]
    fn test_header_book_seqno() {
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

        assert_eq!(header.book_seqno, 0);
        assert_eq!(header.increment_book_seqno(), 1);
        assert_eq!(header.increment_book_seqno(), 2);
        assert_eq!(header.book_seqno, 2);
    }

    #[test]
    fn test_jit_penalty_detection() {
        let header = SlabHeader::new(
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

        // Order created before batch_open - no JIT penalty
        assert!(!header.is_jit_order(50, 100));

        // Order created at batch_open - JIT penalty
        assert!(header.is_jit_order(100, 100));

        // Order created after batch_open - JIT penalty
        assert!(header.is_jit_order(150, 100));
    }

    #[test]
    fn test_timestamp_update() {
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

        assert_eq!(header.current_ts, 0);
        header.update_timestamp(12345);
        assert_eq!(header.current_ts, 12345);
    }
}

// NOTE: Order book operation tests are deferred to integration tests with surfpool
// Testing with the ~70KB SlabState (ULTRA-CHEAP mode: ~0.5 SOL rent!)
// Integration tests will provide better coverage of book operations in a realistic environment
