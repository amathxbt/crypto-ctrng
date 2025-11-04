**WARNING: This project is experimental and work-in-progress, use at your own risk!**

## Overview

This crate provides a lightweight and secure abstraction layer over a cosmic True Random Number Generator (cTRNG) backend.
It allows deterministic or hardware-backed randomness to be consumed through the standard `RngCore` and `CryptoRng` traits from `rand_core`.

The main goals is to expose a clean, composable API for hardware or remote entropy backends. It can serve as the randomness foundation for distributed protocols such as threshold ECDSA, or MPC systems that require reproducible yet secure entropy sources.

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
