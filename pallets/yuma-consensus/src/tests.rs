use crate::mock::*;
use crate::{
    EpochIncentives, EpochScoreTotals, Error, PermittedValidators, ValidatorCount, Validators,
};
use frame_support::{assert_noop, assert_ok};

fn rotate_permits(epoch: u64) {
    assert_ok!(crate::Pallet::<Test>::rotate_permits(
        RuntimeOrigin::root(),
        epoch
    ));
}

#[test]
fn submit_then_compute_aggregates_scores() {
    new_test_ext().execute_with(|| {
        assert_ok!(crate::Pallet::<Test>::add_validator(
            RuntimeOrigin::root(),
            1,
            100,
            1,
        ));
        assert_ok!(crate::Pallet::<Test>::add_validator(
            RuntimeOrigin::root(),
            2,
            100,
            2,
        ));
        rotate_permits(7);
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
            RuntimeOrigin::root(),
            7,
        ));
        assert_eq!(EpochIncentives::<Test>::get(7, 100), 3333);
        assert_eq!(EpochIncentives::<Test>::get(7, 101), 6666);
    });
}

#[test]
fn governed_validator_membership_controls_submitters() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            crate::Pallet::<Test>::submit_weights(
                RuntimeOrigin::signed(1),
                7,
                sp_std::vec![(100, 1000)],
            ),
            Error::<Test>::UnauthorizedValidator
        );
        assert_ok!(crate::Pallet::<Test>::add_validator(
            RuntimeOrigin::root(),
            1,
            100,
            1,
        ));
        assert_eq!(ValidatorCount::<Test>::get(), 1);
        assert!(Validators::<Test>::contains_key(1));
        assert_noop!(
            crate::Pallet::<Test>::submit_weights(
                RuntimeOrigin::signed(1),
                7,
                sp_std::vec![(100, 1000)],
            ),
            Error::<Test>::UnauthorizedValidator
        );
        rotate_permits(7);
        assert_ok!(crate::Pallet::<Test>::submit_weights(
            RuntimeOrigin::signed(1),
            7,
            sp_std::vec![(100, 1000)],
        ));
        assert_ok!(crate::Pallet::<Test>::remove_validator(
            RuntimeOrigin::root(),
            1,
        ));
        assert_eq!(ValidatorCount::<Test>::get(), 0);
        assert_noop!(
            crate::Pallet::<Test>::submit_weights(
                RuntimeOrigin::signed(1),
                8,
                sp_std::vec![(100, 1000)],
            ),
            Error::<Test>::UnauthorizedValidator
        );
    });
}

#[test]
fn finalized_epoch_rejects_late_weight_submissions() {
    new_test_ext().execute_with(|| {
        assert_ok!(crate::Pallet::<Test>::add_validator(
            RuntimeOrigin::root(),
            1,
            100,
            1,
        ));
        rotate_permits(7);
        assert_ok!(crate::Pallet::<Test>::compute_epoch_incentives(
            RuntimeOrigin::root(),
            7,
        ));
        assert_noop!(
            crate::Pallet::<Test>::submit_weights(
                RuntimeOrigin::signed(1),
                7,
                sp_std::vec![(100, 1000)],
            ),
            Error::<Test>::EpochAlreadyComputed
        );
    });
}

#[test]
fn validator_weight_vector_length_is_bounded() {
    new_test_ext().execute_with(|| {
        assert_ok!(crate::Pallet::<Test>::add_validator(
            RuntimeOrigin::root(),
            1,
            100,
            1,
        ));
        rotate_permits(7);
        assert_noop!(
            crate::Pallet::<Test>::submit_weights(
                RuntimeOrigin::signed(1),
                7,
                sp_std::vec![(100, 1), (101, 1), (102, 1), (103, 1), (104, 1)],
            ),
            Error::<Test>::WeightVectorTooLarge
        );
    });
}

