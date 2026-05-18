//! Hand-tuned weight defaults for `pallet-attestation-registry`.

#![allow(clippy::unnecessary_cast)]

use frame_support::traits::Get;
use frame_support::weights::Weight;
use frame_system::Config;
use sp_std::marker::PhantomData;

pub trait WeightInfo {
    fn submit() -> Weight;
    fn revoke() -> Weight;
    fn add_to_crl() -> Weight;
}

pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: Config> WeightInfo for SubstrateWeight<T> {
    fn submit() -> Weight {
        Weight::from_parts(40_000_000, 4096)
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(1))
    }
    fn revoke() -> Weight {
        Weight::from_parts(30_000_000, 2048)
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(1))
    }
    fn add_to_crl() -> Weight {
        Weight::from_parts(30_000_000, 2048)
            .saturating_add(T::DbWeight::get().writes(1))
    }
}

impl WeightInfo for () {
    fn submit() -> Weight {
        Weight::from_parts(40_000_000, 4096)
    }
    fn revoke() -> Weight {
        Weight::from_parts(30_000_000, 2048)
    }
    fn add_to_crl() -> Weight {
        Weight::from_parts(30_000_000, 2048)
    }
}
