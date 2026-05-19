//! Orogen runtime composition.
//!
//! Wires every pallet in the suite into a single `Runtime` enum via
//! `construct_runtime!`, plus the standard FRAME utility pallets
//! (`pallet-balances`, `pallet-aura`, `pallet-grandpa`, `pallet-timestamp`,
//! `pallet-transaction-payment`, `pallet-sudo`) needed to make the chain
//! actually produce blocks under a real Substrate service.
//!
//! Configuration here is dev-grade: small block weights, no economic
//! tuning. Mainnet / testnet wiring lives in the dedicated
//! `runtime-mainnet` / `runtime-testnet` crates.

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

extern crate alloc;

// Make the WASM binary visible to consumers (notably `chain-node` when it
// builds the chain spec). Populated by `substrate-wasm-builder` via
// `build.rs`.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use sp_api::impl_runtime_apis;
use sp_core::OpaqueMetadata;
use sp_runtime::{
    generic, impl_opaque_keys,
    traits::{
        AccountIdLookup, BlakeTwo256, Block as BlockT, IdentifyAccount, NumberFor, One, Verify,
    },
    transaction_validity::{TransactionSource, TransactionValidity},
    ApplyExtrinsicResult, MultiSignature,
};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

use frame_support::{
    construct_runtime, derive_impl, parameter_types,
    traits::{ConstU32, ConstU64, ConstU8},
    weights::{
        constants::{
            BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND,
        },
        IdentityFee, Weight,
    },
};
use frame_system::limits::{BlockLength, BlockWeights};
pub use frame_system::Call as SystemCall;
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;
use pallet_transaction_payment::{ConstFeeMultiplier, FungibleAdapter, Multiplier};

// ---------------------------------------------------------------------------
// Primitive type aliases
// ---------------------------------------------------------------------------

/// Signature scheme over runtime calls.
pub type Signature = MultiSignature;
/// AccountId derived from the public-key half of the signature.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
/// Numeric balance type used by `pallet-balances` and downstream pallets.
pub type Balance = u128;
/// Block number type.
pub type BlockNumber = u32;
/// Generic hash type.
pub type Hash = sp_core::H256;
/// Account-nonce / index.
pub type Nonce = u32;

/// Header type for the runtime block.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Concrete runtime block type. Replaces the test-only
/// `frame_system::mocking::MockBlock` used in the original skeleton.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// Unchecked extrinsic, parametric over the runtime.
pub type UncheckedExtrinsic =
    generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;
/// Address shape used by `pallet-balances::transfer_allow_death` etc.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
/// Block id alias used by runtime APIs.
pub type BlockId = generic::BlockId<Block>;
/// Signed extension stack applied to every extrinsic.
pub type SignedExtra = (
    frame_system::CheckNonZeroSender<Runtime>,
    frame_system::CheckSpecVersion<Runtime>,
    frame_system::CheckTxVersion<Runtime>,
    frame_system::CheckGenesis<Runtime>,
    frame_system::CheckEra<Runtime>,
    frame_system::CheckNonce<Runtime>,
    frame_system::CheckWeight<Runtime>,
    pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);
/// Executive: dispatches extrinsics, runs on-runtime-upgrade hooks, etc.
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPalletsWithSystem,
>;
/// Convenience alias for tests/RPC callers needing the runtime hash type.
pub type AccountIndex = u32;

// ---------------------------------------------------------------------------
// Opaque (client-side) types
// ---------------------------------------------------------------------------

/// Opaque types used by the network layer (client) that don't depend on the
/// concrete runtime.
pub mod opaque {
    use super::*;

    /// Opaque block header (matches runtime header shape).
    pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
    /// Opaque block type — extrinsics aren't decoded.
    pub type Block = generic::Block<Header, sp_runtime::OpaqueExtrinsic>;
    /// Opaque block ID.
    pub type BlockId = generic::BlockId<Block>;

    impl_opaque_keys! {
        pub struct SessionKeys {
            pub aura: Aura,
            pub grandpa: Grandpa,
        }
    }
}

// ---------------------------------------------------------------------------
// Runtime version
// ---------------------------------------------------------------------------

