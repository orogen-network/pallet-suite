use crate::mock::*;
use crate::{Adapters, BaseModels, Error};
use frame_support::{assert_noop, assert_ok};
use sp_core::H256;

#[test]
fn register_base_model_works() {
    new_test_ext().execute_with(|| {
        let id = H256::repeat_byte(1);
        let manifest = H256::repeat_byte(2);
        assert_ok!(crate::Pallet::<Test>::register_base_model(
            RuntimeOrigin::signed(1),
            id,
            manifest,
        ));
        assert!(BaseModels::<Test>::contains_key(id));
    });
}

#[test]
fn duplicate_registration_fails() {
    new_test_ext().execute_with(|| {
        let id = H256::repeat_byte(3);
        let manifest = H256::repeat_byte(4);
        assert_ok!(crate::Pallet::<Test>::register_base_model(
            RuntimeOrigin::signed(1),
            id,
            manifest,
        ));
        assert_noop!(
            crate::Pallet::<Test>::register_base_model(RuntimeOrigin::signed(1), id, manifest,),
            Error::<Test>::AlreadyRegistered
        );
    });
}

#[test]
fn register_adapter_against_unknown_fails() {
    new_test_ext().execute_with(|| {
        let aid = H256::repeat_byte(5);
        let bid = H256::repeat_byte(6);
        assert_noop!(
            crate::Pallet::<Test>::register_adapter(
                RuntimeOrigin::signed(1),
                aid,
                bid,
                H256::repeat_byte(7),
            ),
            Error::<Test>::UnknownModel
        );
        assert!(!Adapters::<Test>::contains_key(aid));
    });
}

#[test]
fn id_collision_between_base_and_adapter_rejected() {
    new_test_ext().execute_with(|| {
        let id = H256::repeat_byte(8);
        assert_ok!(crate::Pallet::<Test>::register_base_model(
            RuntimeOrigin::signed(1),
            id,
            H256::repeat_byte(9),
        ));
        // Register a separate base model to use as the adapter parent.
        let bid = H256::repeat_byte(10);
        assert_ok!(crate::Pallet::<Test>::register_base_model(
            RuntimeOrigin::signed(2),
            bid,
            H256::repeat_byte(11),
        ));
        // Now another user tries to register an adapter at the same id as
        // the first base model. Must be rejected.
        assert_noop!(
            crate::Pallet::<Test>::register_adapter(
                RuntimeOrigin::signed(3),
                id,
                bid,
                H256::repeat_byte(12),
            ),
            Error::<Test>::IdCollision
        );
    });
}
