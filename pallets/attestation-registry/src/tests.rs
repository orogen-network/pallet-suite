use crate::mock::*;
use crate::{Attestations, CrlKind};
use frame_support::assert_ok;
use sp_core::H256;

#[test]
fn submit_and_revoke_works() {
    new_test_ext().execute_with(|| {
        let h = H256::repeat_byte(1);
        assert_ok!(crate::Pallet::<Test>::submit(
            RuntimeOrigin::signed(1),
            h,
            H256::repeat_byte(2),
            0x1 | 0x2, // NVIDIA + IntelTDX
            H256::repeat_byte(3),
            100,
        ));
        assert!(Attestations::<Test>::get(h).is_some());

        // Signed origin cannot revoke.
        assert!(crate::Pallet::<Test>::revoke(RuntimeOrigin::signed(1), h).is_err());
        // Root can revoke.
        assert_ok!(crate::Pallet::<Test>::revoke(RuntimeOrigin::root(), h));
        assert!(Attestations::<Test>::get(h).unwrap().revoked);
    });
}

#[test]
fn crl_lookup_works() {
    new_test_ext().execute_with(|| {
        let target = H256::repeat_byte(9);
        assert!(!crate::Pallet::<Test>::is_revoked(CrlKind::FirmwareHash, target));
        // Signed cannot add to CRL.
        assert!(crate::Pallet::<Test>::add_to_crl(
            RuntimeOrigin::signed(1),
            CrlKind::FirmwareHash,
            target,
        )
        .is_err());
        // Root can.
        assert_ok!(crate::Pallet::<Test>::add_to_crl(
            RuntimeOrigin::root(),
            CrlKind::FirmwareHash,
            target,
        ));
        assert!(crate::Pallet::<Test>::is_revoked(CrlKind::FirmwareHash, target));
    });
}
