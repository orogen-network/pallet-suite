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
            crate::Pallet::<Test>::register(
                RuntimeOrigin::signed(1),
                100,
                H256::repeat_byte(1),
            ),
            Error::<Test>::AlreadyRegistered
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
        assert_ok!(crate::Pallet::<Test>::slash(RuntimeOrigin::root(), 1, 10, 0));
        assert_eq!(Operators::<Test>::get(1).unwrap().stake, 90);
    });
}
