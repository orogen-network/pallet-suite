use crate::mock::*;
use frame_support::assert_ok;

#[test]
fn submit_updates_twap_median() {
    new_test_ext().execute_with(|| {
        assert_ok!(crate::Pallet::<Test>::submit_price(
            RuntimeOrigin::root(),
            1_000_000,
        ));
        assert_eq!(crate::Pallet::<Test>::current_twap(), 1_000_000);
        // Bump the block so the rate-limit allows the next sample.
        frame_system::Pallet::<Test>::set_block_number(2);
        assert_ok!(crate::Pallet::<Test>::submit_price(
            RuntimeOrigin::root(),
            2_000_000,
        ));
        // Median of [1_000_000, 2_000_000] (sorted) at index len/2 = 1 is 2_000_000.
        assert_eq!(crate::Pallet::<Test>::current_twap(), 2_000_000);
        // A third sample anchors a true median.
        frame_system::Pallet::<Test>::set_block_number(3);
        assert_ok!(crate::Pallet::<Test>::submit_price(
            RuntimeOrigin::root(),
            500_000,
        ));
        assert_eq!(crate::Pallet::<Test>::current_twap(), 1_000_000);
    });
}

#[test]
fn empty_oracle_returns_zero() {
    new_test_ext().execute_with(|| {
        assert_eq!(crate::Pallet::<Test>::current_twap(), 0);
    });
}

#[test]
fn signed_origin_rejected() {
    new_test_ext().execute_with(|| {
        assert!(crate::Pallet::<Test>::submit_price(RuntimeOrigin::signed(1), 1_000_000).is_err());
    });
}
