use crate::mock::*;
use crate::{ArbitrationVote, Error, FaultCode, MultisigDecision, SlashState, Slashes};
use frame_support::{assert_noop, assert_ok};
use sp_core::H256;

#[test]
fn slash_dispute_arbitrate_ratify_flow() {
    new_test_ext().execute_with(|| {
        assert_ok!(pallet_operator_stake::Pallet::<Test>::register(
            RuntimeOrigin::signed(42),
            1_000,
            H256::repeat_byte(1),
        ));
        assert_ok!(crate::Pallet::<Test>::submit_slashing_evidence(
            RuntimeOrigin::root(),
            42,
            FaultCode::WrongModel,
            H256::repeat_byte(7),
        ));
        let s = Slashes::<Test>::get(0).unwrap();
        assert_eq!(s.severity_bps, 1000);
        assert_eq!(s.state, SlashState::Pending);
        assert!(
            pallet_operator_stake::Operators::<Test>::get(42)
                .unwrap()
                .frozen
        );

        // Disputes are still signed-origin (operator under fire).
        assert_noop!(
            crate::Pallet::<Test>::dispute_slashing(
                RuntimeOrigin::signed(99),
                0,
                H256::repeat_byte(8),
            ),
            Error::<Test>::NotSlashOperator
        );
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
        assert_eq!(
            Slashes::<Test>::get(0).unwrap().state,
            SlashState::Arbitrated
        );

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

        // Finalize moves Ratified → Finalized without waiting for the
        // undisputed-pending window.
        assert_ok!(crate::Pallet::<Test>::finalize_slash(
            RuntimeOrigin::root(),
            0
        ));
        assert_eq!(
            Slashes::<Test>::get(0).unwrap().state,
            SlashState::Finalized
        );
        let op = pallet_operator_stake::Operators::<Test>::get(42).unwrap();
        assert_eq!(op.stake, 900);
        assert!(!op.frozen);
    });
}

#[test]
fn pending_slash_cannot_finalize_before_dispute_window() {
    new_test_ext().execute_with(|| {
        assert_ok!(pallet_operator_stake::Pallet::<Test>::register(
            RuntimeOrigin::signed(42),
            1_000,
            H256::repeat_byte(1),
        ));
        assert_ok!(crate::Pallet::<Test>::submit_slashing_evidence(
            RuntimeOrigin::root(),
            42,
            FaultCode::WrongModel,
            H256::repeat_byte(7),
        ));
        assert_noop!(
            crate::Pallet::<Test>::finalize_slash(RuntimeOrigin::root(), 0),
            Error::<Test>::DisputeWindowOpen
        );
        System::set_block_number(11);
        assert_ok!(crate::Pallet::<Test>::finalize_slash(
            RuntimeOrigin::root(),
            0
        ));
        assert_eq!(
            Slashes::<Test>::get(0).unwrap().state,
            SlashState::Finalized
        );
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
fn overturned_ratification_releases_freeze_without_slash() {
    new_test_ext().execute_with(|| {
        assert_ok!(pallet_operator_stake::Pallet::<Test>::register(
            RuntimeOrigin::signed(42),
            1_000,
            H256::repeat_byte(1),
        ));
        assert_ok!(crate::Pallet::<Test>::submit_slashing_evidence(
            RuntimeOrigin::root(),
            42,
            FaultCode::WrongModel,
            H256::repeat_byte(7),
        ));
        assert_ok!(crate::Pallet::<Test>::dispute_slashing(
            RuntimeOrigin::signed(42),
            0,
            H256::repeat_byte(8),
        ));
        assert_ok!(crate::Pallet::<Test>::arbitrate_dispute(
            RuntimeOrigin::root(),
            0,
            ArbitrationVote::Overturn,
        ));
        assert_ok!(crate::Pallet::<Test>::ratify_dispute(
            RuntimeOrigin::root(),
            0,
            MultisigDecision::Overturn,
        ));

        let op = pallet_operator_stake::Operators::<Test>::get(42).unwrap();
        assert_eq!(op.stake, 1_000);
        assert!(!op.frozen);
        assert_eq!(
            Slashes::<Test>::get(0).unwrap().state,
            SlashState::Finalized
        );
        assert_noop!(
            crate::Pallet::<Test>::finalize_slash(RuntimeOrigin::root(), 0),
            Error::<Test>::BadState
        );
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
