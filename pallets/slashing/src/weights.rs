//! Hand-tuned weight defaults for `pallet-slashing`.

#![allow(clippy::unnecessary_cast)]

use frame_support::traits::Get;
use frame_support::weights::Weight;
use frame_system::Config;
use sp_std::marker::PhantomData;

pub trait WeightInfo {
    fn submit_slashing_evidence() -> Weight;
    fn dispute_slashing() -> Weight;
    fn arbitrate_dispute() -> Weight;
    fn ratify_dispute() -> Weight;
    fn finalize_slash() -> Weight;
}

pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: Config> WeightInfo for SubstrateWeight<T> {
    fn submit_slashing_evidence() -> Weight {
        Weight::from_parts(40_000_000, 4096)
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(2))
    }
    fn dispute_slashing() -> Weight {
        Weight::from_parts(30_000_000, 2048)
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(1))
    }
    fn arbitrate_dispute() -> Weight {
        Weight::from_parts(30_000_000, 2048)
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(1))
    }
    fn ratify_dispute() -> Weight {
        Weight::from_parts(30_000_000, 2048)
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(1))
    }
    fn finalize_slash() -> Weight {
        Weight::from_parts(30_000_000, 2048)
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(1))
    }
}

impl WeightInfo for () {
    fn submit_slashing_evidence() -> Weight {
        Weight::from_parts(40_000_000, 4096)
    }
    fn dispute_slashing() -> Weight {
        Weight::from_parts(30_000_000, 2048)
    }
    fn arbitrate_dispute() -> Weight {
        Weight::from_parts(30_000_000, 2048)
    }
    fn ratify_dispute() -> Weight {
        Weight::from_parts(30_000_000, 2048)
    }
    fn finalize_slash() -> Weight {
        Weight::from_parts(30_000_000, 2048)
    }
}
