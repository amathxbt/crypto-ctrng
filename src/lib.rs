use rand_chacha::ChaCha20Rng;
use rand_core::{impls, CryptoRng, Error, RngCore, SeedableRng};
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

#[derive(Debug, serde::Deserialize)]
struct BeaconData {
    sequence: u64,
    timestamp: u64,
    ctrng: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct BeaconResponse {
    previous: String,
    data: BeaconData,
}


/// Errors that can occur while interacting with a cosmic TRNG backend.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CtrngError {
    #[error("cTRNG backend failure: {0}")]
    Backend(String),
}

impl CtrngError {
    pub fn backend(msg: impl Into<String>) -> Self {
        Self::Backend(msg.into())
    }
}

impl From<CtrngError> for rand_core::Error {
    fn from(err: CtrngError) -> Self {
        rand_core::Error::new(err.to_string())
    }
}

/// A source of 32-byte randomness blocks.
pub trait RandomBlockSource {
    /// Fetch the next 32 random bytes from the TRNG backend.
    fn next_block(&mut self) -> Result<[u8; 32], CtrngError>;
}

/// [`CryptoRng`] adaptor that consumes 32-byte blocks from a TRNG backend.
#[derive(Debug)]
pub struct CtrngRng<C> {
    client: C,
    buffer: [u8; 32],
    cursor: usize,
}

impl<C: RandomBlockSource> CtrngRng<C> {
    pub fn new(mut client: C) -> Result<Self, CtrngError> {
        let buffer = client.next_block()?;
        Ok(Self {
            client,
            buffer,
            cursor: 0,
        })
    }

    pub fn client_mut(&mut self) -> &mut C {
        &mut self.client
    }

    fn refill(&mut self) -> Result<(), CtrngError> {
        self.buffer.zeroize();
        self.buffer = self.client.next_block()?;
        self.cursor = 0;
        Ok(())
    }

    fn fill_bytes_internal(&mut self, dest: &mut [u8]) -> Result<(), CtrngError> {
        let mut filled = 0;
        while filled < dest.len() {
            if self.cursor == self.buffer.len() {
                self.refill()?;
            }

            let available = self.buffer.len() - self.cursor;
            let needed = dest.len() - filled;
            let to_copy = available.min(needed);

            dest[filled..filled + to_copy]
                .copy_from_slice(&self.buffer[self.cursor..self.cursor + to_copy]);

            self.cursor += to_copy;
            filled += to_copy;
        }
        Ok(())
    }

    pub fn try_fill_bytes_fallible(&mut self, dest: &mut [u8]) -> Result<(), CtrngError> {
        self.fill_bytes_internal(dest)
    }
}

impl<C: RandomBlockSource> RngCore for CtrngRng<C> {
    fn next_u32(&mut self) -> u32 {
        impls::next_u32_via_fill(self)
    }

    fn next_u64(&mut self) -> u64 {
        impls::next_u64_via_fill(self)
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        if let Err(err) = self.fill_bytes_internal(dest) {
            panic!("cTRNG error while filling bytes: {err:?}");
        }
    }
    

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
        self.fill_bytes_internal(dest).map_err(Error::from)
    }
}

impl<C: RandomBlockSource> CryptoRng for CtrngRng<C> {}

impl<C> Drop for CtrngRng<C> {
    fn drop(&mut self) {
        self.buffer.zeroize();
        self.cursor.zeroize();
    }
}

/// Randomness backend that pulls 32-byte blocks from an IPFS beacon
#[derive(Debug)]
pub struct IpfsCtrngClient {
    gateway_base: String,
    beacon_key: String,

    // cache of decoded 32-byte values from the last fetch
    cache: Vec<[u8; 32]>,
    cursor: usize,
}

impl IpfsCtrngClient {
    /// Build a new IPFS-backed client.
    ///
    /// gateway_base: base URL of the IPFS gateway (ex: "https://ipfs.io")
    /// beacon_key: IPNS key for the beacon (ex: "k2k4r8pigrw8i34z63om8f015tt5igdq0c46xupq8spp1bogt35k5vhe").
    pub fn new(gateway_base: impl Into<String>, beacon_key: impl Into<String>) -> Self {
        Self {
            gateway_base: gateway_base.into(),
            beacon_key: beacon_key.into(),
            cache: Vec::new(),
            cursor: 0,
        }
    }

