//! # Yuma Consensus Pallet
//!
//! Stake-weighted validator-set scoring of operator quality. Per epoch a
//! validator submits a weight vector (operator → u16 quality score, scaled
//! 0..=65535). At epoch boundary, `compute_epoch_incentives` aggregates
//! weights, applies a Yuma median, and emits per-operator incentive shares
//! consumed by `pallet-bme`.
//!
//! Validator membership is governed on-chain by `GovernanceOrigin`; only
//! registered validators may submit weights. Admission records include a
//! governance-approved stake weight and entity id, so incentive computation can
//! use stake-weighted scoring while enforcing an entity concentration cap.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(deprecated)]

pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod weights;
pub use weights::WeightInfo;

const STORAGE_VERSION: frame_support::traits::StorageVersion =
    frame_support::traits::StorageVersion::new(3);

#[frame_support::pallet]
pub mod pallet {
    use crate::weights::WeightInfo;
    use frame_support::pallet_prelude::*;
    use frame_support::traits::{Contains, PalletInfoAccess, StorageVersion};
    use frame_support::weights::Weight;
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::Zero;

    pub const INCENTIVE_BPS: u128 = 10_000;

    #[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, PartialEq, Eq, Debug)]
    pub struct ValidatorInfo {
        pub stake_weight: u128,
        pub entity_id: u32,
    }

