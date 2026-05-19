use crate as pallet_yuma_consensus;
use frame_support::derive_impl;
use frame_system::EnsureRoot;
use sp_runtime::BuildStorage;

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        YumaConsensus: pallet_yuma_consensus,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type Block = Block;
}

impl pallet_yuma_consensus::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type GovernanceOrigin = EnsureRoot<u64>;
    type MaxValidators = frame_support::traits::ConstU32<3>;
    type MaxPermittedValidators = frame_support::traits::ConstU32<2>;
    type MaxWeightVectorLen = frame_support::traits::ConstU32<4>;
    type MaxEntityStakeBps = frame_support::traits::ConstU16<6_000>;
    type ComputeOrigin = EnsureRoot<u64>;
    type WeightInfo = ();
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    let mut ext: sp_io::TestExternalities = t.into();
    ext.execute_with(|| frame_system::Pallet::<Test>::set_block_number(1));
    ext
}
