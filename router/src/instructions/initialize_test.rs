#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::state::SlabRegistry;
    use pinocchio::pubkey::Pubkey;

    #[test]
    fn test_registry_initialization() {
        // This test validates SlabRegistry initialization logic
        let program_id = Pubkey::default();
        let governance = Pubkey::from([1; 32]);
        let bump = 255; // Use a test bump value

        // Initialize registry using the new() constructor
        let registry = SlabRegistry::new(program_id, governance, bump);

        // Verify initial state
        assert_eq!(registry.router_id, program_id);
        assert_eq!(registry.governance, governance);
        assert_eq!(registry.slab_count, 0);
        assert_eq!(registry.bump, bump);

        // Verify that slabs array is zero-initialized
        for i in 0..percolator_common::MAX_SLABS {
            assert_eq!(registry.slabs[i].slab_id, Pubkey::default());
            assert!(!registry.slabs[i].active);
        }
    }

    #[test]
    fn test_registry_size() {
        // Ensure registry size matches expected layout
        use core::mem::size_of;
        let actual_size = size_of::<SlabRegistry>();
        assert_eq!(actual_size, SlabRegistry::LEN);

        // Size should be reasonable (header + slab entries)
        // Router ID (32) + Governance (32) + count (2) + bump (1) + padding (5)
        // + slabs array
        let header_size = 32 + 32 + 2 + 1 + 5;
        let slab_entry_size = core::mem::size_of::<crate::state::SlabEntry>();
        let expected_min = header_size + (slab_entry_size * percolator_common::MAX_SLABS);

        assert!(actual_size >= expected_min);
    }
}