#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
    // Runtime identity for the Orogen testnet/genesis line. Do not apply this
    // identity change as a live upgrade over a chain using a different
    // `spec_name`.
    spec_name: alloc::borrow::Cow::Borrowed("orogen"),
    impl_name: alloc::borrow::Cow::Borrowed("orogen"),
    authoring_version: 1,
    // Bumped from 5 to 6: Yuma submission authorization now uses governed
    // top-K permits and epoch aggregation clips scores to operator medians.
    spec_version: 6,
    impl_version: 1,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 6,
    system_version: 1,
};

#[cfg(feature = "std")]
/// The version information used to identify this runtime when compiled
/// natively.
pub fn native_version() -> NativeVersion {
    NativeVersion {
        runtime_version: VERSION,
        can_author_with: Default::default(),
    }
}

// ---------------------------------------------------------------------------
// Block-time / weight constants
// ---------------------------------------------------------------------------

/// Target block time (milliseconds). 6 seconds matches the RFC-0003 cadence
/// assumed by `pallet-nonce-vault::REPLAY_WINDOW_BLOCKS`.
pub const MILLISECS_PER_BLOCK: u64 = 6_000;
pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

/// Maximum number of authorities allowed in Aura / Grandpa.
pub const MAX_AUTHORITIES: u32 = 32;

/// 2 second computation time per block. Standard substrate-node-template
/// dev value.
const MAXIMUM_BLOCK_WEIGHT: Weight =
    Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND.saturating_mul(2), u64::MAX);

const NORMAL_DISPATCH_RATIO: sp_runtime::Perbill = sp_runtime::Perbill::from_percent(75);

parameter_types! {
    pub const BlockHashCount: BlockNumber = 2400;
    pub const Version: RuntimeVersion = VERSION;
    pub RuntimeBlockLength: BlockLength = {
        #[allow(deprecated)]
        BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO)
    };
    pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
        .base_block(BlockExecutionWeight::get())
        .for_class(frame_support::dispatch::DispatchClass::all(), |weights| {
            weights.base_extrinsic = ExtrinsicBaseWeight::get();
        })
        .for_class(frame_support::dispatch::DispatchClass::Normal, |weights| {
            weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
        })
        .for_class(frame_support::dispatch::DispatchClass::Operational, |weights| {
            weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
            weights.reserved = Some(
                MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT,
            );
        })
        .avg_block_initialization(sp_runtime::Perbill::from_percent(10))
        .build_or_panic();
    pub const SS58Prefix: u8 = 42;
}

// ---------------------------------------------------------------------------
// FRAME system + utility pallet configs
// ---------------------------------------------------------------------------

#[derive_impl(frame_system::config_preludes::SolochainDefaultConfig)]
impl frame_system::Config for Runtime {
    type BlockWeights = RuntimeBlockWeights;
    type BlockLength = RuntimeBlockLength;
    type AccountId = AccountId;
    type Nonce = Nonce;
    type Hash = Hash;
    type Hashing = BlakeTwo256;
    type Lookup = AccountIdLookup<AccountId, ()>;
    type Block = Block;
    type RuntimeEvent = RuntimeEvent;
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type RuntimeTask = RuntimeTask;
    type BlockHashCount = BlockHashCount;
    type DbWeight = RocksDbWeight;
    type Version = Version;
    type AccountData = pallet_balances::AccountData<Balance>;
    type SS58Prefix = SS58Prefix;
}

impl pallet_aura::Config for Runtime {
    type AuthorityId = sp_consensus_aura::sr25519::AuthorityId;
    type DisabledValidators = ();
    type MaxAuthorities = ConstU32<MAX_AUTHORITIES>;
    type AllowMultipleBlocksPerSlot = frame_support::traits::ConstBool<false>;
    type SlotDuration = pallet_aura::MinimumPeriodTimesTwo<Runtime>;
}

impl pallet_grandpa::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type MaxAuthorities = ConstU32<MAX_AUTHORITIES>;
    type MaxNominators = ConstU32<0>;
    type MaxSetIdSessionEntries = ConstU64<0>;
    type KeyOwnerProof = sp_core::Void;
    type EquivocationReportSystem = ();
}

