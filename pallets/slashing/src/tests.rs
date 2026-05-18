use crate::mock::*;
use crate::{ArbitrationVote, FaultCode, MultisigDecision, SlashState, Slashes};
use frame_support::assert_ok;
use sp_core::H256;

#[test]
fn slash_dispute_arbitrate_ratify_flow() {
    new_test_ext().execute_with(|| {
        assert_ok!(crate::Pallet::<Test>::submit_slashing_evidence(
            RuntimeOrigin::root(),
            42,
            FaultCode::WrongModel,
            H256::repeat_byte(7),
        ));
        let s = Slashes::<Test>::get(0).unwrap();
        assert_eq!(s.severity_bps, 1000);
        assert_eq!(s.state, SlashState::Pending);

        // Disputes are still signed-origin (operator under fire).
        assert_ok!(crate::Pallet::<Test>::dispute_slashing(
            RuntimeOrigin::signed(42),
            0,
            H256::repeat_byte(8),
        ));
        assert_eq!(Slashes::<Test>::get(0).unwrap().state, SlashState::Disputed);

        // Arbitrate / ratify require panel (root) origin.
        assert!(crate::Pallet::<Test>::arbitrate_dispute(
            RuntimeOrigin::signed(99),
            0,
            ArbitrationVote::Uphold,
        )
        .is_err());
        assert_ok!(crate::Pallet::<Test>::arbitrate_dispute(
            RuntimeOrigin::root(),
            0,
            ArbitrationVote::Uphold,
        ));
        assert_eq!(Slashes::<Test>::get(0).unwrap().state, SlashState::Arbitrated);

        assert!(crate::Pallet::<Test>::ratify_dispute(
            RuntimeOrigin::signed(100),
            0,
            MultisigDecision::Uphold,
        )
        .is_err());
        assert_ok!(crate::Pallet::<Test>::ratify_dispute(
            RuntimeOrigin::root(),
            0,
            MultisigDecision::Uphold,
        ));
        assert_eq!(Slashes::<Test>::get(0).unwrap().state, SlashState::Ratified);

        // Finalize moves Ratified → Finalized.
        assert_ok!(crate::Pallet::<Test>::finalize_slash(RuntimeOrigin::root(), 0));
        assert_eq!(Slashes::<Test>::get(0).unwrap().state, SlashState::Finalized);
    });
}

#[test]
fn submit_requires_evidence_origin() {
    new_test_ext().execute_with(|| {
        assert!(crate::Pallet::<Test>::submit_slashing_evidence(
            RuntimeOrigin::signed(1),
            42,
            FaultCode::WrongModel,
            H256::repeat_byte(7),
        )
        .is_err());
    });
}

#[test]
fn fault_severity_table_matches_rfc() {
    assert_eq!(FaultCode::WrongModel.base_severity_bps(), 1000);
    assert_eq!(FaultCode::WrongResponse.base_severity_bps(), 500);
    assert_eq!(FaultCode::DeviceCertCollision.base_severity_bps(), 10_000);
    assert_eq!(FaultCode::FakeBurn.base_severity_bps(), 5000);
    assert_eq!(FaultCode::HeartbeatMiss.base_severity_bps(), 0);
}