    #[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, PartialEq, Eq, Debug)]
    #[scale_info(skip_type_params(T))]
    pub struct WeightSubmission<T: Config> {
        pub stake_weight: u128,
        pub vector: BoundedVec<(T::AccountId, u16), T::MaxWeightVectorLen>,
    }

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Origin authorized to mutate the validator membership set.
        type GovernanceOrigin: EnsureOrigin<Self::RuntimeOrigin>;
        /// Maximum governed validators in the active set.
        #[pallet::constant]
        type MaxValidators: Get<u32>;
        /// Maximum validators with an active Yuma submission permit.
        #[pallet::constant]
        type MaxPermittedValidators: Get<u32>;
        /// Maximum operator scores in a single submitted validator vector.
        #[pallet::constant]
        type MaxWeightVectorLen: Get<u32>;
        /// Maximum entity share of total validator stake weight, in basis points.
        #[pallet::constant]
        type MaxEntityStakeBps: Get<u16>;
        /// Origin authorized to compute and finalize epoch incentives.
        type ComputeOrigin: EnsureOrigin<Self::RuntimeOrigin>;
        /// Weight info.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::storage_version(crate::STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_runtime_upgrade() -> Weight {
            let on_chain = Pallet::<T>::on_chain_storage_version();
            if on_chain >= StorageVersion::new(3) {
                return Weight::zero();
            }
            let pallet = Self::name().as_bytes();
            let mut removed = 0u64;
            if on_chain < StorageVersion::new(2) {
                for item in [
                    b"Weights".as_slice(),
                    b"Validators".as_slice(),
                    b"ValidatorCount".as_slice(),
                    b"TotalValidatorStake".as_slice(),
                    b"EntityStake".as_slice(),
                    b"EntityCount".as_slice(),
                    b"EpochScoreTotals".as_slice(),
                    b"Computed".as_slice(),
                    b"EpochIncentives".as_slice(),
                ] {
                    let result = frame_support::storage::migration::clear_storage_prefix(
                        pallet, item, b"", None, None,
                    );
                    removed = removed.saturating_add(result.backend as u64);
                }
            }
            for item in [b"PermittedValidators".as_slice(), b"PermitCount".as_slice()] {
                let result = frame_support::storage::migration::clear_storage_prefix(
                    pallet, item, b"", None, None,
                );
                removed = removed.saturating_add(result.backend as u64);
            }
            for item in [
                b"EpochPermittedValidators".as_slice(),
                b"EpochPermitCount".as_slice(),
            ] {
                let result = frame_support::storage::migration::clear_storage_prefix(
                    pallet, item, b"", None, None,
                );
                removed = removed.saturating_add(result.backend as u64);
            }
            StorageVersion::new(3).put::<Pallet<T>>();
            T::DbWeight::get().reads_writes(1, removed.saturating_add(1))
        }
    }

    /// Per-validator weight vector for an epoch.
    #[pallet::storage]
    pub type Weights<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        u64, // epoch
        Blake2_128Concat,
        T::AccountId, // validator
        WeightSubmission<T>,
    >;

    /// Computed per-operator incentive share for an epoch (u32 fixed-point bps).
    #[pallet::storage]
    pub type EpochIncentives<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, u64, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

    /// Records which epochs have already been computed — guard against
    /// double-counting.
    #[pallet::storage]
    pub type Computed<T: Config> = StorageMap<_, Blake2_128Concat, u64, bool, ValueQuery>;

    /// Governed validator membership set. `T::ValidatorSet` should usually be
    /// wired to `Pallet<T>` so this storage becomes the authorization source.
    #[pallet::storage]
    pub type Validators<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, ValidatorInfo, OptionQuery>;

    /// Active top-K validators allowed to submit Yuma weights.
    #[pallet::storage]
    pub type PermittedValidators<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, (), OptionQuery>;

    /// Count of active Yuma submission permits.
    #[pallet::storage]
    pub type PermitCount<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// Epoch-scoped Yuma submission permits.
    #[pallet::storage]
    pub type EpochPermittedValidators<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        u64,
        Blake2_128Concat,
        T::AccountId,
        ValidatorInfo,
        OptionQuery,
    >;

    /// Count of epoch-scoped Yuma submission permits.
    #[pallet::storage]
    pub type EpochPermitCount<T: Config> = StorageMap<_, Blake2_128Concat, u64, u32, ValueQuery>;

    /// Active validator count, maintained alongside `Validators` to enforce a
    /// bounded membership set without iterating storage.
    #[pallet::storage]
    pub type ValidatorCount<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// Total governed stake weight of the active validator set.
    #[pallet::storage]
    pub type TotalValidatorStake<T: Config> = StorageValue<_, u128, ValueQuery>;

    /// Per-entity validator stake weight used to enforce concentration caps.
    #[pallet::storage]
    pub type EntityStake<T: Config> = StorageMap<_, Blake2_128Concat, u32, u128, ValueQuery>;

    /// Number of active entities with non-zero validator stake.
    #[pallet::storage]
    pub type EntityCount<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// Temporary weighted score totals for computed epochs, retained as audit
    /// evidence for the incentive calculation.
    #[pallet::storage]
    pub type EpochScoreTotals<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        u64,
        Blake2_128Concat,
        T::AccountId,
        u128,
        ValueQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        ValidatorAdded {
            validator: T::AccountId,
            stake_weight: u128,
            entity_id: u32,
        },
        ValidatorRemoved {
            validator: T::AccountId,
        },
        WeightsSubmitted {
            validator: T::AccountId,
            epoch: u64,
            vector_len: u32,
        },
        EpochComputed {
            epoch: u64,
            operator_count: u32,
        },
        ValidatorStakeUpdated {
            validator: T::AccountId,
            stake_weight: u128,
            entity_id: u32,
        },
        PermitsRotated {
            permitted_count: u32,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        WeightVectorTooLarge,
        EpochAlreadyComputed,
        UnauthorizedValidator,
        ValidatorAlreadyExists,
        ValidatorNotFound,
        TooManyValidators,
        InvalidValidatorStake,
        EntityStakeCapExceeded,
        EpochAlreadyStarted,
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
            ensure!(
                EpochPermittedValidators::<T>::contains_key(epoch, &who),
                Error::<T>::UnauthorizedValidator
            );
            ensure!(!Computed::<T>::get(epoch), Error::<T>::EpochAlreadyComputed);
            let info = EpochPermittedValidators::<T>::get(epoch, &who)
                .ok_or(Error::<T>::UnauthorizedValidator)?;
            let bounded: BoundedVec<_, T::MaxWeightVectorLen> = vector
                .try_into()
                .map_err(|_| Error::<T>::WeightVectorTooLarge)?;
            let len = bounded.len() as u32;
            Weights::<T>::insert(
                epoch,
                &who,
                WeightSubmission::<T> {
                    stake_weight: info.stake_weight,
                    vector: bounded,
                },
            );
            Self::deposit_event(Event::WeightsSubmitted {
                validator: who,
                epoch,
                vector_len: len,
            });
            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::compute_epoch_incentives(
            T::MaxValidators::get(),
            T::MaxWeightVectorLen::get(),
        ))]
        pub fn compute_epoch_incentives(origin: OriginFor<T>, epoch: u64) -> DispatchResult {
            T::ComputeOrigin::ensure_origin(origin)?;
            ensure!(!Computed::<T>::get(epoch), Error::<T>::EpochAlreadyComputed);
            // Mark first so any nested mutation/re-entry path is safe.
            Computed::<T>::insert(epoch, true);
            let mut submissions: sp_std::vec::Vec<WeightSubmission<T>> = sp_std::vec::Vec::new();
            let mut total_weighted_score: u128 = 0;
            for (_validator, submission) in Weights::<T>::iter_prefix(epoch) {
                if submissions.len() >= T::MaxValidators::get() as usize {
                    break;
                }
                submissions.push(submission);
            }
            let medians = Self::operator_medians(&submissions);
            for submission in submissions.iter() {
                for (op, score) in submission.vector.iter() {
                    let median = Self::median_score(&medians, op);
                    let clipped_score = (*score).min(median);
                    let weighted = submission
                        .stake_weight
                        .saturating_mul(u128::from(clipped_score));
                    if weighted.is_zero() {
                        continue;
                    }
                    EpochScoreTotals::<T>::mutate(epoch, op, |slot| {
                        *slot = slot.saturating_add(weighted);
                    });
                    total_weighted_score = total_weighted_score.saturating_add(weighted);
                }
            }
            let mut operator_count: u32 = 0;
            if !total_weighted_score.is_zero() {
                for (op, weighted_score) in EpochScoreTotals::<T>::iter_prefix(epoch) {
                    let share = weighted_score
                        .saturating_mul(INCENTIVE_BPS)
                        .saturating_div(total_weighted_score);
                    EpochIncentives::<T>::insert(epoch, op, share.min(u128::from(u32::MAX)) as u32);
                    operator_count = operator_count.saturating_add(1);
                }
            }
            Self::deposit_event(Event::EpochComputed {
                epoch,
                operator_count,
            });
            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::add_validator(T::MaxValidators::get()))]
        pub fn add_validator(
            origin: OriginFor<T>,
            validator: T::AccountId,
            stake_weight: u128,
            entity_id: u32,
        ) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            ensure!(!stake_weight.is_zero(), Error::<T>::InvalidValidatorStake);
            ensure!(
                !Validators::<T>::contains_key(&validator),
                Error::<T>::ValidatorAlreadyExists
            );
            let next = ValidatorCount::<T>::get().saturating_add(1);
            ensure!(
                next <= T::MaxValidators::get(),
                Error::<T>::TooManyValidators
            );
            let new_entity = EntityStake::<T>::get(entity_id).is_zero();
            let active_entities = EntityCount::<T>::get().saturating_add(u32::from(new_entity));
            Self::ensure_entity_caps(entity_id, None, 0, stake_weight, active_entities)?;
            Validators::<T>::insert(
                &validator,
                ValidatorInfo {
                    stake_weight,
                    entity_id,
                },
            );
            ValidatorCount::<T>::put(next);
            TotalValidatorStake::<T>::mutate(|total| *total = total.saturating_add(stake_weight));
            Self::add_entity_stake(entity_id, stake_weight);
            Self::deposit_event(Event::ValidatorAdded {
                validator,
                stake_weight,
                entity_id,
            });
            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::remove_validator())]
        pub fn remove_validator(origin: OriginFor<T>, validator: T::AccountId) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            ensure!(
                Validators::<T>::contains_key(&validator),
                Error::<T>::ValidatorNotFound
            );
            let info = Validators::<T>::take(&validator).ok_or(Error::<T>::ValidatorNotFound)?;
            if PermittedValidators::<T>::take(&validator).is_some() {
                PermitCount::<T>::mutate(|count| *count = count.saturating_sub(1));
            }
            ValidatorCount::<T>::mutate(|count| {
                *count = count.saturating_sub(1);
            });
            TotalValidatorStake::<T>::mutate(|total| {
                *total = total.saturating_sub(info.stake_weight)
            });
            Self::sub_entity_stake(info.entity_id, info.stake_weight);
            Self::deposit_event(Event::ValidatorRemoved { validator });
            Ok(())
        }

        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::update_validator_stake(T::MaxValidators::get()))]
        pub fn update_validator_stake(
            origin: OriginFor<T>,
            validator: T::AccountId,
            stake_weight: u128,
            entity_id: u32,
        ) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            ensure!(!stake_weight.is_zero(), Error::<T>::InvalidValidatorStake);
            let old = Validators::<T>::get(&validator).ok_or(Error::<T>::ValidatorNotFound)?;
            let active_entities = Self::active_entities_after_update(
                old.entity_id,
                entity_id,
                old.stake_weight,
                stake_weight,
            );
            Self::ensure_entity_caps(
                entity_id,
                Some(old.entity_id),
                old.stake_weight,
                stake_weight,
                active_entities,
            )?;
            Validators::<T>::insert(
                &validator,
                ValidatorInfo {
                    stake_weight,
                    entity_id,
                },
            );
            TotalValidatorStake::<T>::mutate(|total| {
                *total = total
                    .saturating_sub(old.stake_weight)
                    .saturating_add(stake_weight);
            });
            Self::sub_entity_stake(old.entity_id, old.stake_weight);
            Self::add_entity_stake(entity_id, stake_weight);
            Self::deposit_event(Event::ValidatorStakeUpdated {
                validator,
                stake_weight,
                entity_id,
            });
            Ok(())
        }

        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::rotate_permits(T::MaxValidators::get()))]
        pub fn rotate_permits(origin: OriginFor<T>, epoch: u64) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            ensure!(!Computed::<T>::get(epoch), Error::<T>::EpochAlreadyComputed);
            ensure!(
                Weights::<T>::iter_prefix(epoch).next().is_none(),
                Error::<T>::EpochAlreadyStarted
            );
            let mut validators: sp_std::vec::Vec<(
                T::AccountId,
                ValidatorInfo,
                sp_std::vec::Vec<u8>,
            )> = Validators::<T>::iter()
                .map(|(account, info)| {
                    let encoded = account.encode();
                    (account, info, encoded)
                })
                .collect();
            validators.sort_by(|left, right| {
                right
                    .1
                    .stake_weight
                    .cmp(&left.1.stake_weight)
                    .then_with(|| left.2.cmp(&right.2))
            });

            let old_permits: sp_std::vec::Vec<T::AccountId> =
                PermittedValidators::<T>::iter_keys().collect();
            for account in old_permits {
                PermittedValidators::<T>::remove(account);
            }
            let old_epoch_permits: sp_std::vec::Vec<T::AccountId> =
                EpochPermittedValidators::<T>::iter_key_prefix(epoch).collect();
            for account in old_epoch_permits {
                EpochPermittedValidators::<T>::remove(epoch, account);
            }

            let limit = T::MaxPermittedValidators::get().min(T::MaxValidators::get()) as usize;
            let mut permitted_count: u32 = 0;
            for (account, info, _encoded) in validators.into_iter().take(limit) {
                PermittedValidators::<T>::insert(&account, ());
                EpochPermittedValidators::<T>::insert(epoch, account, info);
                permitted_count = permitted_count.saturating_add(1);
            }
            PermitCount::<T>::put(permitted_count);
            EpochPermitCount::<T>::insert(epoch, permitted_count);
            Self::deposit_event(Event::PermitsRotated { permitted_count });
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        fn operator_medians(
            submissions: &[WeightSubmission<T>],
        ) -> sp_std::vec::Vec<(sp_std::vec::Vec<u8>, u16)> {
            let mut scores: sp_std::vec::Vec<(sp_std::vec::Vec<u8>, u16)> = sp_std::vec::Vec::new();
            for submission in submissions {
                for (op, score) in submission.vector.iter() {
                    scores.push((op.encode(), *score));
                }
            }
            if scores.is_empty() {
                return sp_std::vec::Vec::new();
            }
            scores.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
            let mut medians: sp_std::vec::Vec<(sp_std::vec::Vec<u8>, u16)> =
                sp_std::vec::Vec::new();
            let mut group_start = 0usize;
            while group_start < scores.len() {
                let key = scores[group_start].0.clone();
                let mut group_end = group_start + 1;
                while group_end < scores.len() && scores[group_end].0 == key {
                    group_end += 1;
                }
                let median_index = group_start + (group_end - group_start - 1) / 2;
                medians.push((key, scores[median_index].1));
                group_start = group_end;
            }
            medians
        }

        fn median_score(medians: &[(sp_std::vec::Vec<u8>, u16)], operator: &T::AccountId) -> u16 {
            let key = operator.encode();
            match medians.binary_search_by(|(candidate, _score)| candidate.cmp(&key)) {
                Ok(index) => medians[index].1,
                Err(_) => 0,
            }
        }

        fn min_entities_for_cap() -> u32 {
            let cap = u32::from(T::MaxEntityStakeBps::get()).max(1);
            10_000u32.saturating_add(cap).saturating_sub(1) / cap
        }

        fn ensure_entity_caps(
            entity_id: u32,
            old_entity_id: Option<u32>,
            old_stake: u128,
            new_stake: u128,
            active_entities_after: u32,
        ) -> DispatchResult {
            if active_entities_after < Self::min_entities_for_cap() {
                return Ok(());
            }
            let total = TotalValidatorStake::<T>::get()
                .saturating_sub(old_stake)
                .saturating_add(new_stake);
            if total.is_zero() {
                return Ok(());
            }
            let cap_bps = u128::from(T::MaxEntityStakeBps::get());
            for (entity, mut stake) in EntityStake::<T>::iter() {
                if Some(entity) == old_entity_id {
                    stake = stake.saturating_sub(old_stake);
                }
                if entity == entity_id {
                    stake = stake.saturating_add(new_stake);
                }
                ensure!(
                    stake.saturating_mul(INCENTIVE_BPS) <= total.saturating_mul(cap_bps),
                    Error::<T>::EntityStakeCapExceeded
                );
            }
            if EntityStake::<T>::get(entity_id).is_zero() {
                ensure!(
                    new_stake.saturating_mul(INCENTIVE_BPS) <= total.saturating_mul(cap_bps),
                    Error::<T>::EntityStakeCapExceeded
                );
            }
            Ok(())
        }

        fn add_entity_stake(entity_id: u32, amount: u128) {
            let was_zero = EntityStake::<T>::get(entity_id).is_zero();
            EntityStake::<T>::mutate(entity_id, |stake| *stake = stake.saturating_add(amount));
            if was_zero && !amount.is_zero() {
                EntityCount::<T>::mutate(|count| *count = count.saturating_add(1));
            }
        }

        fn sub_entity_stake(entity_id: u32, amount: u128) {
            EntityStake::<T>::mutate(entity_id, |stake| *stake = stake.saturating_sub(amount));
            if EntityStake::<T>::get(entity_id).is_zero() {
                EntityStake::<T>::remove(entity_id);
                EntityCount::<T>::mutate(|count| *count = count.saturating_sub(1));
            }
        }

        fn active_entities_after_update(
            old_entity_id: u32,
            new_entity_id: u32,
            old_stake: u128,
            new_stake: u128,
        ) -> u32 {
            let mut count = EntityCount::<T>::get();
            if old_entity_id != new_entity_id {
                if EntityStake::<T>::get(new_entity_id).is_zero() && !new_stake.is_zero() {
                    count = count.saturating_add(1);
                }
                if EntityStake::<T>::get(old_entity_id) <= old_stake {
                    count = count.saturating_sub(1);
                }
            }
            count
        }
    }

    impl<T: Config> Contains<T::AccountId> for Pallet<T> {
        fn contains(account: &T::AccountId) -> bool {
            PermittedValidators::<T>::contains_key(account)
        }
    }
}
