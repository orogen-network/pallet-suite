//! Hand-tuned weight defaults for `pallet-model-registry`.

#![allow(clippy::unnecessary_cast)]

use frame_support::traits::Get;
use frame_support::weights::Weight;
use frame_system::Config;
use sp_std::marker::PhantomData;

pub trait WeightInfo {
    fn register_base_model() -> Weight;
    fn register_adapter() -> Weight;
    fn deprecate() -> Weight;
}

pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: Config> WeightInfo for SubstrateWeight<T> {
    fn register_base_model() -> Weight {
        Weight::from_parts(40_000_000, 4096)
            .saturating_add(T::DbWeight::get().reads(2))
            .saturating_add(T::DbWeight::get().writes(1))
    }
    fn register_adapter() -> Weight {
        Weight::from_parts(40_000_000, 4096)
            .saturating_add(T::DbWeight::get().reads(3))
            .saturating_add(T::DbWeight::get().writes(1))
    }
    fn deprecate() -> Weight {
        Weight::from_parts(40_000_000, 4096)
            .saturating_add(T::DbWeight::get().reads(2))
            .saturating_add(T::DbWeight::get().writes(2))
    }
}

impl WeightInfo for () {
    fn register_base_model() -> Weight {
        Weight::from_parts(40_000_000, 4096)
    }
    fn register_adapter() -> Weight {
        Weight::from_parts(40_000_000, 4096)
    }
    fn deprecate() -> Weight {
        Weight::from_parts(40_000_000, 4096)
    }
}
