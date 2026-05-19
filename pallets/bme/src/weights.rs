//! Hand-tuned weight defaults for `pallet-bme`. Replace with benchmark output.

#![allow(clippy::unnecessary_cast)]

use frame_support::traits::Get;
use frame_support::weights::Weight;
use frame_system::Config;
use sp_std::marker::PhantomData;

pub trait WeightInfo {
    fn submit_burn() -> Weight;
    fn mint_to_operator() -> Weight;
    fn set_elasticity() -> Weight;
}

pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: Config> WeightInfo for SubstrateWeight<T> {
    fn submit_burn() -> Weight {
        Weight::from_parts(30_000_000, 2048)
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(1))
    }
    fn mint_to_operator() -> Weight {
        Weight::from_parts(40_000_000, 4096)
            .saturating_add(T::DbWeight::get().reads(4))
            .saturating_add(T::DbWeight::get().writes(2))
    }
    fn set_elasticity() -> Weight {
        Weight::from_parts(20_000_000, 1024).saturating_add(T::DbWeight::get().writes(1))
    }
}

impl WeightInfo for () {
    fn submit_burn() -> Weight {
        Weight::from_parts(30_000_000, 2048)
    }
    fn mint_to_operator() -> Weight {
        Weight::from_parts(40_000_000, 4096)
    }
    fn set_elasticity() -> Weight {
        Weight::from_parts(20_000_000, 1024)
    }
}
