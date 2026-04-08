# Project context

> For AI assistants.

## Purpose

**crypto-ctrng** is a Rust crate that provides secure randomness sourced from a cosmic True Random Number Generator (cTRNG) backend. It wraps this entropy into the standard `rand_core::RngCore` / `CryptoRng` traits so any Rust cryptographic library can consume it directly.

The crate does **not** generate randomness itself; it **fetches, validates, and delivers** 32-byte blocks from one or more remote backends, enforcing timestamp monotonicity and duplicate detection.

## Architecture

```
User code
  │
  ▼
Ctrng          ← public enum; one variant per backend (currently Ipfs)
  │
  ├─► IpfsCtrng   ← fetches JSON beacon via IPFS gateways, caches blocks
  │
  ├─► (future: DataHeavenCtrng, hardware TRNG, …)
  │
  ▼
RandomBlockSource trait    ← fn next_block() -> Result<[u8; 32], SourceError>
  │
  ▼
MixedCtrng     ← XORs remote block with OS entropy (defense in depth)
  │
  ▼
ReseedingRng   ← ChaCha20-based DRBG, auto-reseeds from source
  │
  ▼
RngCore / CryptoRng   ← standard rand_core traits
```

## Key types

| Type | File | Role |
|------|------|------|
| `RandomBlockSource` | `src/traits.rs` | Trait every entropy backend must implement |
| `Ctrng` | `src/ctrng/mod.rs` | Public enum dispatching to concrete backends |
| `IpfsCtrng` | `src/ctrng/ipfs.rs` | IPFS beacon client (internal, behind `Ctrng`) |
| `IpfsConfig` | `src/ctrng/ipfs.rs` | Gateway list, `use_defaults`, timeout |
| `IpfsGateway` | `src/ctrng/ipfs.rs` | Newtype for gateway URL; builds `/ipfs/` and `/ipns/` paths |
| `CtrngBlock` | `src/ctrng/types.rs` | Block with sequence number, timestamp, 32-byte data |
| `MixedCtrng<R>` | `src/mixed.rs` | XOR of remote + local OS entropy |
| `ReseedingRng` | `src/reseed.rs` | ChaCha20 DRBG with configurable reseed intervals |
| `BlockRng` | `src/rng.rs` | Thin `RngCore` adapter over any `RandomBlockSource` |
| `LocalRng` | `src/local.rs` | OS-level randomness (`getrandom`) |
| `SourceError` | `src/error.rs` | Error enum: `Ctrng(String)` / `Os(String)` |

## File layout

```
src/
├── lib.rs             public re-exports
├── traits.rs          RandomBlockSource trait
├── error.rs           SourceError
├── ctrng/
│   ├── mod.rs         Ctrng enum + RandomBlockSource impl
│   ├── ipfs.rs        IpfsCtrng, IpfsGateway, IpfsConfig, DEFAULT_GATEWAYS
│   └── types.rs       BeaconResponse, CtrngBlock
├── mixed.rs           MixedCtrng (XOR mixer)
├── reseed.rs          ReseedingRng, ReseedConfig
├── rng.rs             BlockRng (RngCore adapter)
└── local.rs           LocalRng (OS entropy)

tests/
├── ipfs_live.rs           live IPFS gateway tests (#[ignore])
├── mixed_tests.rs         MixedCtrng tests (some #[ignore])
├── reseed_tests.rs        ReseedingRng offline tests
├── rng_tests.rs           BlockRng error propagation
└── statistical_tests.rs   TestU01 / PractRand (behind features)
```

## Conventions

- **PRs**: follow [.github/pull_request_template.md](.github/pull_request_template.md) — compiles without warnings, `cargo test` passes, clear commits, link issue if applicable.
- **Tests**: `cargo test` runs offline tests only. Online (IPFS) tests are `#[ignore]` — run with `cargo test -- --include-ignored`.

## Useful commands

```bash
cargo test                              # offline unit + integration
cargo test -- --include-ignored         # includes live IPFS tests
cargo test --features testu01           # TestU01 SmallCrush
cargo test --features practrand         # PractRand (1 MiB)
```
