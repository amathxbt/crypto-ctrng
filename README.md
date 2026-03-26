**WARNING: This project is experimental and work-in-progress, use at your own risk!**

## Overview

This crate provides a lightweight and secure abstraction layer over a cosmic True Random Number Generator (cTRNG) backend.
It allows deterministic or hardware-backed randomness to be consumed through the standard `RngCore` and `CryptoRng` traits from `rand_core`.

The main goals is to expose a clean, composable API for hardware or remote entropy backends. It can serve as the randomness foundation for distributed protocols such as threshold ECDSA, or MPC systems that require reproducible yet secure entropy sources.

### Entropy ownership contract

This crate's responsibility is to **verify** that the cTRNG backend provides unique blocks. The randomness flow is split between three layers:

- **Gateway/back-end** (e.g. IPFS beacon, hardware TRNG) is responsible for generating unique 32-byte blocks with monotonic timestamps. This responsibility sits outside of this crate.
- **This crate** (`crypto-ctrng`) verifies uniqueness by enforcing timestamp monotonicity and rejecting duplicate blocks.
- **Application/client code** (e.g. TECDSA library) is responsible for personalizing the provided blocks to domain-separate pulls so a single user never receives the same derived value twice, even if they consume the same cTRNG block as other users (but this is something that we are looking to prevent in the future).

**Note on entropy mixing:**
- `MixedCtrng` combines entropy sources (XOR of IPFS beacon + local OS randomness) for enhanced security, but still relies on the gateway for uniqueness guarantees.

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

### Fetch raw entropy (default gateways)

```rust
use crypto_ctrng::{Ctrng, RandomBlockSource};

let beacon_key = "k2k4r8lvomw737sajfnpav0dpeernugnryng50uheyk1k39lursmn09f";

let mut ctrng = Ctrng::ipfs(beacon_key, None);
let block = ctrng.next_block().expect("failed to fetch IPFS block");
```

### Custom gateways with automatic fallback

If a gateway is unresponsive or down, the next one in the list is tried automatically.
The default timeout is 10 seconds per gateway.

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

Statistical testing is provided by the separate [`rng-statistical-tests`](https://github.com/spacecomputer-io/statistical-verification) library, which supports both **TestU01** and **PractRand**.

TestU01 and PractRand are automatically downloaded and built during compilation - no manual setup required.

To run statistical tests:

```bash
cargo test --features testu01     # TestU01 SmallCrush
cargo test --features practrand   # PractRand (1 MiB)
```


## License

This project is licensed under the terms of the MIT License. See the [LICENSE](LICENSE) file for details.

## Contributing

We welcome contributions to this project! If you have suggestions for improvements or new features, please open an issue or submit a pull request.
