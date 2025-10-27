//! Kani Proofs for LP Adapter Authority Model (Router Production Code)
//!
//! These proofs verify production Router code, specifically the operator delegation
//! logic in RouterLpSeat. Matcher-side authority (PDA verification, seat-scoping)
//! is enforced by on-chain checks in the matcher program, not verified here.

#![cfg_attr(not(feature = "std"), no_std)]

use pinocchio::pubkey::Pubkey;

// ═══════════════════════════════════════════════════════════════════════════════
// DESIGN MODELS (for documentation, not verified by Kani)
// ═══════════════════════════════════════════════════════════════════════════════
// These models document the intended matcher-side behavior but are not production
// Router code. The matcher program implements these properties via on-chain checks.

#[allow(dead_code)]
mod design_models {
    use super::*;

    // Model types representing matcher-side concepts
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct SeatRow {
        pub seat_id: Pubkey,
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct VenueObject {
        pub id: u128,
        pub seat_id: Pubkey,
    }

    pub struct MatcherState {
        pub seats: [Option<SeatRow>; 8],
    }

    impl MatcherState {
        pub fn new() -> Self {
            Self { seats: [None; 8] }
        }

        pub fn add_seat(&mut self, seat: SeatRow) {
            for slot in &mut self.seats {
                if slot.is_none() {
                    *slot = Some(seat);
                    return;
                }
            }
        }

        pub fn find_seat(&self, seat_id: Pubkey) -> Option<&SeatRow> {
            self.seats.iter().filter_map(|s| s.as_ref()).find(|r| r.seat_id == seat_id)
        }
    }

    pub fn owns_object(row: &SeatRow, obj: &VenueObject) -> bool {
        row.seat_id == obj.seat_id
    }

    pub fn is_valid_router_pda(pda: Pubkey, expected: Pubkey) -> bool {
        pda == expected
    }
}

// ── Proof 4: Operator delegation works correctly ───────────────────────────────
#[cfg(kani)]
#[kani::proof]
fn proof_operator_delegation_correct() {
    use crate::state::RouterLpSeat;

    let mut seat = unsafe { core::mem::zeroed::<RouterLpSeat>() };
    let owner = Pubkey::from(kani::any::<[u8; 32]>());
    let operator = Pubkey::from(kani::any::<[u8; 32]>());
    let stranger = Pubkey::from(kani::any::<[u8; 32]>());

    kani::assume(owner != operator);
    kani::assume(owner != stranger);
    kani::assume(operator != stranger);

    // Initialize seat without operator
    seat.initialize_in_place(
        Pubkey::default(),
        Pubkey::default(),
        Pubkey::default(),
        0,
        255,
    );

    // Owner is always authorized
    assert!(seat.is_authorized(&owner, &owner));

    // Operator and stranger not authorized yet
    assert!(!seat.is_authorized(&operator, &owner));
    assert!(!seat.is_authorized(&stranger, &owner));

    // Set operator
    seat.set_operator(operator);

    // Now both owner and operator are authorized
    assert!(seat.is_authorized(&owner, &owner));
    assert!(seat.is_authorized(&operator, &owner));
    assert!(!seat.is_authorized(&stranger, &owner));

    // Clear operator
    seat.clear_operator();

    // Only owner is authorized again
    assert!(seat.is_authorized(&owner, &owner));
    assert!(!seat.is_authorized(&operator, &owner));
    assert!(!seat.is_authorized(&stranger, &owner));
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::design_models::*;

    #[test]
    fn test_seat_ownership_model() {
        let seat = Pubkey::from([1; 32]);
        let other = Pubkey::from([2; 32]);

        let row = SeatRow { seat_id: seat };
        let obj_owned = VenueObject {
            id: 1,
            seat_id: seat,
        };
        let obj_foreign = VenueObject {
            id: 2,
            seat_id: other,
        };

        assert!(owns_object(&row, &obj_owned));
        assert!(!owns_object(&row, &obj_foreign));
    }

    #[test]
    fn test_router_pda_validation() {
        let expected = Pubkey::from([1; 32]);
        let valid_caller = Pubkey::from([1; 32]);
        let invalid_caller = Pubkey::from([2; 32]);

        assert!(is_valid_router_pda(valid_caller, expected));
        assert!(!is_valid_router_pda(invalid_caller, expected));
    }

    #[test]
    fn test_matcher_state_seat_management() {
        let mut state = MatcherState::new();
        let seat1_id = Pubkey::from([1; 32]);
        let seat2_id = Pubkey::from([2; 32]);

        let seat1 = SeatRow { seat_id: seat1_id };
        let seat2 = SeatRow { seat_id: seat2_id };

        state.add_seat(seat1);
        state.add_seat(seat2);

        assert!(state.find_seat(seat1_id).is_some());
        assert!(state.find_seat(seat2_id).is_some());
        assert!(state.find_seat(Pubkey::from([3; 32])).is_none());
    }
}