#[test]
fn validator_membership_requires_governance_and_respects_bound() {
    new_test_ext().execute_with(|| {
        assert!(crate::Pallet::<Test>::add_validator(RuntimeOrigin::signed(1), 1, 100, 1).is_err());
        assert_ok!(crate::Pallet::<Test>::add_validator(
            RuntimeOrigin::root(),
            1,
            100,
            1,
        ));
        assert_noop!(
            crate::Pallet::<Test>::add_validator(RuntimeOrigin::root(), 1, 100, 1),
            Error::<Test>::ValidatorAlreadyExists
        );
        assert_ok!(crate::Pallet::<Test>::add_validator(
            RuntimeOrigin::root(),
            2,
            100,
            2,
        ));
        assert_ok!(crate::Pallet::<Test>::add_validator(
            RuntimeOrigin::root(),
            3,
            100,
            3,
        ));
        assert_noop!(
            crate::Pallet::<Test>::add_validator(RuntimeOrigin::root(), 4, 100, 4),
            Error::<Test>::TooManyValidators
        );
        assert_noop!(
            crate::Pallet::<Test>::remove_validator(RuntimeOrigin::root(), 4),
            Error::<Test>::ValidatorNotFound
        );
    });
}

#[test]
fn validator_entity_concentration_cap_is_enforced_on_updates() {
    new_test_ext().execute_with(|| {
        assert_ok!(crate::Pallet::<Test>::add_validator(
            RuntimeOrigin::root(),
            1,
            100,
            1,
        ));
        assert_ok!(crate::Pallet::<Test>::add_validator(
            RuntimeOrigin::root(),
            2,
            100,
            2,
        ));
        assert_ok!(crate::Pallet::<Test>::update_validator_stake(
            RuntimeOrigin::root(),
            1,
            150,
            1,
        ));
        assert_noop!(
            crate::Pallet::<Test>::update_validator_stake(RuntimeOrigin::root(), 1, 200, 1),
            Error::<Test>::EntityStakeCapExceeded
        );
        assert_noop!(
            crate::Pallet::<Test>::update_validator_stake(RuntimeOrigin::root(), 1, 0, 1),
            Error::<Test>::InvalidValidatorStake
        );
    });
}

#[test]
fn entity_cap_is_checked_globally_after_bootstrap() {
    new_test_ext().execute_with(|| {
        assert_ok!(crate::Pallet::<Test>::add_validator(
            RuntimeOrigin::root(),
            1,
            100,
            1,
        ));
        assert_noop!(
            crate::Pallet::<Test>::add_validator(RuntimeOrigin::root(), 2, 1, 2),
            Error::<Test>::EntityStakeCapExceeded
        );
    });
}

#[test]
fn submitted_weights_use_stake_snapshot_even_if_membership_changes_before_compute() {
    new_test_ext().execute_with(|| {
        assert_ok!(crate::Pallet::<Test>::add_validator(
            RuntimeOrigin::root(),
            1,
            100,
            1,
        ));
        assert_ok!(crate::Pallet::<Test>::add_validator(
            RuntimeOrigin::root(),
            2,
            100,
            2,
        ));
        rotate_permits(7);
        assert_ok!(crate::Pallet::<Test>::submit_weights(
            RuntimeOrigin::signed(1),
            7,
            sp_std::vec![(100, 1000)],
        ));
        assert_ok!(crate::Pallet::<Test>::submit_weights(
            RuntimeOrigin::signed(2),
            7,
            sp_std::vec![(101, 1000)],
        ));
        assert_ok!(crate::Pallet::<Test>::remove_validator(
            RuntimeOrigin::root(),
            1,
        ));
        assert_ok!(crate::Pallet::<Test>::compute_epoch_incentives(
            RuntimeOrigin::root(),
            7,
        ));
        assert_eq!(EpochIncentives::<Test>::get(7, 100), 5000);
        assert_eq!(EpochIncentives::<Test>::get(7, 101), 5000);
    });
}

#[test]
fn rotate_permits_selects_top_stake_validators() {
    new_test_ext().execute_with(|| {
        assert_ok!(crate::Pallet::<Test>::add_validator(
            RuntimeOrigin::root(),
            1,
            100,
            1,
        ));
        assert_ok!(crate::Pallet::<Test>::add_validator(
            RuntimeOrigin::root(),
            2,
            150,
            2,
        ));
        assert_ok!(crate::Pallet::<Test>::add_validator(
            RuntimeOrigin::root(),
            3,
            125,
            3,
        ));
        rotate_permits(7);
        assert!(!PermittedValidators::<Test>::contains_key(1));
        assert!(PermittedValidators::<Test>::contains_key(2));
        assert!(PermittedValidators::<Test>::contains_key(3));
        assert_noop!(
            crate::Pallet::<Test>::submit_weights(
                RuntimeOrigin::signed(1),
                7,
                sp_std::vec![(100, 1000)],
            ),
            Error::<Test>::UnauthorizedValidator
        );
        assert_ok!(crate::Pallet::<Test>::submit_weights(
            RuntimeOrigin::signed(2),
            7,
            sp_std::vec![(100, 1000)],
        ));
    });
}

