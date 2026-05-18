//! # Yuma Consensus Pallet
//!
//! Stake-weighted validator-set scoring of operator quality. Per epoch a
//! validator submits a weight vector (operator → u16 quality score, scaled
//! 0..=65535). At epoch boundary, `compute_epoch_incentives` aggregates
//! weights, applies a Yuma median, and emits per-operator incentive shares
//! consumed by `pallet-bme`.
//!
//! This skeleton only persists the raw submissions; consensus math is TBD.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(deprecated)]

pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod weights;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
    use crate::weights::WeightInfo;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    /// Maximum number of validators considered per epoch in
    /// `compute_epoch_incentives`. Bounds the storage iteration.
    pub const MAX_VALIDATORS_PER_EPOCH: u32 = 1024;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>>
            + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Weight info.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Per-validator weight vector for an epoch.
    #[pallet::storage]
    pub type Weights<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        u64,            // epoch
        Blake2_128Concat,
        T::AccountId,   // validator
        BoundedVec<(T::AccountId, u16), ConstU32<4096>>,
    >;

    /// Computed per-operator incentive share for an epoch (u32 fixed-point bps).
    #[pallet::storage]
    pub type EpochIncentives<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        u64,
        Blake2_128Concat,
        T::AccountId,
        u32,
        ValueQuery,
    >;

    /// Records which epochs have already been computed — guard against
    /// double-counting.
    #[pallet::storage]
    pub type Computed<T: Config> = StorageMap<_, Blake2_128Concat, u64, bool, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        WeightsSubmitted { validator: T::AccountId, epoch: u64, vector_len: u32 },
        EpochComputed { epoch: u64, operator_count: u32 },
    }

    #[pallet::error]
    pub enum Error<T> {
        WeightVectorTooLarge,
        EpochAlreadyComputed,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::submit_weights())]
        pub fn submit_weights(
            origin: OriginFor<T>,
            epoch: u64,
            vector: sp_std::vec::Vec<(T::AccountId, u16)>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let bounded: BoundedVec<_, ConstU32<4096>> = vector
                .try_into()
                .map_err(|_| Error::<T>::WeightVectorTooLarge)?;
            let len = bounded.len() as u32;
            Weights::<T>::insert(epoch, &who, bounded);
            Self::deposit_event(Event::WeightsSubmitted {
                validator: who,
                epoch,
                vector_len: len,
            });
            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::compute_epoch_incentives())]
        pub fn compute_epoch_incentives(origin: OriginFor<T>, epoch: u64) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            ensure!(!Computed::<T>::get(epoch), Error::<T>::EpochAlreadyComputed);
            // Mark first so any nested mutation/re-entry path is safe.
            Computed::<T>::insert(epoch, true);
            // Skeleton: real Yuma median aggregation deferred. Just emit event.
            let mut count: u32 = 0;
            let mut validators_seen: u32 = 0;
            for (_validator, vector) in Weights::<T>::iter_prefix(epoch) {
                if validators_seen >= MAX_VALIDATORS_PER_EPOCH {
                    break;
                }
                validators_seen = validators_seen.saturating_add(1);
                for (op, score) in vector.iter() {
                    EpochIncentives::<T>::mutate(epoch, op, |slot| {
                        *slot = slot.saturating_add(*score as u32);
                    });
                    count = count.saturating_add(1);
                }
            }
            Self::deposit_event(Event::EpochComputed { epoch, operator_count: count });
            Ok(())
        }
    }
}
