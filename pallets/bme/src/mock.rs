use crate as pallet_bme;
use frame_support::derive_impl;
use frame_system::EnsureRoot;
use sp_runtime::BuildStorage;

type Block = frame_system::mocking::MockBlock<Test>;
pub type AccountId = u64;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        Bme: pallet_bme,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type Block = Block;
}

impl pallet_bme::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type GatewayOrigin = EnsureRoot<AccountId>;
    type MintOrigin = EnsureRoot<AccountId>;
    type GovernanceOrigin = EnsureRoot<AccountId>;
    type WeightInfo = ();
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    let mut ext: sp_io::TestExternalities = t.into();
    ext.execute_with(|| {
        frame_system::Pallet::<Test>::set_block_number(1);
        // Default elasticity 1.0 (10_000 bps).
        pallet_bme::Elasticity::<Test>::put(10_000);
    });
    ext
}
