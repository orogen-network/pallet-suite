# pallet-suite

Cargo workspace for the Orogen's chain pallets and the runtime
composition that wires them together.

This repository is the **on-chain** half of the network (per master plan §1.1).
Worker, gateway, validator, wallet, explorer and CDN code lives in sibling
repos; this crate only owns FRAME pallets and runtime composition.

## Status

Skeleton pallets, bootable runtime. Every pallet stub compiles, every
pallet has a tiny mock runtime plus 2-3 unit tests, and the runtime crate
composes all 11 custom pallets plus the standard FRAME utility pallets
(`pallet-balances`, `pallet-aura`, `pallet-grandpa`, `pallet-timestamp`,
`pallet-transaction-payment`, `pallet-sudo`) into a runtime that boots
under `chain-node --dev --tmp` and produces blocks.

Business logic (Yuma math, full BME settlement, slash-escrow timer,
dispute panel selection, Merkle proofs, oracle aggregation) is deferred.
Each custom-pallet `Config` trait still carries only `RuntimeEvent`; full
config plumbing (currencies, weights, origin tightening) lands when the
runtime moves into `runtime-mainnet` / `runtime-testnet`.

## Layout

```
pallet-suite/
├── Cargo.toml                                  # workspace root
├── rust-toolchain.toml                         # Rust 1.94.1 stable
├── runtime/
│   ├── Cargo.toml                              # includes substrate-wasm-builder build-dep
│   ├── build.rs                                # emits WASM_BINARY constant
│   └── src/lib.rs                              # construct_runtime!() + impl_runtime_apis!()
└── pallets/
    ├── model-registry/                         # base models + LoRA adapters
    ├── operator-stake/                         # register / heartbeat / slash hook (RFC-0003)
    ├── job-market/                             # submit / assign / finalize / dispute
    ├── yuma-consensus/                         # validator weight vectors + epoch incentives
    ├── bme/                                    # burn-mint equilibrium (RFC-0004 batch path)
    ├── slashing/                               # 4-extrinsic ABI (RFC-0005)
    ├── pouw-mint/                              # cuPOW transcripts (deferred to Q4 2028)
    ├── attestation-registry/                   # multi-vendor TEE + CRL (RFC-0002)
    ├── oracle-twap/                            # 4–12h TWAP price (RFC-0008 shape)
    ├── nonce-vault/                            # customer-nonce 24h anti-replay (RFC-0007)
    └── treasury-ext/                           # foundation proposal/multisig hook
```

## Build & test

```bash
cd pallet-suite

# Compile everything (std).
cargo check --workspace

# All-features compile (try-runtime + runtime-benchmarks).
cargo check --workspace --all-features

# no_std sanity: a single pallet on wasm32.
cargo check -p pallet-model-registry --no-default-features --target wasm32-unknown-unknown

# Run all unit tests.
cargo test --workspace
```

The mock runtimes use FRAME's `derive_impl(TestDefaultConfig)`; tests exercise
event emission, storage round-trips, and state-machine transitions.

## Dependency stance

Pinned to the latest stable individual Substrate primitives (May 2026):

| Crate | Version |
|---|---|
| `frame-support` | 47.0.0 |
| `frame-system` | 47.0.0 |
| `sp-runtime` | 47.0.0 |
| `sp-core` | 41.0.0 |
| `sp-io` | 46.0.0 |
| `sp-std` | 14.0.0 |
| `parity-scale-codec` | 3.7 |
| `scale-info` | 2.11 |

The umbrella `polkadot-sdk` (2604.0.0) pulls hundreds of indirect dependencies
including parachain/cumulus/bridges that are not relevant at this stage. We
depend on individual primitives directly. Switching to the umbrella when the
node/RPC layer lands is a 1-line workspace change.

Rust toolchain is pinned to `1.94.1` (stable) via `rust-toolchain.toml`.

## RFC linkage

| Pallet | RFC |
|---|---|
| `pallet-attestation-registry` | RFC-0002 (multi-vendor attestation) |
| `pallet-operator-stake` | RFC-0003 (heartbeat schema) |
| `pallet-bme`, `pallet-job-market` | RFC-0004 (batch settlement) |
| `pallet-slashing` | RFC-0005 (slashing extrinsic ABI) |
| `pallet-nonce-vault` | RFC-0007 (customer nonce anti-replay) |
| `pallet-oracle-twap` | RFC-0008 (TWAP oracle, draft) |

Receipt format (RFC-0001) is consumed off-chain by `validator-replay`; only the
Merkle root and per-operator summary hashes touch the chain via `pallet-bme`
and `pallet-job-market`.

## Known follow-ups

- `RuntimeEvent` associated type is the deprecated explicit form; modern FRAME
  inlines the bound on `Config`. Migrating is mechanical and will land when
  `runtime-mainnet` is wired.
- All dispatchables use placeholder static weights; benchmarks generate real
  ones once a representative storage model exists.
- `slash` is gated by `ensure_signed` (skeleton); production wiring uses a
  custom `EnsureSlashingPallet` origin from `pallet-slashing`.
- Token balances inside the BME / operator-stake / treasury-ext pallets
  remain placeholder `u128` storage maps. `pallet-balances` is wired into
  the composed runtime for fees / sudo / dev accounts; integration with a
  custom `pallet-cuc` for protocol-level transfers happens in the
  `runtime-mainnet` / `runtime-testnet` crates.
- `pallet-pouw-mint::emit_pouw_reward` is gated behind a runtime-set `Enabled`
  flag; intended to remain off until Q4 2028.
- `impl_runtime_apis!` exports the standard API set (Core, BlockBuilder,
  TaggedTransactionQueue, OffchainWorkerApi, AuraApi, GrandpaApi,
  SessionKeys, AccountNonceApi, TransactionPaymentApi, Metadata,
  GenesisBuilder). `Benchmark` and `TryRuntime` APIs are deferred until
  the benchmarking story is on the critical path.

## License

Apache-2.0
