//! Auto-generated-style weight scaffolding for `pallet-operator-stake`.
//!
//! These are not benchmark-derived; they are hand-tuned conservative defaults
//! that price each storage read/write against `T::DbWeight`. Replace with
//! generated weights once `benchmarking.rs` is wired up.

#![allow(clippy::unnecessary_cast)]

use frame_support::traits::Get;
use frame_support::weights::Weight;
use frame_system::Config;
use sp_std::marker::PhantomData;

pub trait WeightInfo {
    fn register() -> Weight;
    fn unregister() -> Weight;
    fn heartbeat() -> Weight;
    fn slash() -> Weight;
}

/// Concrete weight implementation pricing in `T::DbWeight`.
pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: Config> WeightInfo for SubstrateWeight<T> {
    fn register() -> Weight {
        // 2 reads (Operators, TotalStake), 2 writes, plus reserve currency op.
        Weight::from_parts(40_000_000, 4096)
            .saturating_add(T::DbWeight::get().reads(3))
            .saturating_add(T::DbWeight::get().writes(3))
    }
    fn unregister() -> Weight {
        Weight::from_parts(30_000_000, 2048)
            .saturating_add(T::DbWeight::get().reads(2))
            .saturating_add(T::DbWeight::get().writes(2))
    }
    fn heartbeat() -> Weight {
        Weight::from_parts(20_000_000, 1024)
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(1))
    }
    fn slash() -> Weight {
        Weight::from_parts(40_000_000, 4096)
            .saturating_add(T::DbWeight::get().reads(3))
            .saturating_add(T::DbWeight::get().writes(3))
    }
}

impl WeightInfo for () {
    fn register() -> Weight {
        Weight::from_parts(40_000_000, 4096)
    }
    fn unregister() -> Weight {
        Weight::from_parts(30_000_000, 2048)
    }
    fn heartbeat() -> Weight {
        Weight::from_parts(20_000_000, 1024)
    }
    fn slash() -> Weight {
        Weight::from_parts(40_000_000, 4096)
    }
}
