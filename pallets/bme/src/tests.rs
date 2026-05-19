use crate::mock::*;
use crate::{CumulativeBurn, CumulativeMint, Elasticity, Error, OperatorBalance};
use frame_support::{assert_noop, assert_ok};
use sp_core::H256;

#[test]
fn burn_then_mint_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(crate::Pallet::<Test>::submit_burn(
            RuntimeOrigin::root(),
            H256::repeat_byte(9),
            1_000,
        ));
        assert_eq!(CumulativeBurn::<Test>::get(), 1_000);

        assert_ok!(crate::Pallet::<Test>::mint_to_operator(
            RuntimeOrigin::root(),
            42,
            500,
        ));
        assert_eq!(CumulativeMint::<Test>::get(), 500);
        assert_eq!(OperatorBalance::<Test>::get(42), 500);
    });
}

#[test]
fn duplicate_burn_batch_is_rejected() {
    new_test_ext().execute_with(|| {
        let batch = H256::repeat_byte(9);
        assert_ok!(crate::Pallet::<Test>::submit_burn(
            RuntimeOrigin::root(),
            batch,
            1_000,
        ));
        assert_noop!(
            crate::Pallet::<Test>::submit_burn(RuntimeOrigin::root(), batch, 1_000),
            Error::<Test>::DuplicateBurnBatch
        );
        assert_eq!(CumulativeBurn::<Test>::get(), 1_000);
    });
}

#[test]
fn elasticity_persists() {
    new_test_ext().execute_with(|| {
        assert_ok!(crate::Pallet::<Test>::set_elasticity(
            RuntimeOrigin::root(),
            12_000,
        ));
        assert_eq!(Elasticity::<Test>::get(), 12_000);
    });
}

#[test]
fn signed_origin_rejected_on_privileged_calls() {
    new_test_ext().execute_with(|| {
        assert!(crate::Pallet::<Test>::submit_burn(
            RuntimeOrigin::signed(1),
            H256::repeat_byte(1),
            100,
        )
        .is_err());
        assert!(
            crate::Pallet::<Test>::mint_to_operator(RuntimeOrigin::signed(1), 42, 10,).is_err()
        );
        assert!(crate::Pallet::<Test>::set_elasticity(RuntimeOrigin::signed(1), 1).is_err());
    });
}

#[test]
fn mint_batch_enforces_headroom() {
    new_test_ext().execute_with(|| {
        assert_ok!(crate::Pallet::<Test>::submit_burn(
            RuntimeOrigin::root(),
            H256::repeat_byte(9),
            1_000,
        ));
        assert_noop!(
            crate::Pallet::<Test>::mint_batch(1, vec![(42, 600), (43, 500)]),
            Error::<Test>::MintExceedsHeadroom
        );
        assert_eq!(CumulativeMint::<Test>::get(), 0);

        assert_ok!(crate::Pallet::<Test>::mint_batch(
            1,
            vec![(42, 600), (43, 400)]
        ));
        assert_eq!(CumulativeMint::<Test>::get(), 1_000);
        assert_eq!(OperatorBalance::<Test>::get(42), 600);
        assert_eq!(OperatorBalance::<Test>::get(43), 400);
    });
}
