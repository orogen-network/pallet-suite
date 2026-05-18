use crate::mock::*;
use crate::{EpochIncentives, Error};
use frame_support::{assert_noop, assert_ok};

#[test]
fn submit_then_compute_aggregates_scores() {
    new_test_ext().execute_with(|| {
        assert_ok!(crate::Pallet::<Test>::submit_weights(
            RuntimeOrigin::signed(1),
            7,
            sp_std::vec![(100, 1000), (101, 2000)],
        ));
        assert_ok!(crate::Pallet::<Test>::submit_weights(
            RuntimeOrigin::signed(2),
            7,
            sp_std::vec![(100, 500), (101, 1000)],
        ));
        assert_ok!(crate::Pallet::<Test>::compute_epoch_incentives(
            RuntimeOrigin::signed(1),
            7,
        ));
        assert_eq!(EpochIncentives::<Test>::get(7, 100), 1500);
        assert_eq!(EpochIncentives::<Test>::get(7, 101), 3000);
    });
}

#[test]
fn double_compute_rejected() {
    new_test_ext().execute_with(|| {
        assert_ok!(crate::Pallet::<Test>::compute_epoch_incentives(
            RuntimeOrigin::signed(1),
            42,
        ));
        assert_noop!(
            crate::Pallet::<Test>::compute_epoch_incentives(RuntimeOrigin::signed(1), 42),
            Error::<Test>::EpochAlreadyComputed
        );
    });
}

#[test]
fn empty_epoch_compiles() {
    new_test_ext().execute_with(|| {
        assert_ok!(crate::Pallet::<Test>::compute_epoch_incentives(
            RuntimeOrigin::signed(1),
            42,
        ));
    });
}
