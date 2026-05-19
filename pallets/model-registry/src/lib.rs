//! # Model Registry Pallet
//!
//! Content-addressed registry of base models and LoRA adapters used by the
//! mining network. Both base models and adapters are keyed by `H256` (the
//! BLAKE2-256 content hash of the canonical weight tensor manifest).
//!
//! Dispatchables (skeleton — logic deferred):
//! - `register_base_model`
//! - `register_adapter`
//! - `deprecate`

#![cfg_attr(not(feature = "std"), no_std)]
// Skeleton stage: tolerate FRAME deprecation for the explicit `RuntimeEvent` associated type.
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

    /// Maximum length in bytes of the manifest URL stored on chain.
    pub const MAX_URL_LEN: u32 = 256;
    /// Maximum length of human-readable model name.
    pub const MAX_NAME_LEN: u32 = 64;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Aggregated event type of the runtime.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Weight info.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// On-chain representation of a base model.
    #[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, PartialEq, Eq, Debug)]
    #[scale_info(skip_type_params(T))]
    pub struct BaseModel<T: Config> {
        pub owner: T::AccountId,
        pub manifest_hash: H256,
        pub registered_at: BlockNumberFor<T>,
        pub deprecated: bool,
    }

    /// On-chain representation of a LoRA adapter, bound to a base model.
    #[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, PartialEq, Eq, Debug)]
    #[scale_info(skip_type_params(T))]
    pub struct Adapter<T: Config> {
        pub owner: T::AccountId,
        pub base_model_id: H256,
        pub manifest_hash: H256,
        pub registered_at: BlockNumberFor<T>,
        pub deprecated: bool,
    }

    #[pallet::storage]
    pub type BaseModels<T: Config> = StorageMap<_, Blake2_128Concat, H256, BaseModel<T>>;

    #[pallet::storage]
    pub type Adapters<T: Config> = StorageMap<_, Blake2_128Concat, H256, Adapter<T>>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        BaseModelRegistered {
            id: H256,
            owner: T::AccountId,
        },
        AdapterRegistered {
            id: H256,
            base_model_id: H256,
            owner: T::AccountId,
        },
        Deprecated {
            id: H256,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        AlreadyRegistered,
        UnknownModel,
        UnknownAdapter,
        NotOwner,
        IdCollision,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register a new base model. The `id` is the content hash of the
        /// weight manifest. Off-chain metadata (name / manifest URL) is the
        /// caller's responsibility — the chain only stores hashes.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::register_base_model())]
        pub fn register_base_model(
            origin: OriginFor<T>,
            id: H256,
            manifest_hash: H256,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(
                !BaseModels::<T>::contains_key(id),
                Error::<T>::AlreadyRegistered
            );
            // Disallow id collision with an existing adapter.
            ensure!(!Adapters::<T>::contains_key(id), Error::<T>::IdCollision);
            let block = frame_system::Pallet::<T>::block_number();
            BaseModels::<T>::insert(
                id,
                BaseModel::<T> {
                    owner: who.clone(),
                    manifest_hash,
                    registered_at: block,
                    deprecated: false,
                },
            );
            Self::deposit_event(Event::BaseModelRegistered { id, owner: who });
            Ok(())
        }

        /// Register a LoRA adapter against an existing base model.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::register_adapter())]
        pub fn register_adapter(
            origin: OriginFor<T>,
            id: H256,
            base_model_id: H256,
            manifest_hash: H256,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(
                BaseModels::<T>::contains_key(base_model_id),
                Error::<T>::UnknownModel
            );
            ensure!(
                !Adapters::<T>::contains_key(id),
                Error::<T>::AlreadyRegistered
            );
            // Disallow id collision with an existing base model.
            ensure!(!BaseModels::<T>::contains_key(id), Error::<T>::IdCollision);
            let block = frame_system::Pallet::<T>::block_number();
            Adapters::<T>::insert(
                id,
                Adapter::<T> {
                    owner: who.clone(),
                    base_model_id,
                    manifest_hash,
                    registered_at: block,
                    deprecated: false,
                },
            );
            Self::deposit_event(Event::AdapterRegistered {
                id,
                base_model_id,
                owner: who,
            });
            Ok(())
        }

        /// Mark a base model or adapter as deprecated. Only owner.
        ///
        /// Checks both `BaseModels` and `Adapters` maps so a deprecation
        /// applies to whichever artifact (if any) the caller owns at this
        /// id. Errors with `UnknownModel` if neither exists.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::deprecate())]
        pub fn deprecate(origin: OriginFor<T>, id: H256) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let mut touched = false;
            if let Some(mut bm) = BaseModels::<T>::get(id) {
                ensure!(bm.owner == who, Error::<T>::NotOwner);
                bm.deprecated = true;
                BaseModels::<T>::insert(id, bm);
                touched = true;
            }
            if let Some(mut ad) = Adapters::<T>::get(id) {
                ensure!(ad.owner == who, Error::<T>::NotOwner);
                ad.deprecated = true;
                Adapters::<T>::insert(id, ad);
                touched = true;
            }
            ensure!(touched, Error::<T>::UnknownModel);
            Self::deposit_event(Event::Deprecated { id });
            Ok(())
        }
    }
}
