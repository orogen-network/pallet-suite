use crate::mock::*;
use crate::{Error, Operators, TotalStake};
use frame_support::{assert_noop, assert_ok};
use sp_core::H256;

#[test]
fn register_and_heartbeat_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(crate::Pallet::<Test>::register(
            RuntimeOrigin::signed(1),
            100,
            H256::repeat_byte(1),
        ));
        assert_eq!(TotalStake::<Test>::get(), 100);
        assert!(Operators::<Test>::contains_key(1));
        assert_ok!(crate::Pallet::<Test>::heartbeat(
            RuntimeOrigin::signed(1),
            42,
            H256::repeat_byte(2),
            H256::repeat_byte(3),
        ));
        assert_eq!(Operators::<Test>::get(1).unwrap().last_heartbeat_epoch, 42);
    });
}

#[test]
fn double_register_fails() {
    new_test_ext().execute_with(|| {
        assert_ok!(crate::Pallet::<Test>::register(
            RuntimeOrigin::signed(1),
            100,
            H256::repeat_byte(1),
        ));
        assert_noop!(
            crate::Pallet::<Test>::register(RuntimeOrigin::signed(1), 100, H256::repeat_byte(1),),
            Error::<Test>::AlreadyRegistered
        );
    });
}

#[test]
fn register_requires_minimum_stake() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            crate::Pallet::<Test>::register(RuntimeOrigin::signed(1), 99, H256::repeat_byte(1),),
            Error::<Test>::InsufficientStake
        );
    });
}

#[test]
fn heartbeat_must_be_monotonic_and_bounded() {
    new_test_ext().execute_with(|| {
        assert_ok!(crate::Pallet::<Test>::register(
            RuntimeOrigin::signed(1),
            100,
            H256::repeat_byte(1),
        ));
        assert_ok!(crate::Pallet::<Test>::heartbeat(
            RuntimeOrigin::signed(1),
            10,
            H256::repeat_byte(2),
            H256::repeat_byte(3),
        ));
        assert_noop!(
            crate::Pallet::<Test>::heartbeat(
                RuntimeOrigin::signed(1),
                9,
                H256::repeat_byte(2),
                H256::repeat_byte(3),
            ),
            Error::<Test>::HeartbeatEpochStale
        );
        assert_noop!(
            crate::Pallet::<Test>::heartbeat(
                RuntimeOrigin::signed(1),
                111,
                H256::repeat_byte(2),
                H256::repeat_byte(3),
            ),
            Error::<Test>::HeartbeatEpochTooFarAhead
        );
    });
}

#[test]
fn slash_requires_root_origin() {
    new_test_ext().execute_with(|| {
        assert_ok!(crate::Pallet::<Test>::register(
            RuntimeOrigin::signed(1),
            100,
            H256::repeat_byte(1),
        ));
        // Signed origin must be rejected.
        assert!(crate::Pallet::<Test>::slash(RuntimeOrigin::signed(99), 1, 10, 0).is_err());
        // Root succeeds.
        assert_ok!(crate::Pallet::<Test>::slash(
            RuntimeOrigin::root(),
            1,
            10,
            0
        ));
        assert_eq!(Operators::<Test>::get(1).unwrap().stake, 90);
    });
}

#[test]
fn frozen_operator_cannot_exit_or_heartbeat() {
    new_test_ext().execute_with(|| {
        assert_ok!(crate::Pallet::<Test>::register(
            RuntimeOrigin::signed(1),
            100,
            H256::repeat_byte(1),
        ));
        assert_ok!(crate::Pallet::<Test>::freeze_operator(&1));
        assert_noop!(
            crate::Pallet::<Test>::unregister(RuntimeOrigin::signed(1)),
            Error::<Test>::Frozen
        );
        assert_noop!(
            crate::Pallet::<Test>::heartbeat(
                RuntimeOrigin::signed(1),
                1,
                H256::repeat_byte(2),
                H256::repeat_byte(3),
            ),
            Error::<Test>::Frozen
        );
    });
}

#[test]
fn multiple_pending_freezes_require_multiple_releases() {
    new_test_ext().execute_with(|| {
        assert_ok!(crate::Pallet::<Test>::register(
            RuntimeOrigin::signed(1),
            100,
            H256::repeat_byte(1),
        ));
        assert_ok!(crate::Pallet::<Test>::freeze_operator(&1));
        assert_ok!(crate::Pallet::<Test>::freeze_operator(&1));
        let op = Operators::<Test>::get(1).unwrap();
        assert!(op.frozen);
        assert_eq!(op.pending_freezes, 2);

        assert_ok!(crate::Pallet::<Test>::unfreeze_operator(&1));
        let op = Operators::<Test>::get(1).unwrap();
        assert!(op.frozen);
        assert_eq!(op.pending_freezes, 1);
        assert_noop!(
            crate::Pallet::<Test>::unregister(RuntimeOrigin::signed(1)),
            Error::<Test>::Frozen
        );

        assert_ok!(crate::Pallet::<Test>::slash_operator_by_bps(&1, 1_000, 42));
        let op = Operators::<Test>::get(1).unwrap();
        assert!(!op.frozen);
        assert_eq!(op.pending_freezes, 0);
        assert_eq!(op.stake, 90);
    });
}
