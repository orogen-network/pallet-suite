//! Hand-tuned weight defaults for `pallet-job-market`.

#![allow(clippy::unnecessary_cast)]

use frame_support::traits::Get;
use frame_support::weights::Weight;
use frame_system::Config;
use sp_std::marker::PhantomData;

pub trait WeightInfo {
    fn submit_job() -> Weight;
    fn assign() -> Weight;
    fn finalize() -> Weight;
    fn dispute() -> Weight;
}

pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: Config> WeightInfo for SubstrateWeight<T> {
    fn submit_job() -> Weight {
        Weight::from_parts(40_000_000, 4096)
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(1))
    }
    fn assign() -> Weight {
        Weight::from_parts(30_000_000, 2048)
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(1))
    }
    fn finalize() -> Weight {
        Weight::from_parts(30_000_000, 2048)
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(1))
    }
    fn dispute() -> Weight {
        Weight::from_parts(30_000_000, 2048)
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(1))
    }
}

impl WeightInfo for () {
    fn submit_job() -> Weight {
        Weight::from_parts(40_000_000, 4096)
    }
    fn assign() -> Weight {
        Weight::from_parts(30_000_000, 2048)
    }
    fn finalize() -> Weight {
        Weight::from_parts(30_000_000, 2048)
    }
    fn dispute() -> Weight {
        Weight::from_parts(30_000_000, 2048)
    }
}
