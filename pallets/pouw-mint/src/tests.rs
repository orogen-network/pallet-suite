use crate::mock::*;
use crate::{Error, Transcripts};
use frame_support::{assert_noop, assert_ok};
use sp_core::H256;

#[test]
fn transcript_submission_dedups() {
    new_test_ext().execute_with(|| {
        let h = H256::repeat_byte(5);
        assert_ok!(crate::Pallet::<Test>::submit_cupow_transcript(
            RuntimeOrigin::signed(1),
            h,
        ));
        assert!(Transcripts::<Test>::contains_key(h));
        assert_noop!(
            crate::Pallet::<Test>::submit_cupow_transcript(RuntimeOrigin::signed(1), h),
            Error::<Test>::DuplicateTranscript
        );
    });
}

#[test]
fn pouw_reward_disabled_at_tge() {
    new_test_ext().execute_with(|| {
        // Even root cannot emit while the lane is disabled.
        assert_noop!(
            crate::Pallet::<Test>::emit_pouw_reward(RuntimeOrigin::root(), 9, 100),
            Error::<Test>::Disabled
        );
    });
}

#[test]
fn pouw_reward_rejects_signed_origin() {
    new_test_ext().execute_with(|| {
        assert!(crate::Pallet::<Test>::emit_pouw_reward(RuntimeOrigin::signed(1), 9, 100).is_err());
    });
}
