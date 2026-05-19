//! Hand-tuned weight defaults for `pallet-yuma-consensus`.

#![allow(clippy::unnecessary_cast)]

use frame_support::traits::Get;
use frame_support::weights::Weight;
use frame_system::Config;
use sp_std::marker::PhantomData;

pub trait WeightInfo {
    fn submit_weights() -> Weight;
    fn compute_epoch_incentives(max_validators: u32, max_vector_len: u32) -> Weight;
    fn add_validator(max_validators: u32) -> Weight;
    fn remove_validator() -> Weight;
    fn update_validator_stake(max_validators: u32) -> Weight;
    fn rotate_permits(max_validators: u32) -> Weight;
}

pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: Config> WeightInfo for SubstrateWeight<T> {
    fn submit_weights() -> Weight {
        Weight::from_parts(80_000_000, 16_384)
            .saturating_add(T::DbWeight::get().reads(3))
            .saturating_add(T::DbWeight::get().writes(1))
    }
    fn compute_epoch_incentives(max_validators: u32, max_vector_len: u32) -> Weight {
        let inner_ops = u64::from(max_validators).saturating_mul(u64::from(max_vector_len));
        // Work performed:
        // - read Computed, write Computed
        // - iterate at most max_validators submissions
        // - sort submitted scores once to derive per-operator medians
        // - mutate EpochScoreTotals once per score (read + write)
        // - second pass over at most inner_ops operator totals and write incentives
        Weight::from_parts(
            500_000_000u64.saturating_add(inner_ops.saturating_mul(200_000)),
            65_536,
        )
        .saturating_add(
            T::DbWeight::get().reads(1 + u64::from(max_validators) + inner_ops + inner_ops),
        )
        .saturating_add(T::DbWeight::get().writes(1 + inner_ops + inner_ops))
    }
    fn add_validator(max_validators: u32) -> Weight {
        // Reads include the bounded entity cap scan after bootstrap.
        Weight::from_parts(
            50_000_000u64.saturating_add(u64::from(max_validators).saturating_mul(25_000)),
            4096,
        )
        .saturating_add(T::DbWeight::get().reads(12 + u64::from(max_validators)))
        .saturating_add(T::DbWeight::get().writes(6))
    }
    fn remove_validator() -> Weight {
        Weight::from_parts(50_000_000, 4096)
            .saturating_add(T::DbWeight::get().reads(8))
            .saturating_add(T::DbWeight::get().writes(7))
    }
    fn update_validator_stake(max_validators: u32) -> Weight {
        // Reads include the bounded entity cap scan after bootstrap.
        Weight::from_parts(
            60_000_000u64.saturating_add(u64::from(max_validators).saturating_mul(25_000)),
            4096,
        )
        .saturating_add(T::DbWeight::get().reads(16 + u64::from(max_validators)))
        .saturating_add(T::DbWeight::get().writes(8))
    }
    fn rotate_permits(max_validators: u32) -> Weight {
        Weight::from_parts(
            80_000_000u64.saturating_add(u64::from(max_validators).saturating_mul(75_000)),
            8192,
        )
        .saturating_add(
            T::DbWeight::get().reads(
                u64::from(max_validators)
                    .saturating_mul(3)
                    .saturating_add(2),
            ),
        )
        .saturating_add(
            T::DbWeight::get().writes(
                u64::from(max_validators)
                    .saturating_mul(4)
                    .saturating_add(2),
            ),
        )
    }
}

impl WeightInfo for () {
    fn submit_weights() -> Weight {
        Weight::from_parts(80_000_000, 16_384)
    }
    fn compute_epoch_incentives(max_validators: u32, max_vector_len: u32) -> Weight {
        let inner_ops = u64::from(max_validators).saturating_mul(u64::from(max_vector_len));
        Weight::from_parts(
            500_000_000u64.saturating_add(inner_ops.saturating_mul(200_000)),
            65_536,
        )
    }
    fn add_validator(max_validators: u32) -> Weight {
        Weight::from_parts(
            50_000_000u64.saturating_add(u64::from(max_validators).saturating_mul(25_000)),
            4096,
        )
    }
    fn remove_validator() -> Weight {
        Weight::from_parts(50_000_000, 4096)
    }
    fn update_validator_stake(max_validators: u32) -> Weight {
        Weight::from_parts(
            60_000_000u64.saturating_add(u64::from(max_validators).saturating_mul(25_000)),
            4096,
        )
    }
    fn rotate_permits(max_validators: u32) -> Weight {
        Weight::from_parts(
            80_000_000u64.saturating_add(u64::from(max_validators).saturating_mul(75_000)),
            8192,
        )
    }
}
