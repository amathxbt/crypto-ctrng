use std::time::{Duration, Instant};

use rand_chacha::ChaCha20Rng;
use rand_core::{CryptoRng, Error, RngCore, SeedableRng, impls};
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

use crate::error::SourceError;
use crate::traits::RandomBlockSource;

pub const DEFAULT_RESEED_BYTES: u64 = 3200;
pub const DEFAULT_RESEED_TIME: Duration = Duration::from_secs(7 * 24 * 60 * 60);

const DRBG_DOMAIN_SEPARATOR: &[u8] = b"crypto-ctrng-drbg-seed-v1";

/// Configuration for automatic reseeding.
#[derive(Debug, Clone)]
pub struct ReseedConfig {
    pub reseed_interval_bytes: u64,
    pub reseed_interval_time: Duration,
}

impl Default for ReseedConfig {
    fn default() -> Self {
        Self {
            reseed_interval_bytes: DEFAULT_RESEED_BYTES,
            reseed_interval_time: DEFAULT_RESEED_TIME,
        }
    }
}

impl ReseedConfig {
    pub fn new(reseed_interval_bytes: u64, reseed_interval_time: Duration) -> Self {
        Self {
            reseed_interval_bytes,
            reseed_interval_time,
        }
    }

    pub fn prediction_resistant() -> Self {
        Self {
            reseed_interval_bytes: 0,
            reseed_interval_time: Duration::ZERO,
        }
    }
}

/// DRBG that automatically reseeds from a RandomBlockSource.
///
/// This struct uses an internal ChaCha20 DRBG and periodically reseeds from the source.
/// It reduces calls to expensive sources (IPFS, OS) while maintaining security.
#[derive(Debug)]
pub struct ReseedingRng<S: RandomBlockSource> {
    source: S,
    config: ReseedConfig,
    drbg: ChaCha20Rng,
    bytes_since_reseed: u64,
    last_reseed: Instant,
}

impl<S: RandomBlockSource> ReseedingRng<S> {
    pub fn new(source: S) -> Result<Self, SourceError> {
        Self::with_config(source, ReseedConfig::default())
    }

    pub fn with_config(mut source: S, config: ReseedConfig) -> Result<Self, SourceError> {
        let block = source.next_block()?;
        let seed = Sha256::digest([DRBG_DOMAIN_SEPARATOR, block.as_slice()].concat());
        let drbg = ChaCha20Rng::from_seed(seed.into());

        Ok(Self {
            source,
            config,
            drbg,
            bytes_since_reseed: 0,
            last_reseed: Instant::now(),
        })
    }

    pub fn with_prediction_resistance(source: S) -> Result<Self, SourceError> {
        Self::with_config(source, ReseedConfig::prediction_resistant())
    }

    pub fn needs_reseed(&self) -> bool {
        self.bytes_since_reseed >= self.config.reseed_interval_bytes
            || self.last_reseed.elapsed() >= self.config.reseed_interval_time
    }

    pub fn reseed(&mut self) -> Result<(), SourceError> {
        let block = self.source.next_block()?;
        let seed = Sha256::digest([DRBG_DOMAIN_SEPARATOR, block.as_slice()].concat());
        self.drbg = ChaCha20Rng::from_seed(seed.into());
        self.bytes_since_reseed = 0;
        self.last_reseed = Instant::now();
        Ok(())
    }

    pub fn config(&self) -> &ReseedConfig {
        &self.config
    }

    pub fn bytes_since_reseed(&self) -> u64 {
        self.bytes_since_reseed
    }

    pub fn time_since_reseed(&self) -> Duration {
        self.last_reseed.elapsed()
    }

    pub fn source_mut(&mut self) -> &mut S {
        &mut self.source
    }

    fn reseed_if_needed(&mut self) -> Result<(), SourceError> {
        if self.needs_reseed() {
            self.reseed()?;
        }
        Ok(())
    }

    fn fill_bytes_internal(&mut self, dest: &mut [u8]) -> Result<(), SourceError> {
        self.reseed_if_needed()?;
        self.drbg.fill_bytes(dest);
        self.bytes_since_reseed += dest.len() as u64;
        Ok(())
    }

    /// Like `try_fill_bytes` but returns `SourceError` for precise error handling.
    pub fn try_fill_bytes_source(&mut self, dest: &mut [u8]) -> Result<(), SourceError> {
        self.fill_bytes_internal(dest)
    }
}

impl<S: RandomBlockSource> RngCore for ReseedingRng<S> {
    fn next_u32(&mut self) -> u32 {
        impls::next_u32_via_fill(self)
    }

    fn next_u64(&mut self) -> u64 {
        impls::next_u64_via_fill(self)
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        if let Err(err) = self.fill_bytes_internal(dest) {
            panic!("ReseedingRng error: {err:?}");
        }
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
        self.fill_bytes_internal(dest).map_err(Error::from)
    }
}

impl<S: RandomBlockSource> CryptoRng for ReseedingRng<S> {}

impl<S: RandomBlockSource> Drop for ReseedingRng<S> {
    fn drop(&mut self) {
        // Overwrite the ChaCha20 DRBG state with volatile zero-writes before
        // releasing memory to the allocator.  A plain assignment
        // (`self.drbg = ChaCha20Rng::from_seed([0; 32])`) drops the old object
        // without first clearing its fields; the compiler is free to elide
        // "dead" stores, leaving keystream material in heap memory.
        // `ptr::write_bytes` with a subsequent compiler fence is not
        // elided because it is treated as an observable side-effect by LLVM.
        unsafe {
            let ptr = &mut self.drbg as *mut ChaCha20Rng as *mut u8;
            let len = std::mem::size_of::<ChaCha20Rng>();
            // Write zero bytes over the DRBG struct in place.
            std::ptr::write_bytes(ptr, 0, len);
            // Compiler fence prevents the store from being reordered or
            // removed by optimisation passes.
            std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
            // Re-initialise with a zero seed so the slot holds a valid object
            // for any subsequent Drop glue (e.g. if ChaCha20Rng has a non-trivial
            // drop impl in a future version of rand_chacha).
            std::ptr::write(&mut self.drbg, ChaCha20Rng::from_seed([0u8; 32]));
        }
        self.bytes_since_reseed.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = ReseedConfig::default();
        assert_eq!(config.reseed_interval_bytes, 3200);
        assert_eq!(
            config.reseed_interval_time,
            Duration::from_secs(7 * 24 * 60 * 60)
        );
    }

    #[test]
    fn prediction_resistant_config() {
        let config = ReseedConfig::prediction_resistant();
        assert_eq!(config.reseed_interval_bytes, 0);
        assert_eq!(config.reseed_interval_time, Duration::ZERO);
    }
}
