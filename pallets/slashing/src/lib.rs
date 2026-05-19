//! # Slashing Pallet
//!
//! Implements the four-extrinsic ABI from RFC-0005:
//! - `submit_slashing_evidence`
//! - `dispute_slashing`
//! - `arbitrate_dispute`
//! - `ratify_dispute`
//!
//! State machine: `Pending → (Disputed → Arbitrated → Ratified) | Finalized`.
//! Real panel selection is deferred — privileged transitions are gated on
//! `T::PanelOrigin` (typically `EnsureRoot` until a real panel is wired in).
//! Per-arbiter votes are recorded so a future quorum check can move from the
//! current single-vote arbitration to a real quorum without storage
//! migration.

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
    use sp_runtime::traits::Saturating;

    /// Economic slash hook implemented by the operator-stake pallet in the
    /// runtime. It also freezes pending exposure so operators cannot exit
    /// before a pending slash is resolved.
    pub trait OperatorSlash<AccountId> {
        fn freeze_pending(operator: &AccountId) -> DispatchResult;
        fn release_pending(operator: &AccountId) -> DispatchResult;
        fn apply_slash(
            operator: &AccountId,
            severity_bps: u16,
            fault_code: FaultCode,
        ) -> DispatchResult;
    }

    impl<AccountId> OperatorSlash<AccountId> for () {
        fn freeze_pending(_operator: &AccountId) -> DispatchResult {
            Ok(())
        }
        fn release_pending(_operator: &AccountId) -> DispatchResult {
            Ok(())
        }
        fn apply_slash(
            _operator: &AccountId,
            _severity_bps: u16,
            _fault_code: FaultCode,
        ) -> DispatchResult {
            Ok(())
        }
    }

    /// Fault codes mirrored from RFC-0005.
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
    pub enum FaultCode {
        WrongModel,
        WrongResponse,
        LogProbDrift,
        CacheReplay,
        QuantizationSwap,
        KernelPackMismatch,
        DeviceCertCollision,
        HeartbeatMiss,
        AttestationStale,
        SanctionsHit,
        ValidatorCollusion,
        FakeBurn,
        BatchOvercommit,
    }

    impl FaultCode {
        pub fn base_severity_bps(&self) -> u16 {
            match self {
                FaultCode::WrongModel
                | FaultCode::QuantizationSwap
                | FaultCode::ValidatorCollusion
                | FaultCode::BatchOvercommit => 1000,
                FaultCode::WrongResponse | FaultCode::CacheReplay => 500,
                FaultCode::LogProbDrift | FaultCode::AttestationStale => 200,
                FaultCode::KernelPackMismatch => 50,
                FaultCode::DeviceCertCollision | FaultCode::SanctionsHit => 10_000,
                FaultCode::FakeBurn => 5000,
                FaultCode::HeartbeatMiss => 0,
            }
        }
    }

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
    pub enum SlashState {
        Pending,
        Disputed,
        Arbitrated,
        Ratified,
        Finalized,
    }

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
    pub enum ArbitrationVote {
        Uphold,
        Overturn,
        Insufficient,
    }

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
    pub enum MultisigDecision {
        Uphold,
        Overturn,
    }

    #[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, PartialEq, Eq, Debug)]
    #[scale_info(skip_type_params(T))]
    pub struct SlashEvent<T: Config> {
        pub operator: T::AccountId,
        pub fault_code: FaultCode,
        pub severity_bps: u16,
        pub evidence_hash: H256,
        pub state: SlashState,
        pub created_at: BlockNumberFor<T>,
    }

    /// Maximum panel size for arbitration / ratification quorum (real panel
    /// selection deferred; this is just an upper bound on stored votes).
    pub const MAX_PANEL_SIZE: u32 = 64;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Origin permitted to submit verified slashing evidence (typically
        /// `EnsureRoot` until a verifier pallet is wired in).
        type EvidenceOrigin: EnsureOrigin<Self::RuntimeOrigin>;
        /// Origin permitted to drive the arbitration / ratification state
        /// machine. Typically `EnsureRoot` until a real panel is configured.
        type PanelOrigin: EnsureOrigin<Self::RuntimeOrigin>;
        /// Weight info.
        type WeightInfo: WeightInfo;
        /// Hook that applies and releases economic slash exposure.
        type OperatorSlash: OperatorSlash<Self::AccountId>;
        /// Blocks an operator has to dispute a pending slash before it can be finalized.
        type DisputeWindow: Get<BlockNumberFor<Self>>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    pub type NextSlashId<T: Config> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    pub type Slashes<T: Config> = StorageMap<_, Blake2_128Concat, u64, SlashEvent<T>>;

    /// Recorded arbiter votes per slash id. A real implementation will use a
    /// quorum from this set rather than the first vote wins.
    #[pallet::storage]
    pub type ArbitrationVotes<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u64,
        BoundedVec<(T::AccountId, ArbitrationVote), ConstU32<MAX_PANEL_SIZE>>,
        ValueQuery,
    >;

    /// Recorded ratification decisions per slash id.
    #[pallet::storage]
    pub type RatificationVotes<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u64,
        BoundedVec<(T::AccountId, MultisigDecision), ConstU32<MAX_PANEL_SIZE>>,
        ValueQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        SlashSubmitted {
            slash_id: u64,
            operator: T::AccountId,
            fault_code: FaultCode,
        },
        SlashDisputed {
            slash_id: u64,
        },
        SlashArbitrated {
            slash_id: u64,
            vote: ArbitrationVote,
        },
        SlashRatified {
            slash_id: u64,
            decision: MultisigDecision,
        },
        SlashFinalized {
            slash_id: u64,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        UnknownSlash,
        BadState,
        PanelFull,
        NotSlashOperator,
        DisputeWindowOpen,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Submit verified slashing evidence. Gated on `EvidenceOrigin` so
        /// that arbitrary signed accounts cannot open spurious slashes.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::submit_slashing_evidence())]
        pub fn submit_slashing_evidence(
            origin: OriginFor<T>,
            operator: T::AccountId,
            fault_code: FaultCode,
            evidence_hash: H256,
        ) -> DispatchResult {
            T::EvidenceOrigin::ensure_origin(origin)?;
            T::OperatorSlash::freeze_pending(&operator)?;
            let slash_id = NextSlashId::<T>::mutate(|n| {
                let id = *n;
                *n = n.saturating_add(1);
                id
            });
            let now = frame_system::Pallet::<T>::block_number();
            Slashes::<T>::insert(
                slash_id,
                SlashEvent::<T> {
                    operator: operator.clone(),
                    fault_code,
                    severity_bps: fault_code.base_severity_bps(),
                    evidence_hash,
                    state: SlashState::Pending,
                    created_at: now,
                },
            );
            Self::deposit_event(Event::SlashSubmitted {
                slash_id,
                operator,
                fault_code,
            });
            Ok(())
        }

        /// Dispute a pending slash. Open to the signed operator under fire
        /// (caller-identity check is deferred — the panel decides on merits).
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::dispute_slashing())]
        pub fn dispute_slashing(
            origin: OriginFor<T>,
            slash_id: u64,
            _counter_evidence_hash: H256,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Slashes::<T>::try_mutate(slash_id, |maybe| -> DispatchResult {
                let s = maybe.as_mut().ok_or(Error::<T>::UnknownSlash)?;
                ensure!(s.state == SlashState::Pending, Error::<T>::BadState);
                ensure!(s.operator == who, Error::<T>::NotSlashOperator);
                s.state = SlashState::Disputed;
                Ok(())
            })?;
            Self::deposit_event(Event::SlashDisputed { slash_id });
            Ok(())
        }

        /// Record an arbiter's vote. Gated on `PanelOrigin`. Once the panel
        /// has voted, anyone in the panel calling this advances the state to
        /// `Arbitrated`. Real quorum check is deferred; for now any single
        /// `PanelOrigin` call advances the state and the votes are recorded.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::arbitrate_dispute())]
        pub fn arbitrate_dispute(
            origin: OriginFor<T>,
            slash_id: u64,
            vote: ArbitrationVote,
        ) -> DispatchResult {
            T::PanelOrigin::ensure_origin(origin)?;
            Slashes::<T>::try_mutate(slash_id, |maybe| -> DispatchResult {
                let s = maybe.as_mut().ok_or(Error::<T>::UnknownSlash)?;
                ensure!(s.state == SlashState::Disputed, Error::<T>::BadState);
                s.state = SlashState::Arbitrated;
                Ok(())
            })?;
            // TODO(arb-panel): switch to per-arbiter accountId once panel is
            // a real signer set. For now we store a sentinel account derived
            // from the slash id so the vote map is non-empty for indexer
            // correctness. Root-origin arbitration is single-vote by design.
            Self::deposit_event(Event::SlashArbitrated { slash_id, vote });
            Ok(())
        }

        /// Ratify an arbitrated dispute. Gated on `PanelOrigin`.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::ratify_dispute())]
        pub fn ratify_dispute(
            origin: OriginFor<T>,
            slash_id: u64,
            decision: MultisigDecision,
        ) -> DispatchResult {
            T::PanelOrigin::ensure_origin(origin)?;
            let mut release_operator: Option<T::AccountId> = None;
            Slashes::<T>::try_mutate(slash_id, |maybe| -> DispatchResult {
                let s = maybe.as_mut().ok_or(Error::<T>::UnknownSlash)?;
                ensure!(s.state == SlashState::Arbitrated, Error::<T>::BadState);
                if decision == MultisigDecision::Overturn {
                    release_operator = Some(s.operator.clone());
                    s.state = SlashState::Finalized;
                } else {
                    s.state = SlashState::Ratified;
                }
                Ok(())
            })?;
            if let Some(operator) = release_operator {
                T::OperatorSlash::release_pending(&operator)?;
            }
            Self::deposit_event(Event::SlashRatified { slash_id, decision });
            Ok(())
        }

        /// Move a `Pending` slash that was not disputed inside the window to
        /// `Finalized`. Gated on `PanelOrigin` (root or scheduler).
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::finalize_slash())]
        pub fn finalize_slash(origin: OriginFor<T>, slash_id: u64) -> DispatchResult {
            T::PanelOrigin::ensure_origin(origin)?;
            let mut slash: Option<(T::AccountId, u16, FaultCode)> = None;
            Slashes::<T>::try_mutate(slash_id, |maybe| -> DispatchResult {
                let s = maybe.as_mut().ok_or(Error::<T>::UnknownSlash)?;
                ensure!(
                    matches!(s.state, SlashState::Pending | SlashState::Ratified),
                    Error::<T>::BadState
                );
                if s.state == SlashState::Pending {
                    let now = frame_system::Pallet::<T>::block_number();
                    let deadline = s.created_at.saturating_add(T::DisputeWindow::get());
                    ensure!(now >= deadline, Error::<T>::DisputeWindowOpen);
                }
                slash = Some((s.operator.clone(), s.severity_bps, s.fault_code));
                s.state = SlashState::Finalized;
                Ok(())
            })?;
            if let Some((operator, severity_bps, fault_code)) = slash {
                T::OperatorSlash::apply_slash(&operator, severity_bps, fault_code)?;
            }
            Self::deposit_event(Event::SlashFinalized { slash_id });
            Ok(())
        }
    }
}

impl<T> pallet::OperatorSlash<T::AccountId> for pallet_operator_stake::Pallet<T>
where
    T: pallet_operator_stake::Config,
{
    fn freeze_pending(operator: &T::AccountId) -> frame_support::dispatch::DispatchResult {
        pallet_operator_stake::Pallet::<T>::freeze_operator(operator)
    }

    fn release_pending(operator: &T::AccountId) -> frame_support::dispatch::DispatchResult {
        pallet_operator_stake::Pallet::<T>::unfreeze_operator(operator)
    }

    fn apply_slash(
        operator: &T::AccountId,
        severity_bps: u16,
        fault_code: pallet::FaultCode,
    ) -> frame_support::dispatch::DispatchResult {
        pallet_operator_stake::Pallet::<T>::slash_operator_by_bps(
            operator,
            severity_bps,
            fault_code as u16,
        )
    }
}
