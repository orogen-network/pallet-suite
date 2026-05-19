use crate as pallet_slashing;
use frame_support::{derive_impl, parameter_types};
use frame_system::EnsureRoot;
use sp_runtime::BuildStorage;

type Block = frame_system::mocking::MockBlock<Test>;
pub type AccountId = u64;
pub type Balance = u128;

parameter_types! {
    pub const MinOperatorStake: Balance = 100;
    pub const MaxHeartbeatEpochAdvance: u64 = 100;
    pub const DisputeWindow: u64 = 10;
}

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        Balances: pallet_balances,
        OperatorStake: pallet_operator_stake,
        Slashing: pallet_slashing,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type Block = Block;
    type AccountData = pallet_balances::AccountData<Balance>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
    type Balance = Balance;
    type AccountStore = System;
}

impl pallet_operator_stake::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type SlashOrigin = EnsureRoot<AccountId>;
    type MinStake = MinOperatorStake;
    type MaxHeartbeatEpochAdvance = MaxHeartbeatEpochAdvance;
    type WeightInfo = ();
}

impl pallet_slashing::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type EvidenceOrigin = EnsureRoot<AccountId>;
    type PanelOrigin = EnsureRoot<AccountId>;
    type WeightInfo = ();
    type OperatorSlash = OperatorStake;
    type DisputeWindow = DisputeWindow;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    pallet_balances::GenesisConfig::<Test> {
        balances: vec![(42, 1_000_000_000), (99, 1_000_000_000)],
        ..Default::default()
    }
    .assimilate_storage(&mut t)
    .unwrap();
    let mut ext: sp_io::TestExternalities = t.into();
    ext.execute_with(|| frame_system::Pallet::<Test>::set_block_number(1));
    ext
}
