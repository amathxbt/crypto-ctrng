**WARNING: This project is experimental and work-in-progress, use at your own risk!**

## Overview

This crate provides a lightweight and secure abstraction layer over a cosmic True Random Number Generator (cTRNG) backend.
It allows deterministic or hardware-backed randomness to be consumed through the standard `RngCore` and `CryptoRng` traits from `rand_core`.

The main goals is to expose a clean, composable API for hardware or remote entropy backends. It can serve as the randomness foundation for distributed protocols such as threshold ECDSA, or MPC systems that require reproducible yet secure entropy sources.

### Entropy ownership contract

This crate's responsibility is to **verify** that the cTRNG backend provides unique blocks. The randomness flow is split between three layers:

- **Gateway/back-end** (e.g. IPFS beacon, hardware TRNG) is responsible for generating unique 32-byte blocks with monotonic timestamps. This responsibility sits outside of this crate.
- **This crate** (`crypto-ctrng`) verifies uniqueness by enforcing timestamp monotonicity and rejecting duplicate blocks.
- **Application/client code** (e.g. TECDSA library) is responsible for personalizing the provided blocks using `derive_seed(execution_id, party_id, counter, block)` to domain-separate pulls so a single user never receives the same derived value twice, even if they consume the same cTRNG block as other users (but this is something that we are looking to prevent in the future).

**Note on entropy mixing:**
- `MixedCtrngClient` combines entropy sources (XOR of IPFS beacon + local OS randomness) for enhanced security, but still relies on the gateway for uniqueness guarantees.

**Uniqueness guarantees:**
- This crate ensures that each `next_block()` call returns a block with a timestamp strictly greater than the previous one, preventing global block reuse (I thought so, but according to today's bug, we can have same ctrng for two different timestamp, so we need to find another way).
- The calling library must use `derive_seed()` to personalize blocks per user/execution to prevent collisions between different users reading the same raw block.

## Usage

To use this crate, add it to your `Cargo.toml`:

```toml
[dependencies]
crypto-ctrng = { git = "https://github.com/spacecomputer-io/crypto-ctrng.git", tag = "v0.1.0" }
```

Then, you can use it in your code:

```rust
use ctrng_rng::{
    CtrngRng, MockCtrngClient, RandomBlockSource
};
        
let gateway = "https://ipfs.io";
let beacon_key = "k2k4r8pigrw8i34z63om8f015tt5igdq0c46xupq8spp1bogt35k5vhe";
let mut ctrng = crypto_ctrng::IpfsCtrngClient::new(gateway, beacon_key);
let seed = ctrng.next_block().expect("failed to fetch IPFS block")
```

## Tests

To run offline tests : 

```cargo tests```

To run offline and online tests : 

```cargo test -- --include ignored```


## License

This project is licensed under the terms of the MIT License. See the [LICENSE](LICENSE) file for details.

## Contributing

We welcome contributions to this project! If you have suggestions for improvements or new features, please open an issue or submit a pull request.
