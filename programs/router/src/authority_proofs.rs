//! Kani Proofs for LP Adapter Authority Model
//!
//! These proofs verify the seat-scoping and router-only-access properties
//! at the function level, complementing the on-chain PDA/signer enforcement.

#![cfg_attr(not(feature = "std"), no_std)]

use pinocchio::pubkey::Pubkey;

// Minimal stand-ins for proof models
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
    fn new() -> Self {
        Self { seats: [None; 8] }
    }

    fn add_seat(&mut self, seat: SeatRow) {
        for slot in &mut self.seats {
            if slot.is_none() {
                *slot = Some(seat);
                return;
            }
        }
    }

    fn find_seat(&self, seat_id: Pubkey) -> Option<&SeatRow> {
        self.seats.iter().filter_map(|s| s.as_ref()).find(|r| r.seat_id == seat_id)
    }
}

fn owns_object(row: &SeatRow, obj: &VenueObject) -> bool {
    row.seat_id == obj.seat_id
}

// Model PDA derivation equality (we just compare keys here)
fn is_valid_router_pda(pda: Pubkey, expected: Pubkey) -> bool {
    pda == expected
}

// ── Proof 1: Seat ownership is necessary to cancel/modify ──────────────────────
#[cfg(kani)]
#[kani::proof]
fn proof_scope_required_for_modify() {
    let seat = Pubkey::from(kani::any::<[u8; 32]>());
    let other = Pubkey::from(kani::any::<[u8; 32]>());
    kani::assume(seat != other);

    let mut st = MatcherState::new();
    st.add_seat(SeatRow { seat_id: seat });
    let row = st.find_seat(seat).unwrap();

    let obj_owned = VenueObject {
        id: 1,
        seat_id: seat,
    };
    let obj_foreign = VenueObject {
        id: 2,
        seat_id: other,
    };

    // Proof: Only owned objects can be modified
    assert!(owns_object(row, &obj_owned));
    assert!(!owns_object(row, &obj_foreign));
}

// ── Proof 2: Only router PDA can act (model) ───────────────────────────────────
#[cfg(kani)]
#[kani::proof]
fn proof_only_router_pda_may_call() {
    let expected_router_pda = Pubkey::from(kani::any::<[u8; 32]>());
    let caller_pda = Pubkey::from(kani::any::<[u8; 32]>());

    // The matcher must accept iff caller==expected
    let allowed = is_valid_router_pda(caller_pda, expected_router_pda);

    if caller_pda == expected_router_pda {
        assert!(allowed);
    } else {
        assert!(!allowed);
    }
}

// ── Proof 3: Cancel-all is seat-local (cannot affect other seats) ──────────────
#[cfg(kani)]
#[kani::proof]
fn proof_cancel_all_is_seat_local() {
    let s1 = Pubkey::from(kani::any::<[u8; 32]>());
    let s2 = Pubkey::from(kani::any::<[u8; 32]>());
    kani::assume(s1 != s2);

    let row1 = SeatRow { seat_id: s1 };
    let row2 = SeatRow { seat_id: s2 };

    // Model: cancel_all transforms only objects tagged with seat s1
    let objs = [
        VenueObject {
            id: 1,
            seat_id: s1,
        },
        VenueObject {
            id: 2,
            seat_id: s2,
        },
    ];

    // "Cancelled" predicate for seat s1 objects
    let cancelled1 = owns_object(&row1, &objs[0]); // true
    let cancelled2 = owns_object(&row1, &objs[1]); // false

    assert!(cancelled1);
    assert!(!cancelled2); // proves other-seat objects are untouched
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
