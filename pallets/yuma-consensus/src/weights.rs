//! Hand-tuned weight defaults for `pallet-yuma-consensus`.

#![allow(clippy::unnecessary_cast)]

use frame_support::traits::Get;
use frame_support::weights::Weight;
use frame_system::Config;
use sp_std::marker::PhantomData;

pub trait WeightInfo {
    fn submit_weights() -> Weight;
    fn compute_epoch_incentives() -> Weight;
}

pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: Config> WeightInfo for SubstrateWeight<T> {
    fn submit_weights() -> Weight {
        Weight::from_parts(80_000_000, 16_384)
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(1))
    }
    fn compute_epoch_incentives() -> Weight {
        // Worst-case: MAX_VALIDATORS_PER_EPOCH (1024) × 4096 inner ops.
        Weight::from_parts(500_000_000_000u64, 65_536)
            .saturating_add(T::DbWeight::get().reads(1024))
            .saturating_add(T::DbWeight::get().writes(1024))
    }
}

impl WeightInfo for () {
    fn submit_weights() -> Weight {
        Weight::from_parts(80_000_000, 16_384)
    }
    fn compute_epoch_incentives() -> Weight {
        Weight::from_parts(500_000_000, 65_536)
    }
}
