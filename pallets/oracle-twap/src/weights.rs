//! Hand-tuned weight defaults for `pallet-oracle-twap`.

#![allow(clippy::unnecessary_cast)]

use frame_support::traits::Get;
use frame_support::weights::Weight;
use frame_system::Config;
use sp_std::marker::PhantomData;

pub trait WeightInfo {
    fn submit_price() -> Weight;
}

pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: Config> WeightInfo for SubstrateWeight<T> {
    fn submit_price() -> Weight {
        // Includes a sort over up to TWAP_WINDOW samples — bounded const work.
        Weight::from_parts(80_000_000, 8192)
            .saturating_add(T::DbWeight::get().reads(2))
            .saturating_add(T::DbWeight::get().writes(2))
    }
}

impl WeightInfo for () {
    fn submit_price() -> Weight {
        Weight::from_parts(80_000_000, 8192)
    }
}
