use crate::mock::*;
use crate::{Error, ProposalState, Proposals};
use frame_support::{assert_noop, assert_ok};

#[test]
fn proposal_executes_at_threshold() {
    new_test_ext().execute_with(|| {
        assert_ok!(crate::Pallet::<Test>::propose_spend(
            RuntimeOrigin::signed(1),
            42,
            1_000,
        ));
        assert_eq!(
            Proposals::<Test>::get(0).unwrap().state,
            ProposalState::Pending
        );
        // Same account approving twice fails.
        assert_ok!(crate::Pallet::<Test>::execute_spend(
            RuntimeOrigin::signed(2),
            0
        ));
        assert_noop!(
            crate::Pallet::<Test>::execute_spend(RuntimeOrigin::signed(2), 0),
            Error::<Test>::AlreadyApproved
        );
        assert_eq!(
            Proposals::<Test>::get(0).unwrap().state,
            ProposalState::Pending
        );
        // A second distinct council member tips the proposal over threshold (2).
        assert_ok!(crate::Pallet::<Test>::execute_spend(
            RuntimeOrigin::signed(3),
            0
        ));
        assert_eq!(
            Proposals::<Test>::get(0).unwrap().state,
            ProposalState::Executed
        );
    });
}

#[test]
fn non_council_rejected() {
    new_test_ext().execute_with(|| {
        // Account 99 is not in the council.
        assert_noop!(
            crate::Pallet::<Test>::propose_spend(RuntimeOrigin::signed(99), 42, 1_000),
            Error::<Test>::NotCouncilMember
        );
        assert_ok!(crate::Pallet::<Test>::propose_spend(
            RuntimeOrigin::signed(1),
            42,
            1_000,
        ));
        assert_noop!(
            crate::Pallet::<Test>::execute_spend(RuntimeOrigin::signed(99), 0),
            Error::<Test>::NotCouncilMember
        );
    });
}