    /// Make a blocking HTTP GET to the beacon "latest" endpoint and
    /// refresh our cache with the newly advertised ctrng entries.
    fn refill_cache(&mut self) -> Result<(), CtrngError> {
        let url = format!(
            "{}/ipns/{}",
            self.gateway_base.trim_end_matches('/'),
            self.beacon_key
        );

        let resp = reqwest::blocking::get(&url)
            .map_err(|e| CtrngError::backend(format!("ipfs fetch failed: {e}")))?;

        if !resp.status().is_success() {
            return Err(CtrngError::backend(format!(
                "ipfs returned HTTP {}",
                resp.status()
            )));
        }

        let bytes = resp
            .bytes()
            .map_err(|e| CtrngError::backend(format!("ipfs body read failed: {e}")))?;

        let parsed: BeaconResponse = serde_json::from_slice(&bytes)
            .map_err(|e| CtrngError::backend(format!("ipfs json parse failed: {e}")))?;

        // Decode the ctrng hex strings into [u8; 32]
        let mut new_cache = Vec::with_capacity(parsed.data.ctrng.len());
        for hex_str in parsed.data.ctrng.iter() {
            let raw = hex::decode(hex_str)
                .map_err(|e| CtrngError::backend(format!("invalid hex in ctrng entry: {e}")))?;

            if raw.len() != 32 {
                return Err(CtrngError::backend(format!(
                    "ctrng entry length is {}, expected 32 bytes",
                    raw.len()
                )));
            }

            let mut block = [0u8; 32];
            block.copy_from_slice(&raw);
            new_cache.push(block);
        }

        // If we didn't get anything usable, this is an error
        if new_cache.is_empty() {
            return Err(CtrngError::backend("ipfs beacon returned empty ctrng list"));
        }

        // Install new cache
        self.cache = new_cache;
        self.cursor = 0;

        Ok(())
    }
}

impl RandomBlockSource for IpfsCtrngClient {
    fn next_block(&mut self) -> Result<[u8; 32], CtrngError> {
        // If we still have unused cached blocks, serve the next one
        if self.cursor < self.cache.len() {
            let block = self.cache[self.cursor];
            self.cursor += 1;
            return Ok(block);
        }

        // Otherwise, fetch latest from IPFS and refill cache
        self.refill_cache()?;

        // After refill, we must have at least one entry
        let block = self.cache[self.cursor];
        self.cursor += 1;
        Ok(block)
    }
}

/// Deterministic cTRNG mock that can be seeded to produce reproducible runs.
#[derive(Clone, Debug)]
pub struct MockCtrngClient<Rng = ChaCha20Rng> {
    inner: Rng,
}

impl<Rng> MockCtrngClient<Rng> {
    /// Wrap an arbitrary RNG to act as the TRNG backend.
    pub fn new(inner: Rng) -> Self {
        Self { inner }
    }
}

impl<Rng: RngCore> RandomBlockSource for MockCtrngClient<Rng> {
    fn next_block(&mut self) -> Result<[u8; 32], CtrngError> {
        let mut block = [0u8; 32];
        self.inner.fill_bytes(&mut block);
        Ok(block)
    }
}

impl MockCtrngClient<ChaCha20Rng> {
    /// Seed the mock with deterministic entropy.
    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self::new(ChaCha20Rng::from_seed(seed))
    }
}

