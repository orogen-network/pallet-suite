//! # Job Market Pallet
//!
//! Soft state machine for inference jobs: `Submitted → Assigned → Finalized`
//! with a `Disputed` branch. Per RFC-0004, most accounting happens via batch
//! settlement; this pallet is the per-job state anchor used for disputes.

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
        type RuntimeEvent: From<Event<Self>>
            + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Origin permitted to drive `assign`/`finalize` transitions
        /// (typically the gateway pallet's verified origin or `EnsureRoot`).
        type GatewayOrigin: EnsureOrigin<Self::RuntimeOrigin>;
        /// Weight info.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[derive(Clone, Copy, Encode, Decode, MaxEncodedLen, TypeInfo, PartialEq, Eq, Debug)]
    pub enum JobState {
        Submitted,
        Assigned,
        Finalized,
        Disputed,
    }

    #[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, PartialEq, Eq, Debug)]
    #[scale_info(skip_type_params(T))]
    pub struct Job<T: Config> {
        pub customer: T::AccountId,
        pub gateway: T::AccountId,
        pub model_id: H256,
        pub adapter_id: Option<H256>,
        pub state: JobState,
        pub assigned_operator: Option<T::AccountId>,
        pub created_at: BlockNumberFor<T>,
    }

    #[pallet::storage]
    pub type Jobs<T: Config> = StorageMap<_, Blake2_128Concat, H256, Job<T>>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        JobSubmitted { job_id: H256, customer: T::AccountId },
        JobAssigned { job_id: H256, operator: T::AccountId },
        JobFinalized { job_id: H256 },
        JobDisputed { job_id: H256 },
    }

    #[pallet::error]
    pub enum Error<T> {
        AlreadyExists,
        UnknownJob,
        BadState,
        NotAuthorized,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::submit_job())]
        pub fn submit_job(
            origin: OriginFor<T>,
            job_id: H256,
            gateway: T::AccountId,
            model_id: H256,
            adapter_id: Option<H256>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(!Jobs::<T>::contains_key(job_id), Error::<T>::AlreadyExists);
            let block = frame_system::Pallet::<T>::block_number();
            Jobs::<T>::insert(
                job_id,
                Job::<T> {
                    customer: who.clone(),
                    gateway,
                    model_id,
                    adapter_id,
                    state: JobState::Submitted,
                    assigned_operator: None,
                    created_at: block,
                },
            );
            Self::deposit_event(Event::JobSubmitted { job_id, customer: who });
            Ok(())
        }

        /// Assign a job to an operator. Gated on `GatewayOrigin`.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::assign())]
        pub fn assign(
            origin: OriginFor<T>,
            job_id: H256,
            operator: T::AccountId,
        ) -> DispatchResult {
            T::GatewayOrigin::ensure_origin(origin)?;
            Jobs::<T>::try_mutate(job_id, |maybe| -> DispatchResult {
                let job = maybe.as_mut().ok_or(Error::<T>::UnknownJob)?;
                ensure!(job.state == JobState::Submitted, Error::<T>::BadState);
                job.state = JobState::Assigned;
                job.assigned_operator = Some(operator.clone());
                Ok(())
            })?;
            Self::deposit_event(Event::JobAssigned { job_id, operator });
            Ok(())
        }

        /// Finalize a job. Gated on `GatewayOrigin`.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::finalize())]
        pub fn finalize(origin: OriginFor<T>, job_id: H256) -> DispatchResult {
            T::GatewayOrigin::ensure_origin(origin)?;
            Jobs::<T>::try_mutate(job_id, |maybe| -> DispatchResult {
                let job = maybe.as_mut().ok_or(Error::<T>::UnknownJob)?;
                ensure!(job.state == JobState::Assigned, Error::<T>::BadState);
                job.state = JobState::Finalized;
                Ok(())
            })?;
            Self::deposit_event(Event::JobFinalized { job_id });
            Ok(())
        }

        /// Dispute a job. Only the customer or gateway recorded on the job
        /// may dispute, and only while the job is `Submitted` or `Assigned`
        /// — once `Finalized` or already `Disputed`, no further transition
        /// is permitted.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::dispute())]
        pub fn dispute(origin: OriginFor<T>, job_id: H256) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Jobs::<T>::try_mutate(job_id, |maybe| -> DispatchResult {
                let job = maybe.as_mut().ok_or(Error::<T>::UnknownJob)?;
                ensure!(
                    matches!(job.state, JobState::Submitted | JobState::Assigned),
                    Error::<T>::BadState
                );
                ensure!(
                    who == job.customer || who == job.gateway,
                    Error::<T>::NotAuthorized
                );
                job.state = JobState::Disputed;
                Ok(())
            })?;
            Self::deposit_event(Event::JobDisputed { job_id });
            Ok(())
        }
    }
}