impl pallet_timestamp::Config for Runtime {
    type Moment = u64;
    type OnTimestampSet = Aura;
    type MinimumPeriod = ConstU64<{ SLOT_DURATION / 2 }>;
    type WeightInfo = ();
}

parameter_types! {
    pub const ExistentialDeposit: Balance = 500;
    pub const MinOperatorStake: Balance = 1_000_000_000_000;
    pub const MaxHeartbeatEpochAdvance: u64 = 1;
    pub const SlashDisputeWindow: BlockNumber = 7_200;
    pub const MaxYumaValidators: u32 = 64;
    pub const MaxYumaPermittedValidators: u32 = 64;
    pub const MaxYumaWeightVectorLen: u32 = 256;
    pub const MaxYumaEntityStakeBps: u16 = 2_000;
}

impl pallet_balances::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
    type Balance = Balance;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type ReserveIdentifier = [u8; 8];
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type FreezeIdentifier = RuntimeFreezeReason;
    type MaxLocks = ConstU32<50>;
    type MaxReserves = ConstU32<50>;
    type MaxFreezes = ConstU32<50>;
    type DoneSlashHandler = ();
}

parameter_types! {
    pub FeeMultiplier: Multiplier = Multiplier::one();
}

impl pallet_transaction_payment::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnChargeTransaction = FungibleAdapter<Balances, ()>;
    type OperationalFeeMultiplier = ConstU8<5>;
    type WeightToFee = IdentityFee<Balance>;
    type LengthToFee = IdentityFee<Balance>;
    type FeeMultiplierUpdate = ConstFeeMultiplier<FeeMultiplier>;
    type WeightInfo = ();
}

#[cfg(feature = "dev-runtime")]
impl pallet_sudo::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type WeightInfo = ();
}

// ---------------------------------------------------------------------------
// Orogen custom pallet configs.
//
// All privileged origins are wired to `EnsureRoot` for the dev-grade
// runtime. Production wiring (council, gateway-multisig, slashing-panel,
// oracle-reporter-set) should replace these per-origin in a mainnet runtime
// crate once that exists. `EnsureRoot` is correct and minimal — it locks
// every privileged call behind sudo / on-chain governance instead of any
// signed account.
// ---------------------------------------------------------------------------

use frame_support::traits::Nothing;
use frame_system::EnsureRoot;

impl pallet_model_registry::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
}
impl pallet_operator_stake::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type SlashOrigin = EnsureRoot<AccountId>;
    type MinStake = MinOperatorStake;
    type MaxHeartbeatEpochAdvance = MaxHeartbeatEpochAdvance;
    type WeightInfo = pallet_operator_stake::weights::SubstrateWeight<Runtime>;
}
impl pallet_job_market::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type GatewayOrigin = EnsureRoot<AccountId>;
    type WeightInfo = ();
}
impl pallet_yuma_consensus::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type GovernanceOrigin = EnsureRoot<AccountId>;
    type MaxValidators = MaxYumaValidators;
    type MaxPermittedValidators = MaxYumaPermittedValidators;
    type MaxWeightVectorLen = MaxYumaWeightVectorLen;
    type MaxEntityStakeBps = MaxYumaEntityStakeBps;
    type ComputeOrigin = EnsureRoot<AccountId>;
    type WeightInfo = pallet_yuma_consensus::weights::SubstrateWeight<Runtime>;
}
impl pallet_bme::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type GatewayOrigin = EnsureRoot<AccountId>;
    type MintOrigin = EnsureRoot<AccountId>;
    type GovernanceOrigin = EnsureRoot<AccountId>;
    type WeightInfo = ();
}
impl pallet_slashing::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type EvidenceOrigin = EnsureRoot<AccountId>;
    type PanelOrigin = EnsureRoot<AccountId>;
    type WeightInfo = pallet_slashing::weights::SubstrateWeight<Runtime>;
    type OperatorSlash = OperatorStake;
    type DisputeWindow = SlashDisputeWindow;
}
impl pallet_pouw_mint::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type PoUWRewardOrigin = EnsureRoot<AccountId>;
    type WeightInfo = ();
}
impl pallet_attestation_registry::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AdminOrigin = EnsureRoot<AccountId>;
    type WeightInfo = ();
}
impl pallet_oracle_twap::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ReporterOrigin = EnsureRoot<AccountId>;
    type WeightInfo = ();
}
impl pallet_nonce_vault::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type GatewayOrigin = EnsureRoot<AccountId>;
    type WeightInfo = pallet_nonce_vault::weights::SubstrateWeight<Runtime>;
}
impl pallet_treasury_ext::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    // Default to empty council set in the dev runtime — production wires
    // a `Contains` impl backed by an on-chain `Members` storage.
    type CouncilMembers = Nothing;
    type WeightInfo = ();
}

