//! # BME (Burn-Mint Equilibrium) Pallet
//!
//! Implements the RFC-0004 batch settlement path: a gateway submits a burn
//! receipt referencing aggregated CUC consumption; the pallet mints OROG to
//! operators in proportion to per-operator summaries. Elasticity factor
//! controls how aggressively the mint scales with burn (subsidy lever).

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
    use sp_std::vec::Vec;

    /// Token amount (placeholder u128 until token pallet wired in).
    pub type TokenAmount = u128;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Origin permitted to submit verified burn receipts (verified gateways,
        /// typically a `EnsureSignedBy<Gateways>` set or `EnsureRoot`).
        type GatewayOrigin: EnsureOrigin<Self::RuntimeOrigin>;
        /// Origin permitted to mint OROG to operators (the job-market /
        /// settlement pallet, or `EnsureRoot`).
        type MintOrigin: EnsureOrigin<Self::RuntimeOrigin>;
        /// Origin permitted to update elasticity / governance parameters
        /// (typically `EnsureRoot`).
        type GovernanceOrigin: EnsureOrigin<Self::RuntimeOrigin>;
        /// Weight info.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Aggregate CUC burned across the network.
    #[pallet::storage]
    pub type CumulativeBurn<T: Config> = StorageValue<_, TokenAmount, ValueQuery>;

    /// Aggregate OROG minted to operators.
    #[pallet::storage]
    pub type CumulativeMint<T: Config> = StorageValue<_, TokenAmount, ValueQuery>;

    /// Elasticity factor in basis points (10_000 = 1.0). Subsidy lever.
    #[pallet::storage]
    pub type Elasticity<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// OROG balance per operator (placeholder until pallet-balances wires in).
    #[pallet::storage]
    pub type OperatorBalance<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, TokenAmount, ValueQuery>;

    /// Burn receipt batch ids already accepted by this runtime.
    #[pallet::storage]
    pub type ProcessedBurnBatches<T: Config> =
        StorageMap<_, Blake2_128Concat, H256, (), OptionQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        BurnSubmitted {
            amount: TokenAmount,
            batch_id: H256,
        },
        Minted {
            operator: T::AccountId,
            amount: TokenAmount,
        },
        ElasticitySet {
            elasticity_bps: u32,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        ZeroBurn,
        DuplicateBurnBatch,
        MintExceedsHeadroom,
        ArithmeticOverflow,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Gateway-submitted burn (RFC-0004 §submit_batch step 5).
        ///
        /// Gated on `GatewayOrigin` — only verified gateways may extend the
        /// network's recorded burn quantity, since this drives the mint
        /// headroom.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::submit_burn())]
        pub fn submit_burn(
            origin: OriginFor<T>,
            batch_id: H256,
            amount: TokenAmount,
        ) -> DispatchResult {
            T::GatewayOrigin::ensure_origin(origin)?;
            ensure!(amount > 0, Error::<T>::ZeroBurn);
            ensure!(
                !ProcessedBurnBatches::<T>::contains_key(batch_id),
                Error::<T>::DuplicateBurnBatch
            );
            ProcessedBurnBatches::<T>::insert(batch_id, ());
            CumulativeBurn::<T>::mutate(|v| *v = v.saturating_add(amount));
            Self::deposit_event(Event::BurnSubmitted { amount, batch_id });
            Ok(())
        }

        /// Mint OROG to a single operator. In production, called by
        /// `pallet-job-market::finalize_batch` after burn verification.
        ///
        /// Gated on `MintOrigin`. Uses `checked_mul` for the headroom cap and
        /// rejects on overflow instead of saturating to `u128::MAX`.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::mint_to_operator())]
        pub fn mint_to_operator(
            origin: OriginFor<T>,
            operator: T::AccountId,
            amount: TokenAmount,
        ) -> DispatchResult {
            T::MintOrigin::ensure_origin(origin)?;
            Self::checked_mint_headroom(amount)?;
            OperatorBalance::<T>::mutate(&operator, |b| *b = b.saturating_add(amount));
            CumulativeMint::<T>::mutate(|v| *v = v.saturating_add(amount));
            Self::deposit_event(Event::Minted { operator, amount });
            Ok(())
        }

        /// Set elasticity factor (governance hook). Root / governance only.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::set_elasticity())]
        pub fn set_elasticity(origin: OriginFor<T>, elasticity_bps: u32) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            Elasticity::<T>::put(elasticity_bps);
            Self::deposit_event(Event::ElasticitySet { elasticity_bps });
            Ok(())
        }
    }

    /// Helper: bulk mint per a SettlementBatch.per_operator_summary vector.
    /// Internal-only — not callable as a dispatchable. Reachable from in-runtime
    /// callers (e.g. `pallet-job-market::finalize_batch`) once wired up.
    impl<T: Config> Pallet<T> {
        fn checked_mint_headroom(additional: TokenAmount) -> DispatchResult {
            let burn = CumulativeBurn::<T>::get();
            let mint = CumulativeMint::<T>::get();
            let elasticity = Elasticity::<T>::get().max(1) as u128;
            let cap = burn
                .checked_mul(elasticity)
                .ok_or(Error::<T>::ArithmeticOverflow)?
                / 10_000;
            let new_total = mint
                .checked_add(additional)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            ensure!(new_total <= cap, Error::<T>::MintExceedsHeadroom);
            Ok(())
        }

        pub fn mint_batch(
            _caller: T::AccountId,
            summaries: Vec<(T::AccountId, TokenAmount)>,
        ) -> DispatchResult {
            let total = summaries.iter().try_fold(0u128, |acc, (_, amount)| {
                acc.checked_add(*amount)
                    .ok_or(Error::<T>::ArithmeticOverflow)
            })?;
            Self::checked_mint_headroom(total)?;
            for (operator, amount) in summaries {
                OperatorBalance::<T>::mutate(&operator, |b| *b = b.saturating_add(amount));
                CumulativeMint::<T>::mutate(|v| *v = v.saturating_add(amount));
                Self::deposit_event(Event::Minted { operator, amount });
            }
            Ok(())
        }
    }
}
