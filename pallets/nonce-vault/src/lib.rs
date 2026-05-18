//! # Nonce Vault Pallet
//!
//! Customer-nonce anti-replay for inference requests. Each request carries an
//! `H256` nonce from the customer SDK; the gateway records it on-chain before
//! routing. Replays inside the 24h window are rejected. RFC-0007.

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
    use sp_runtime::traits::{SaturatedConversion, Saturating};

    /// Replay window in blocks. Assumes 6s block-time; 24h = 14_400 blocks.
    pub const REPLAY_WINDOW_BLOCKS: u32 = 14_400;

    /// How many expired nonces to evict per block during the on-initialize
    /// pruning sweep. Bounds the work done per block.
    pub const PRUNE_BATCH_PER_BLOCK: u32 = 64;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>>
            + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Origin permitted to record nonces (the gateway pallet's verified
        /// origin or `EnsureRoot` in dev).
        type GatewayOrigin: EnsureOrigin<Self::RuntimeOrigin>;
        /// Weight info.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// `nonce -> block_recorded_at`. Pruned in `on_initialize`.
    #[pallet::storage]
    pub type Nonces<T: Config> = StorageMap<_, Blake2_128Concat, H256, BlockNumberFor<T>>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        NonceRecorded { nonce: H256 },
        NoncesPruned { count: u32 },
    }

    #[pallet::error]
    pub enum Error<T> {
        Replay,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        /// Prune up to `PRUNE_BATCH_PER_BLOCK` expired nonces per block.
        fn on_initialize(now: BlockNumberFor<T>) -> Weight {
            let window: BlockNumberFor<T> = REPLAY_WINDOW_BLOCKS.saturated_into();
            let mut pruned = 0u32;
            let mut to_remove: sp_std::vec::Vec<H256> = sp_std::vec::Vec::new();
            for (k, recorded_at) in Nonces::<T>::iter() {
                if pruned >= PRUNE_BATCH_PER_BLOCK {
                    break;
                }
                if Saturating::saturating_sub(now, recorded_at) >= window {
                    to_remove.push(k);
                    pruned += 1;
                }
            }
            for k in to_remove {
                Nonces::<T>::remove(k);
            }
            if pruned > 0 {
                Self::deposit_event(Event::NoncesPruned { count: pruned });
            }
            T::DbWeight::get().reads_writes(
                (pruned as u64).saturating_add(1),
                pruned as u64,
            )
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Record a customer-nonce. Gated on `GatewayOrigin` so arbitrary
        /// signed accounts cannot pre-burn customer nonces.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::record_nonce())]
        pub fn record_nonce(origin: OriginFor<T>, nonce: H256) -> DispatchResult {
            T::GatewayOrigin::ensure_origin(origin)?;
            let now = frame_system::Pallet::<T>::block_number();
            if let Some(recorded_at) = Nonces::<T>::get(nonce) {
                let window: BlockNumberFor<T> = REPLAY_WINDOW_BLOCKS.saturated_into();
                if Saturating::saturating_sub(now, recorded_at) < window {
                    return Err(Error::<T>::Replay.into());
                }
            }
            Nonces::<T>::insert(nonce, now);
            Self::deposit_event(Event::NonceRecorded { nonce });
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Query helper used by gateway-router before routing.
        pub fn check_nonce(nonce: H256) -> bool {
            let now = frame_system::Pallet::<T>::block_number();
            let window: BlockNumberFor<T> = REPLAY_WINDOW_BLOCKS.saturated_into();
            match Nonces::<T>::get(nonce) {
                Some(at) => Saturating::saturating_sub(now, at) >= window,
                None => true,
            }
        }
    }
}
