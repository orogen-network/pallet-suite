//! # Oracle TWAP Pallet
//!
//! Time-weighted average price (4-12 hour window) of CUC vs USD. Submitters
//! push spot price snapshots; the pallet computes a TWAP that `pallet-bme`
//! reads when checking burn/mint ratio. RFC-0008 shape (final spec TBD).

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

    /// Price in micro-USD per CUC (e.g. $1.000000 = 1_000_000).
    pub type Price = u64;

    /// Window size (number of recent submissions to average). 4-12h depending on cadence.
    pub const TWAP_WINDOW: u32 = 240;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>>
            + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Origin allowed to submit prices (oracle reporter set). Restricting
        /// this prevents any signed account from dominating the TWAP.
        type ReporterOrigin: EnsureOrigin<Self::RuntimeOrigin>;
        /// Weight info.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Ring buffer of recent submissions.
    #[pallet::storage]
    pub type Samples<T: Config> =
        StorageValue<_, BoundedVec<Price, ConstU32<TWAP_WINDOW>>, ValueQuery>;

    /// Cached TWAP (median-of-N) recomputed on each submission.
    #[pallet::storage]
    pub type CurrentTwap<T: Config> = StorageValue<_, Price, ValueQuery>;

    /// Last block in which a price was submitted — one submission per block.
    #[pallet::storage]
    pub type LastSubmissionBlock<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        PriceSubmitted { price: Price },
        TwapUpdated { twap: Price },
    }

    #[pallet::error]
    pub enum Error<T> {
        InvalidPrice,
        SubmissionRateLimited,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::submit_price())]
        pub fn submit_price(origin: OriginFor<T>, price: Price) -> DispatchResult {
            T::ReporterOrigin::ensure_origin(origin)?;
            ensure!(price > 0, Error::<T>::InvalidPrice);
            // One-submission-per-block cap.
            let now = frame_system::Pallet::<T>::block_number();
            let last = LastSubmissionBlock::<T>::get();
            ensure!(now > last, Error::<T>::SubmissionRateLimited);
            LastSubmissionBlock::<T>::put(now);
            Samples::<T>::mutate(|v| {
                if v.len() == TWAP_WINDOW as usize {
                    let _ = v.remove(0);
                }
                let _ = v.try_push(price);
            });
            let mut samples: sp_std::vec::Vec<Price> = Samples::<T>::get().into_inner();
            // Median-of-N rather than mean: a single outlier cannot move the
            // computed TWAP by more than one step in the sorted window.
            samples.sort_unstable();
            let twap = if samples.is_empty() {
                0
            } else {
                samples[samples.len() / 2]
            };
            CurrentTwap::<T>::put(twap);
            Self::deposit_event(Event::PriceSubmitted { price });
            Self::deposit_event(Event::TwapUpdated { twap });
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Query helper used by `pallet-bme`.
        pub fn current_twap() -> Price {
            CurrentTwap::<T>::get()
        }
    }
}
