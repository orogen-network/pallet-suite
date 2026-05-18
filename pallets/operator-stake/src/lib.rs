//! # Operator Stake Pallet
//!
//! Tracks registered operators, their stake, last heartbeat epoch, and a
//! `slash` hook used by `pallet-slashing`. Mirrors RFC-0003 (heartbeat) and
//! RFC-0005 (slashing) on-chain fields.

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
    use frame_support::sp_runtime::traits::Saturating;
    use frame_support::traits::{Currency, ReservableCurrency};
    use frame_system::pallet_prelude::*;
    use sp_core::H256;

    /// Convenience alias for the currency balance type.
    pub type BalanceOf<T> = <<T as Config>::Currency as Currency<
        <T as frame_system::Config>::AccountId,
    >>::Balance;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>>
            + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Currency used to reserve operator stake.
        type Currency: ReservableCurrency<Self::AccountId>;
        /// Origin allowed to slash operators (typically `EnsureRoot` or the
        /// slashing pallet's verified origin).
        type SlashOrigin: EnsureOrigin<Self::RuntimeOrigin>;
        /// Weight info for benchmarked extrinsics.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Operator on-chain state.
    #[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, PartialEq, Eq, Debug)]
    #[scale_info(skip_type_params(T))]
    pub struct Operator<T: Config> {
        pub stake: BalanceOf<T>,
        pub last_heartbeat_epoch: u64,
        pub current_attestation_hash: H256,
        pub registered_at: BlockNumberFor<T>,
        pub frozen: bool,
    }

    #[pallet::storage]
    pub type Operators<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, Operator<T>>;

    #[pallet::storage]
    pub type TotalStake<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        Registered { who: T::AccountId, stake: BalanceOf<T> },
        Unregistered { who: T::AccountId },
        Heartbeat { who: T::AccountId, epoch: u64 },
        Slashed { who: T::AccountId, amount: BalanceOf<T>, reason_code: u16 },
    }

    #[pallet::error]
    pub enum Error<T> {
        AlreadyRegistered,
        NotRegistered,
        InsufficientStake,
        Frozen,
        ReserveFailed,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register an operator. RFC-0003: stake-bound hotkey identity.
        ///
        /// Reserves `stake` from the caller's free balance for the lifetime of
        /// the registration. The reservation is released on `unregister` and
        /// reduced by `slash`.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::register())]
        pub fn register(
            origin: OriginFor<T>,
            stake: BalanceOf<T>,
            attestation_hash: H256,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(!Operators::<T>::contains_key(&who), Error::<T>::AlreadyRegistered);
            ensure!(stake > BalanceOf::<T>::from(0u32), Error::<T>::InsufficientStake);
            T::Currency::reserve(&who, stake).map_err(|_| Error::<T>::ReserveFailed)?;
            let now = frame_system::Pallet::<T>::block_number();
            Operators::<T>::insert(
                &who,
                Operator::<T> {
                    stake,
                    last_heartbeat_epoch: 0,
                    current_attestation_hash: attestation_hash,
                    registered_at: now,
                    frozen: false,
                },
            );
            TotalStake::<T>::mutate(|t| *t = t.saturating_add(stake));
            Self::deposit_event(Event::Registered { who, stake });
            Ok(())
        }

        /// Voluntarily unregister. Real version exits via unbonding window.
        ///
        /// Releases the previously reserved stake back to the operator's free
        /// balance.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::unregister())]
        pub fn unregister(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let op = Operators::<T>::take(&who).ok_or(Error::<T>::NotRegistered)?;
            ensure!(!op.frozen, Error::<T>::Frozen);
            // Unreserve any remaining reserved stake.
            let _ = T::Currency::unreserve(&who, op.stake);
            TotalStake::<T>::mutate(|t| *t = t.saturating_sub(op.stake));
            Self::deposit_event(Event::Unregistered { who });
            Ok(())
        }

        /// Heartbeat: extend liveness for the current epoch. RFC-0003.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::heartbeat())]
        pub fn heartbeat(
            origin: OriginFor<T>,
            epoch_number: u64,
            _capabilities_summary_hash: H256,
            _attestation_report_hash: H256,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Operators::<T>::try_mutate(&who, |maybe| -> DispatchResult {
                let op = maybe.as_mut().ok_or(Error::<T>::NotRegistered)?;
                ensure!(!op.frozen, Error::<T>::Frozen);
                op.last_heartbeat_epoch = epoch_number;
                Ok(())
            })?;
            Self::deposit_event(Event::Heartbeat { who, epoch: epoch_number });
            Ok(())
        }

        /// Slash hook called by `pallet-slashing` after dispute resolution.
        ///
        /// Gated by `T::SlashOrigin` (typically `EnsureRoot` in dev / a
        /// dedicated slashing-panel origin in production). Burns the slashed
        /// amount from the operator's reserved balance.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::slash())]
        pub fn slash(
            origin: OriginFor<T>,
            who: T::AccountId,
            amount: BalanceOf<T>,
            reason_code: u16,
        ) -> DispatchResult {
            T::SlashOrigin::ensure_origin(origin)?;
            Operators::<T>::try_mutate(&who, |maybe| -> DispatchResult {
                let op = maybe.as_mut().ok_or(Error::<T>::NotRegistered)?;
                let take = amount.min(op.stake);
                let (_neg_imbalance, _remaining) =
                    T::Currency::slash_reserved(&who, take);
                op.stake = op.stake.saturating_sub(take);
                TotalStake::<T>::mutate(|t| *t = t.saturating_sub(take));
                Ok(())
            })?;
            Self::deposit_event(Event::Slashed { who, amount, reason_code });
            Ok(())
        }
    }
}
