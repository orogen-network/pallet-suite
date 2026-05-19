use crate::mock::*;
use crate::{Error, JobState, Jobs};
use frame_support::{assert_noop, assert_ok};
use sp_core::H256;

#[test]
fn job_lifecycle_submit_assign_finalize() {
    new_test_ext().execute_with(|| {
        let jid = H256::repeat_byte(11);
        assert_ok!(crate::Pallet::<Test>::submit_job(
            RuntimeOrigin::signed(1),
            jid,
            2, // gateway
            H256::repeat_byte(22),
            None,
        ));
        assert_eq!(Jobs::<Test>::get(jid).unwrap().state, JobState::Submitted);
        // Signed cannot assign — must come from `GatewayOrigin` (root in mock).
        assert!(crate::Pallet::<Test>::assign(RuntimeOrigin::signed(2), jid, 3).is_err());
        assert_ok!(crate::Pallet::<Test>::assign(RuntimeOrigin::root(), jid, 3));
        assert_eq!(Jobs::<Test>::get(jid).unwrap().state, JobState::Assigned);
        assert_ok!(crate::Pallet::<Test>::finalize(RuntimeOrigin::root(), jid));
        assert_eq!(Jobs::<Test>::get(jid).unwrap().state, JobState::Finalized);
    });
}

#[test]
fn dispute_moves_state_for_customer() {
    new_test_ext().execute_with(|| {
        let jid = H256::repeat_byte(33);
        assert_ok!(crate::Pallet::<Test>::submit_job(
            RuntimeOrigin::signed(1),
            jid,
            2,
            H256::repeat_byte(44),
            None,
        ));
        // Random third-party cannot dispute.
        assert_noop!(
            crate::Pallet::<Test>::dispute(RuntimeOrigin::signed(99), jid),
            Error::<Test>::NotAuthorized
        );
        // Customer can.
        assert_ok!(crate::Pallet::<Test>::dispute(
            RuntimeOrigin::signed(1),
            jid
        ));
        assert_eq!(Jobs::<Test>::get(jid).unwrap().state, JobState::Disputed);
    });
}

#[test]
fn cannot_dispute_finalized_job() {
    new_test_ext().execute_with(|| {
        let jid = H256::repeat_byte(55);
        assert_ok!(crate::Pallet::<Test>::submit_job(
            RuntimeOrigin::signed(1),
            jid,
            2,
            H256::repeat_byte(44),
            None,
        ));
        assert_ok!(crate::Pallet::<Test>::assign(RuntimeOrigin::root(), jid, 3));
        assert_ok!(crate::Pallet::<Test>::finalize(RuntimeOrigin::root(), jid));
        assert_noop!(
            crate::Pallet::<Test>::dispute(RuntimeOrigin::signed(1), jid),
            Error::<Test>::BadState
        );
    });
}