// ---------------------------------------------------------------------------
// Runtime composition
// ---------------------------------------------------------------------------

// `pallet-sudo` is included only behind the `dev-runtime` feature.
// Production builds must NOT compile with `dev-runtime` enabled.
#[cfg(feature = "dev-runtime")]
construct_runtime!(
    pub enum Runtime {
        System: frame_system = 0,
        Timestamp: pallet_timestamp = 1,
        Aura: pallet_aura = 2,
        Grandpa: pallet_grandpa = 3,
        Balances: pallet_balances = 4,
        TransactionPayment: pallet_transaction_payment = 5,
        Sudo: pallet_sudo = 6,

        // Orogen pallets
        ModelRegistry: pallet_model_registry = 10,
        OperatorStake: pallet_operator_stake = 11,
        JobMarket: pallet_job_market = 12,
        YumaConsensus: pallet_yuma_consensus = 13,
        Bme: pallet_bme = 14,
        Slashing: pallet_slashing = 15,
        PouwMint: pallet_pouw_mint = 16,
        AttestationRegistry: pallet_attestation_registry = 17,
        OracleTwap: pallet_oracle_twap = 18,
        NonceVault: pallet_nonce_vault = 19,
        TreasuryExt: pallet_treasury_ext = 20,
    }
);

#[cfg(not(feature = "dev-runtime"))]
construct_runtime!(
    pub enum Runtime {
        System: frame_system = 0,
        Timestamp: pallet_timestamp = 1,
        Aura: pallet_aura = 2,
        Grandpa: pallet_grandpa = 3,
        Balances: pallet_balances = 4,
        TransactionPayment: pallet_transaction_payment = 5,

        // Orogen pallets
        ModelRegistry: pallet_model_registry = 10,
        OperatorStake: pallet_operator_stake = 11,
        JobMarket: pallet_job_market = 12,
        YumaConsensus: pallet_yuma_consensus = 13,
        Bme: pallet_bme = 14,
        Slashing: pallet_slashing = 15,
        PouwMint: pallet_pouw_mint = 16,
        AttestationRegistry: pallet_attestation_registry = 17,
        OracleTwap: pallet_oracle_twap = 18,
        NonceVault: pallet_nonce_vault = 19,
        TreasuryExt: pallet_treasury_ext = 20,
    }
);

// ---------------------------------------------------------------------------
// Runtime APIs
// ---------------------------------------------------------------------------

