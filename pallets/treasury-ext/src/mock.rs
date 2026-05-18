use crate as pallet_treasury_ext;
use frame_support::derive_impl;
use frame_support::traits::Contains;
use sp_runtime::BuildStorage;

type Block = frame_system::mocking::MockBlock<Test>;
pub type AccountId = u64;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        TreasuryExt: pallet_treasury_ext,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type Block = Block;
}

/// Mock council membership: accounts 1, 2, 3.
pub struct MockCouncil;
impl Contains<AccountId> for MockCouncil {
    fn contains(who: &AccountId) -> bool {
        matches!(*who, 1 | 2 | 3)
    }
}

impl pallet_treasury_ext::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type CouncilMembers = MockCouncil;
    type WeightInfo = ();
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    let mut ext: sp_io::TestExternalities = t.into();
    ext.execute_with(|| {
        frame_system::Pallet::<Test>::set_block_number(1);
        pallet_treasury_ext::Threshold::<Test>::put(2);
    });
    ext
}
