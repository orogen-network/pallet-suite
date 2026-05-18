use crate::mock::*;
use crate::Error;
use frame_support::{assert_noop, assert_ok};
use sp_core::H256;

#[test]
fn first_use_succeeds_replay_fails() {
    new_test_ext().execute_with(|| {
        let n = H256::repeat_byte(1);
        // Root succeeds, signed fails.
        assert!(crate::Pallet::<Test>::record_nonce(RuntimeOrigin::signed(1), n).is_err());
        assert_ok!(crate::Pallet::<Test>::record_nonce(RuntimeOrigin::root(), n));
        assert!(!crate::Pallet::<Test>::check_nonce(n));
        assert_noop!(
            crate::Pallet::<Test>::record_nonce(RuntimeOrigin::root(), n),
            Error::<Test>::Replay
        );
    });
}

#[test]
fn unseen_nonce_passes_check() {
    new_test_ext().execute_with(|| {
        assert!(crate::Pallet::<Test>::check_nonce(H256::repeat_byte(9)));
    });
}