/// Deterministically derive a 32-byte seed by mixing the execution ID, party ID,
/// per-stage counter and the raw cTRNG block.
pub fn derive_seed(
    execution_id: &[u8],
    party_id: u16,
    counter: u64,
    trng_block: &[u8; 32],
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(execution_id);
    hasher.update(party_id.to_be_bytes());
    hasher.update(counter.to_be_bytes());
    hasher.update(trng_block);
    let digest = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

/// Build a local, deterministic RNG from a 32-byte seed block.
pub fn rng_from_seed_block(
    seed_block: [u8; 32],
) -> CtrngRng<MockCtrngClient<ChaCha20Rng>> {
    // This is safe because CtrngRng::new only fails if next_block() fails,
    // and MockCtrngClient::from_seed is deterministic and infallible.
    CtrngRng::new(
        MockCtrngClient::from_seed(seed_block)
    ).expect("MockCtrngClient::from_seed cannot fail")
}

#[cfg(test)]
mod tests {
    use super::{CtrngError, CtrngRng, MockCtrngClient, RandomBlockSource, derive_seed, rng_from_seed_block};
    use rand_core::RngCore;
    use std::collections::HashSet;

    #[test]
    fn mock_ctrng_emits_unique_blocks() {
        let mut client = MockCtrngClient::from_seed([0x42; 32]);
        let first = client.next_block().unwrap();
        let second = client.next_block().unwrap();
        assert_ne!(first, second);
    }

    #[test]
    fn ctrng_rng_does_not_repeat_blocks_under_mock() {
        let mut rng = CtrngRng::new(MockCtrngClient::from_seed([0x33; 32]))
            .expect("mock should yield first block");
        let mut seen = HashSet::new();
        for idx in 0..128 {
            let mut block = [0u8; 32];
            rng.try_fill_bytes_fallible(&mut block)
                .expect("mock should not fail");
            assert!(seen.insert(block), "duplicate block observed at draw {idx}");
        }
    }

    #[test]
    fn flaky_source_propagates_error() {
        struct Flaky {
            ok_block: [u8; 32],
            fail_every: usize,
            calls: usize,
        }

        impl RandomBlockSource for Flaky {
            fn next_block(&mut self) -> Result<[u8; 32], CtrngError> {
                self.calls += 1;
                if self.calls % self.fail_every == 0 {
                    Err(CtrngError::backend(format!(
                        "backend failure after {} calls",
                        self.calls
                    )))
                } else {
                    Ok(self.ok_block)
                }
            }
        }

        let mut rng = CtrngRng::new(Flaky {
            ok_block: [0x11; 32],
            fail_every: 3,
            calls: 0,
        })
        .expect("first block should succeed");

        // first fill works
        let mut buf = [0u8; 64];
        rng.try_fill_bytes_fallible(&mut buf)
            .expect("second block ok");

        // second fill triggers failure
        let mut buf2 = [0u8; 64];
        let err = rng
            .try_fill_bytes_fallible(&mut buf2)
            .expect_err("third call must error");
        assert!(matches!(err, CtrngError::Backend(_)));

        // rand_core::try_fill_bytes should surface the same error code on a failing backend.
        let mut rng_trait = CtrngRng::new(Flaky {
            ok_block: [0x22; 32],
            fail_every: 2,
            calls: 0,
        })
        .expect("first block should succeed");
        let mut buf3 = [0u8; 64];
        assert!(rng_trait.try_fill_bytes(&mut buf3).is_err());
    }

    #[test]
    fn derive_seed_yields_unique_material() {
        let exec = b"derive-seed-exec-id";
        let trng = [0xAA; 32];
        let mut seen = HashSet::new();
        for ctr in 0..256u64 {
            let seed = derive_seed(exec, 1, ctr, &trng);
            assert!(
                seen.insert(seed),
                "duplicate seed for counter {ctr}"
            );
        }
    }

    #[test]
    fn rng_from_seed_block_is_deterministic_for_same_seed() {
        let seed = [0x11; 32];

        let mut rng1 = rng_from_seed_block(seed);
        let mut rng2 = rng_from_seed_block(seed);

        let mut buf1 = [0u8; 64];
        let mut buf2 = [0u8; 64];

        rng1.try_fill_bytes_fallible(&mut buf1).unwrap();
        rng2.try_fill_bytes_fallible(&mut buf2).unwrap();

        assert_eq!(buf1, buf2, "same seed should give identical output");
    }

    #[test]
    fn rng_from_seed_block_differs_for_different_seeds() {
        let mut rng_a = rng_from_seed_block([0x11; 32]);
        let mut rng_b = rng_from_seed_block([0x22; 32]);

        let mut buf_a = [0u8; 64];
        let mut buf_b = [0u8; 64];

        rng_a.try_fill_bytes_fallible(&mut buf_a).unwrap();
        rng_b.try_fill_bytes_fallible(&mut buf_b).unwrap();

        assert_ne!(buf_a, buf_b, "different seeds should give different output");
    }

    #[test]
    #[ignore] //todo : add an offline test for the ipfs client
    fn ipfs_client_can_fetch_live_block() {
        let gateway = "https://ipfs.io";
        let beacon_key = "k2k4r8pigrw8i34z63om8f015tt5igdq0c46xupq8spp1bogt35k5vhe";

        let mut client = super::IpfsCtrngClient::new(gateway, beacon_key);

        let block = client.next_block()
            .expect("should fetch a 32-byte block from the beacon");

        assert_eq!(block.len(), 32, "block must be exactly 32 bytes");
    }

}
