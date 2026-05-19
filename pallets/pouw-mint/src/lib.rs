//! # PoUW Mint Pallet
//!
//! cuPOW pool emission (5% lane, Hopper-only, deferred to Q4 2028). Skeleton
//! exists at TGE so call_indexes are reserved and the storage layout doesn't
//! shift on activation.

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
    use sp_core::H256;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Origin permitted to emit PoUW reward events (until Q4 2028,
        /// typically `EnsureRoot`).
        type PoUWRewardOrigin: EnsureOrigin<Self::RuntimeOrigin>;
        /// Weight info.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, PartialEq, Eq, Debug)]
    #[scale_info(skip_type_params(T))]
    pub struct CupowTranscript<T: Config> {
        pub operator: T::AccountId,
        pub transcript_hash: H256,
        pub submitted_at: BlockNumberFor<T>,
    }

    #[pallet::storage]
    pub type Transcripts<T: Config> = StorageMap<_, Blake2_128Concat, H256, CupowTranscript<T>>;

    #[pallet::storage]
    pub type Enabled<T: Config> = StorageValue<_, bool, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        TranscriptSubmitted {
            operator: T::AccountId,
            hash: H256,
        },
        PouwRewardEmitted {
            operator: T::AccountId,
            amount: u128,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        Disabled,
        DuplicateTranscript,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::submit_cupow_transcript())]
        pub fn submit_cupow_transcript(
            origin: OriginFor<T>,
            transcript_hash: H256,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(
                !Transcripts::<T>::contains_key(transcript_hash),
                Error::<T>::DuplicateTranscript
            );
            let now = frame_system::Pallet::<T>::block_number();
            Transcripts::<T>::insert(
                transcript_hash,
                CupowTranscript::<T> {
                    operator: who.clone(),
                    transcript_hash,
                    submitted_at: now,
                },
            );
            Self::deposit_event(Event::TranscriptSubmitted {
                operator: who,
                hash: transcript_hash,
            });
            Ok(())
        }

        /// Emit a PoUW reward event. Gated on `PoUWRewardOrigin` (Root until
        /// the cuPOW lane activates).
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::emit_pouw_reward())]
        pub fn emit_pouw_reward(
            origin: OriginFor<T>,
            operator: T::AccountId,
            amount: u128,
        ) -> DispatchResult {
            T::PoUWRewardOrigin::ensure_origin(origin)?;
            ensure!(Enabled::<T>::get(), Error::<T>::Disabled);
            Self::deposit_event(Event::PouwRewardEmitted { operator, amount });
            Ok(())
        }
    }
}
