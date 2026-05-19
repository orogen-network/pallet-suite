//! # Attestation Registry Pallet
//!
//! Stores on-chain attestation summaries per RFC-0002. Full report blobs live
//! off-chain (chain-indexer + IPFS); only `report_hash`, `gpu_uuid`,
//! `vendor_set` flags, `measured_vm_bundle`, and expiry are persisted. CRL
//! entries are maintained per kind.

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

    #[derive(
        Clone,
        Copy,
        Encode,
        Decode,
        DecodeWithMemTracking,
        MaxEncodedLen,
        TypeInfo,
        PartialEq,
        Eq,
        Debug,
    )]
    pub enum CrlKind {
        FirmwareHash,
        DeviceCert,
        ModelHash,
        VendorPkiChain,
    }

    #[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, PartialEq, Eq, Debug)]
    #[scale_info(skip_type_params(T))]
    pub struct OnChainAttestation<T: Config> {
        pub operator: T::AccountId,
        pub report_hash: H256,
        pub gpu_uuid: H256,
        pub vendor_set: u8, // bitflags: 0x1 NVIDIA, 0x2 IntelTDX, 0x4 AmdSEV, 0x8 RIM
        pub measured_vm_bundle: H256,
        pub expires_at: BlockNumberFor<T>,
        pub revoked: bool,
    }

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Origin permitted to revoke attestations and manage CRL entries
        /// (typically `EnsureRoot` or a registry-admin multisig).
        type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;
        /// Weight info.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Active attestations keyed by `report_hash`.
    #[pallet::storage]
    pub type Attestations<T: Config> = StorageMap<_, Blake2_128Concat, H256, OnChainAttestation<T>>;

    /// CRL: (kind, target) → block_number at which entry was added.
    #[pallet::storage]
    pub type Crl<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, CrlKind, Blake2_128Concat, H256, BlockNumberFor<T>>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        Submitted {
            operator: T::AccountId,
            report_hash: H256,
        },
        Revoked {
            report_hash: H256,
        },
        CrlAdded {
            kind: CrlKind,
            target: H256,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        AlreadySubmitted,
        UnknownReport,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::submit())]
        pub fn submit(
            origin: OriginFor<T>,
            report_hash: H256,
            gpu_uuid: H256,
            vendor_set: u8,
            measured_vm_bundle: H256,
            expires_at: BlockNumberFor<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(
                !Attestations::<T>::contains_key(report_hash),
                Error::<T>::AlreadySubmitted
            );
            Attestations::<T>::insert(
                report_hash,
                OnChainAttestation::<T> {
                    operator: who.clone(),
                    report_hash,
                    gpu_uuid,
                    vendor_set,
                    measured_vm_bundle,
                    expires_at,
                    revoked: false,
                },
            );
            Self::deposit_event(Event::Submitted {
                operator: who,
                report_hash,
            });
            Ok(())
        }

        /// Revoke a previously-submitted attestation. Admin-gated.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::revoke())]
        pub fn revoke(origin: OriginFor<T>, report_hash: H256) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;
            Attestations::<T>::try_mutate(report_hash, |maybe| -> DispatchResult {
                let a = maybe.as_mut().ok_or(Error::<T>::UnknownReport)?;
                a.revoked = true;
                Ok(())
            })?;
            Self::deposit_event(Event::Revoked { report_hash });
            Ok(())
        }

        /// Add an entry to the certificate revocation list. Admin-gated.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::add_to_crl())]
        pub fn add_to_crl(origin: OriginFor<T>, kind: CrlKind, target: H256) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;
            let now = frame_system::Pallet::<T>::block_number();
            Crl::<T>::insert(kind, target, now);
            Self::deposit_event(Event::CrlAdded { kind, target });
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Query helper used by the worker daemon and validator-replay.
        pub fn is_revoked(kind: CrlKind, target: H256) -> bool {
            Crl::<T>::contains_key(kind, target)
        }
    }
}