#[test]
fn permits_are_epoch_scoped_and_cannot_rotate_after_submissions() {
    new_test_ext().execute_with(|| {
        assert_ok!(crate::Pallet::<Test>::add_validator(
            RuntimeOrigin::root(),
            1,
            100,
            1,
        ));
        rotate_permits(7);
        assert_ok!(crate::Pallet::<Test>::submit_weights(
            RuntimeOrigin::signed(1),
            7,
            sp_std::vec![(100, 1000)],
        ));
        assert_noop!(
            crate::Pallet::<Test>::submit_weights(
                RuntimeOrigin::signed(1),
                8,
                sp_std::vec![(100, 1000)],
            ),
            Error::<Test>::UnauthorizedValidator
        );
        assert_noop!(
            crate::Pallet::<Test>::rotate_permits(RuntimeOrigin::root(), 7),
            Error::<Test>::EpochAlreadyStarted
        );
    });
}

#[test]
fn epoch_permits_snapshot_validator_stake() {
    new_test_ext().execute_with(|| {
        assert_ok!(crate::Pallet::<Test>::add_validator(
            RuntimeOrigin::root(),
            1,
            100,
            1,
        ));
        rotate_permits(7);
        assert_ok!(crate::Pallet::<Test>::remove_validator(
            RuntimeOrigin::root(),
            1,
        ));
        assert_ok!(crate::Pallet::<Test>::add_validator(
            RuntimeOrigin::root(),
            1,
            500,
            1,
        ));
        assert_ok!(crate::Pallet::<Test>::submit_weights(
            RuntimeOrigin::signed(1),
            7,
            sp_std::vec![(100, 10)],
        ));
        assert_ok!(crate::Pallet::<Test>::compute_epoch_incentives(
            RuntimeOrigin::root(),
            7,
        ));
        assert_eq!(EpochScoreTotals::<Test>::get(7, 100), 1_000);
    });
}

#[test]
fn compute_clips_scores_to_operator_median() {
    new_test_ext().execute_with(|| {
        assert_ok!(crate::Pallet::<Test>::add_validator(
            RuntimeOrigin::root(),
            1,
            100,
            1,
        ));
        assert_ok!(crate::Pallet::<Test>::add_validator(
            RuntimeOrigin::root(),
            2,
            100,
            2,
        ));
        rotate_permits(7);
        assert_ok!(crate::Pallet::<Test>::submit_weights(
            RuntimeOrigin::signed(1),
            7,
            sp_std::vec![(100, 60_000)],
        ));
        assert_ok!(crate::Pallet::<Test>::submit_weights(
            RuntimeOrigin::signed(2),
            7,
            sp_std::vec![(100, 100)],
        ));
        assert_ok!(crate::Pallet::<Test>::compute_epoch_incentives(
            RuntimeOrigin::root(),
            7,
        ));
        assert_eq!(EpochScoreTotals::<Test>::get(7, 100), 20_000);
        assert_eq!(EpochIncentives::<Test>::get(7, 100), 10_000);
    });
}

#[test]
fn double_compute_rejected() {
    new_test_ext().execute_with(|| {
        assert_ok!(crate::Pallet::<Test>::compute_epoch_incentives(
            RuntimeOrigin::root(),
            42,
        ));
        assert_noop!(
            crate::Pallet::<Test>::compute_epoch_incentives(RuntimeOrigin::root(), 42),
            Error::<Test>::EpochAlreadyComputed
        );
    });
}

#[test]
fn empty_epoch_compiles() {
    new_test_ext().execute_with(|| {
        assert_ok!(crate::Pallet::<Test>::compute_epoch_incentives(
            RuntimeOrigin::root(),
            42,
        ));
    });
}

#[test]
fn compute_requires_authorized_origin() {
    new_test_ext().execute_with(|| {
        assert!(
            crate::Pallet::<Test>::compute_epoch_incentives(RuntimeOrigin::signed(1), 42,).is_err()
        );
    });
}