impl_runtime_apis! {
    impl sp_api::Core<Block> for Runtime {
        fn version() -> RuntimeVersion {
            VERSION
        }

        fn execute_block(block: <Block as BlockT>::LazyBlock) {
            Executive::execute_block(block);
        }

        fn initialize_block(header: &<Block as BlockT>::Header) -> sp_runtime::ExtrinsicInclusionMode {
            Executive::initialize_block(header)
        }
    }

    impl sp_api::Metadata<Block> for Runtime {
        fn metadata() -> OpaqueMetadata {
            OpaqueMetadata::new(Runtime::metadata().into())
        }

        fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
            Runtime::metadata_at_version(version)
        }

        fn metadata_versions() -> Vec<u32> {
            Runtime::metadata_versions()
        }
    }

    impl sp_block_builder::BlockBuilder<Block> for Runtime {
        fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
            Executive::apply_extrinsic(extrinsic)
        }

        fn finalize_block() -> <Block as BlockT>::Header {
            Executive::finalize_block()
        }

        fn inherent_extrinsics(
            data: sp_inherents::InherentData,
        ) -> Vec<<Block as BlockT>::Extrinsic> {
            data.create_extrinsics()
        }

        fn check_inherents(
            block: <Block as BlockT>::LazyBlock,
            data: sp_inherents::InherentData,
        ) -> sp_inherents::CheckInherentsResult {
            data.check_extrinsics(&block)
        }
    }

    impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
        fn validate_transaction(
            source: TransactionSource,
            tx: <Block as BlockT>::Extrinsic,
            block_hash: <Block as BlockT>::Hash,
        ) -> TransactionValidity {
            Executive::validate_transaction(source, tx, block_hash)
        }
    }

    impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(header: &<Block as BlockT>::Header) {
            Executive::offchain_worker(header)
        }
    }

    impl sp_consensus_aura::AuraApi<Block, sp_consensus_aura::sr25519::AuthorityId> for Runtime {
        fn slot_duration() -> sp_consensus_aura::SlotDuration {
            sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
        }

        fn authorities() -> Vec<sp_consensus_aura::sr25519::AuthorityId> {
            pallet_aura::Authorities::<Runtime>::get().into_inner()
        }
    }

    impl sp_consensus_grandpa::GrandpaApi<Block> for Runtime {
        fn grandpa_authorities() -> sp_consensus_grandpa::AuthorityList {
            Grandpa::grandpa_authorities()
        }

        fn current_set_id() -> sp_consensus_grandpa::SetId {
            Grandpa::current_set_id()
        }

        fn submit_report_equivocation_unsigned_extrinsic(
            _equivocation_proof: sp_consensus_grandpa::EquivocationProof<
                <Block as BlockT>::Hash,
                NumberFor<Block>,
            >,
            _key_owner_proof: sp_consensus_grandpa::OpaqueKeyOwnershipProof,
        ) -> Option<()> {
            None
        }

        fn generate_key_ownership_proof(
            _set_id: sp_consensus_grandpa::SetId,
            _authority_id: sp_consensus_grandpa::AuthorityId,
        ) -> Option<sp_consensus_grandpa::OpaqueKeyOwnershipProof> {
            None
        }
    }

    impl sp_session::SessionKeys<Block> for Runtime {
        fn generate_session_keys(
            owner: Vec<u8>,
            seed: Option<Vec<u8>>,
        ) -> sp_session::OpaqueGeneratedSessionKeys {
            opaque::SessionKeys::generate(&owner, seed).into()
        }

        fn decode_session_keys(
            encoded: Vec<u8>,
        ) -> Option<Vec<(Vec<u8>, sp_core::crypto::KeyTypeId)>> {
            opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
        }
    }

    impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
        fn account_nonce(account: AccountId) -> Nonce {
            System::account_nonce(account)
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
        fn query_info(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_info(uxt, len)
        }

        fn query_fee_details(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment::FeeDetails<Balance> {
            TransactionPayment::query_fee_details(uxt, len)
        }

        fn query_weight_to_fee(weight: Weight) -> Balance {
            TransactionPayment::weight_to_fee(weight)
        }

        fn query_length_to_fee(length: u32) -> Balance {
            TransactionPayment::length_to_fee(length)
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall>
        for Runtime
    {
        fn query_call_info(
            call: RuntimeCall,
            len: u32,
        ) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_call_info(call, len)
        }

        fn query_call_fee_details(
            call: RuntimeCall,
            len: u32,
        ) -> pallet_transaction_payment::FeeDetails<Balance> {
            TransactionPayment::query_call_fee_details(call, len)
        }

        fn query_weight_to_fee(weight: Weight) -> Balance {
            TransactionPayment::weight_to_fee(weight)
        }

        fn query_length_to_fee(length: u32) -> Balance {
            TransactionPayment::length_to_fee(length)
        }
    }

    impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {
        fn build_state(config: Vec<u8>) -> sp_genesis_builder::Result {
            frame_support::genesis_builder_helper::build_state::<RuntimeGenesisConfig>(config)
        }

        fn get_preset(id: &Option<sp_genesis_builder::PresetId>) -> Option<Vec<u8>> {
            frame_support::genesis_builder_helper::get_preset::<RuntimeGenesisConfig>(id, |_| None)
        }

        fn preset_names() -> Vec<sp_genesis_builder::PresetId> {
            vec![]
        }
    }
}
