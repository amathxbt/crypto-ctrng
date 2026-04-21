# SpaceComputer | crypto-ctrng

![spacecomputer logo](https://raw.githubusercontent.com/spacecomputer-io/media-kit/refs/heads/main/SpaceComputer/logo/SpaceComputer_banner.png)

![Tests](https://github.com/spacecomputer-io/crypto-ctrng/actions/workflows/rust.yml/badge.svg?branch=main)

This repository contains the crypto-ctrng project by SpaceComputer.

## Overview

This crate provides a lightweight and secure abstraction layer over a cosmic True Random Number Generator (cTRNG) backend. It allows deterministic or hardware-backed randomness to be consumed through the standard `RngCore` and `CryptoRng` traits from `rand_core`.

The main goals is to expose a clean, composable API for hardware or remote entropy backends.
[Orbitport](https://github.com/spacecomputer-io/orbitport) is the entity that maintains the ipfs randomness beacon. 

## Links

* [SpaceComputer docs](https://docs.spacecomputer.io)

### Entropy ownership contract

This crate's responsibility is to **verify** that the cTRNG backend provides unique blocks. The randomness flow is split between three layers:

- **Gateway/back-end** (e.g. randomness beacon, hardware TRNG) is responsible for generating unique 32-byte blocks with monotonic timestamps. This responsibility sits outside of this crate.
- **This crate** (`crypto-ctrng`) verifies uniqueness by enforcing timestamp monotonicity and rejecting duplicate blocks.
- **Application/client code** (e.g. TECDSA library) is responsible for personalizing the provided blocks to domain-separate pulls so a single user never receives the same derived value twice, even if they consume the same cTRNG block as other users (but this is something that we are looking to prevent in the future).

**Note on entropy mixing:**
- `MixedCtrng` combines entropy sources (XOR of randomness beacon + local OS randomness) for enhanced security.

**Uniqueness guarantees:**
- This crate ensures that each `next_block()` call returns a block with a timestamp strictly greater than the previous one, preventing global block reuse.
- The calling library must personalize blocks per user/execution to prevent collisions between different users reading the same raw block.

## Usage modes

| Mode | Code | DRBG | Reseed |
|------|------|------|--------|
| Raw entropy | `BlockRng::new(source)` | N/A | N/A |
| Seeded DRBG (testing) | `ChaCha20Rng::from_seed(seed)` | ChaCha20 | N/A |
| DRBG + auto reseed | `ReseedingRng::new(source)` | ChaCha20 | 3200 bytes / 1 week |
| DRBG + prediction resistance | `ReseedingRng::with_prediction_resistance(source)` | ChaCha20 | every call |

## Usage

To use this crate, add it to your `Cargo.toml`:

```toml
[dependencies]
crypto-ctrng = { git = "https://github.com/spacecomputer-io/crypto-ctrng.git", tag = "v0.1.0" }
```

### Fetch raw entropy (default ipfs gateways)

```rust
use crypto_ctrng::{Ctrng, RandomBlockSource};

let beacon_key = "k2k4r8lvomw737sajfnpav0dpeernugnryng50uheyk1k39lursmn09f";

let mut ctrng = Ctrng::ipfs(beacon_key, None);
let block = ctrng.next_block().expect("failed to fetch IPFS block");
```

For now, entropy gathered from ipfs gateways is public, two people gathering randomness from ipfs with the same beacon key and at the same time will get the same output. In the future, we plan to introduce private beacons. 

### Custom ipfs gateways with automatic fallback

If an ipfs gateway is unresponsive or down, the next one in the list is tried automatically.
The default timeout is 10 seconds per ipfs gateway.

```rust
use crypto_ctrng::{Ctrng, IpfsConfig, RandomBlockSource};
use std::time::Duration;

let beacon_key = "k2k4r8lvomw737sajfnpav0dpeernugnryng50uheyk1k39lursmn09f";

// Custom gateways + default gateways appended automatically
let config = IpfsConfig {
    gateways: vec!["https://my-gateway.example.com".into()],
    ..Default::default()
};
let mut ctrng = Ctrng::ipfs(beacon_key, Some(config));
let block = ctrng.next_block().expect("at least one gateway should respond");

// Custom gateways only (no defaults), with a custom timeout
let config = IpfsConfig {
    gateways: vec![
        "https://ipfs.filebase.io".into(),
        "https://ipfs.io".into(),
    ],
    use_defaults: false,
    timeout: Duration::from_secs(5),
};
let mut ctrng = Ctrng::ipfs(beacon_key, Some(config));
```

### DRBG with automatic reseeding (recommended)

```rust
use crypto_ctrng::{Ctrng, MixedCtrng, ReseedingRng};
use rand_core::RngCore;

let beacon_key = "k2k4r8lvomw737sajfnpav0dpeernugnryng50uheyk1k39lursmn09f";

let ipfs = Ctrng::ipfs(beacon_key, None);
let mixed = MixedCtrng::new(ipfs).unwrap();
let mut rng = ReseedingRng::new(mixed).unwrap();

let mut key = [0u8; 32];
rng.fill_bytes(&mut key);
```

### Custom reseed intervals

```rust
use std::time::Duration;
use crypto_ctrng::{Ctrng, MixedCtrng, ReseedConfig, ReseedingRng};

let beacon_key = "k2k4r8lvomw737sajfnpav0dpeernugnryng50uheyk1k39lursmn09f";

let ipfs = Ctrng::ipfs(beacon_key, None);
let mixed = MixedCtrng::new(ipfs).unwrap();

let config = ReseedConfig::new(1600, Duration::from_secs(3600)); // 50 blocks, 1 hour
let mut rng = ReseedingRng::with_config(mixed, config).unwrap();
```

## Tests

To run offline tests :

```bash
cargo test -- --skip test_e2e
```

To run the IPFS-backed end-to-end tests :

```bash
cargo test test_e2e
```

You can also run the dedicated e2e integration targets individually:

```bash
cargo test --test e2e_ipfs
cargo test --test e2e_mixed
```

### Statistical verification

Statistical testing is provided by the separate [`statistical-verification`](https://github.com/spacecomputer-io/statistical-verification) library, which supports both **TestU01** and **PractRand**.

TestU01 and PractRand are automatically downloaded and built during compilation - no manual setup required.

To run statistical tests:

```bash
cargo test --features testu01     # TestU01 SmallCrush
cargo test --features practrand   # PractRand (1 MiB)
```
