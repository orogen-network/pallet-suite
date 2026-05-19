//! # Treasury Extension Pallet
//!
//! Lightweight wrapper that records a foundation spend proposal, requires
//! multisig approval (threshold-of-set), then marks the proposal executed.
//! Actual fund transfer is layered on top in a production runtime — this
//! pallet is the chain-level proposal log.

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
    use frame_support::traits::Contains;
    use frame_system::pallet_prelude::*;

    pub type Amount = u128;

    /// Maximum council size — bounds the per-proposal approval set.
    pub const MAX_COUNCIL: u32 = 32;

    #[derive(Clone, Copy, Encode, Decode, MaxEncodedLen, TypeInfo, PartialEq, Eq, Debug)]
    pub enum ProposalState {
        Pending,
        Executed,
        Rejected,
    }

    #[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, PartialEq, Eq, Debug)]
    #[scale_info(skip_type_params(T))]
    pub struct Proposal<T: Config> {
        pub proposer: T::AccountId,
        pub beneficiary: T::AccountId,
        pub amount: Amount,
        pub approvals: BoundedVec<T::AccountId, ConstU32<MAX_COUNCIL>>,
        pub state: ProposalState,
        pub created_at: BlockNumberFor<T>,
    }

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// The set of accounts that may propose / approve a spend (the
        /// foundation council). Membership is decided by the runtime; the
        /// pallet only checks `contains`.
        type CouncilMembers: Contains<Self::AccountId>;
        /// Weight info.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    pub type NextProposalId<T: Config> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    pub type Proposals<T: Config> = StorageMap<_, Blake2_128Concat, u64, Proposal<T>>;

    /// Approval threshold (e.g. 5-of-7). Default `0` is rejected at runtime —
    /// callers must set this via governance.
    #[pallet::storage]
    pub type Threshold<T: Config> = StorageValue<_, u32, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        Proposed {
            proposal_id: u64,
            proposer: T::AccountId,
            beneficiary: T::AccountId,
            amount: Amount,
        },
        Approved {
            proposal_id: u64,
            approver: T::AccountId,
            approvals: u32,
        },
        Executed {
            proposal_id: u64,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        UnknownProposal,
        BadState,
        BelowThreshold,
        NotCouncilMember,
        AlreadyApproved,
        TooManyApprovals,
        ThresholdNotSet,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::propose_spend())]
        pub fn propose_spend(
            origin: OriginFor<T>,
            beneficiary: T::AccountId,
            amount: Amount,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(
                T::CouncilMembers::contains(&who),
                Error::<T>::NotCouncilMember
            );
            let proposal_id = NextProposalId::<T>::mutate(|n| {
                let id = *n;
                *n = n.saturating_add(1);
                id
            });
            let now = frame_system::Pallet::<T>::block_number();
            Proposals::<T>::insert(
                proposal_id,
                Proposal::<T> {
                    proposer: who.clone(),
                    beneficiary: beneficiary.clone(),
                    amount,
                    approvals: BoundedVec::default(),
                    state: ProposalState::Pending,
                    created_at: now,
                },
            );
            Self::deposit_event(Event::Proposed {
                proposal_id,
                proposer: who,
                beneficiary,
                amount,
            });
            Ok(())
        }

        /// Multisig-gated approval / execution.
        ///
        /// Each caller must be a current `CouncilMembers` member and may
        /// only approve a given proposal once. Once the unique approver set
        /// reaches `Threshold`, the proposal is marked `Executed`. The
        /// pallet does not move funds on its own — a separate spend
        /// extrinsic in the runtime layers atop this signal.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::execute_spend())]
        pub fn execute_spend(origin: OriginFor<T>, proposal_id: u64) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(
                T::CouncilMembers::contains(&who),
                Error::<T>::NotCouncilMember
            );
            let threshold = Threshold::<T>::get();
            ensure!(threshold > 0, Error::<T>::ThresholdNotSet);
            Proposals::<T>::try_mutate(proposal_id, |maybe| -> DispatchResult {
                let p = maybe.as_mut().ok_or(Error::<T>::UnknownProposal)?;
                ensure!(p.state == ProposalState::Pending, Error::<T>::BadState);
                ensure!(
                    !p.approvals.iter().any(|a| a == &who),
                    Error::<T>::AlreadyApproved
                );
                p.approvals
                    .try_push(who.clone())
                    .map_err(|_| Error::<T>::TooManyApprovals)?;
                let n = p.approvals.len() as u32;
                Self::deposit_event(Event::Approved {
                    proposal_id,
                    approver: who.clone(),
                    approvals: n,
                });
                if n >= threshold {
                    p.state = ProposalState::Executed;
                    Self::deposit_event(Event::Executed { proposal_id });
                }
                Ok(())
            })
        }
    }
}
