use crate::mock::*;
use crate::{Error, Nonces, PRUNE_BATCH_PER_BLOCK, PRUNE_SCAN_LIMIT_PER_BLOCK};
use frame_support::{assert_noop, assert_ok, traits::Hooks};
use sp_core::H256;

#[test]
fn first_use_succeeds_replay_fails() {
    new_test_ext().execute_with(|| {
        let n = H256::repeat_byte(1);
        // Root succeeds, signed fails.
        assert!(crate::Pallet::<Test>::record_nonce(RuntimeOrigin::signed(1), n).is_err());
        assert_ok!(crate::Pallet::<Test>::record_nonce(
            RuntimeOrigin::root(),
            n
        ));
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

#[test]
fn prune_scan_is_bounded_even_when_entries_are_expired() {
    new_test_ext().execute_with(|| {
        for i in 0..(PRUNE_SCAN_LIMIT_PER_BLOCK + 10) {
            Nonces::<Test>::insert(H256::from_low_u64_be(i as u64), 1);
        }
        frame_system::Pallet::<Test>::set_block_number(20_000);
        let _ = crate::Pallet::<Test>::on_initialize(20_000);
        let remaining = Nonces::<Test>::iter().count() as u32;
        assert_eq!(
            remaining,
            PRUNE_SCAN_LIMIT_PER_BLOCK + 10 - PRUNE_BATCH_PER_BLOCK
        );
    });
}
