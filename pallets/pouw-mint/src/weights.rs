//! Hand-tuned weight defaults for `pallet-pouw-mint`.

#![allow(clippy::unnecessary_cast)]

use frame_support::traits::Get;
use frame_support::weights::Weight;
use frame_system::Config;
use sp_std::marker::PhantomData;

pub trait WeightInfo {
    fn submit_cupow_transcript() -> Weight;
    fn emit_pouw_reward() -> Weight;
}

pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: Config> WeightInfo for SubstrateWeight<T> {
    fn submit_cupow_transcript() -> Weight {
        Weight::from_parts(40_000_000, 4096)
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(1))
    }
    fn emit_pouw_reward() -> Weight {
        Weight::from_parts(20_000_000, 1024).saturating_add(T::DbWeight::get().reads(1))
    }
}

impl WeightInfo for () {
    fn submit_cupow_transcript() -> Weight {
        Weight::from_parts(40_000_000, 4096)
    }
    fn emit_pouw_reward() -> Weight {
        Weight::from_parts(20_000_000, 1024)
    }
}
