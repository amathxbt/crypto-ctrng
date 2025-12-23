use std::time::{Duration, Instant};

use rand_chacha::ChaCha20Rng;
use rand_core::{CryptoRng, Error, RngCore, SeedableRng, impls};
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

use crate::error::CtrngError;
use crate::traits::RandomBlockSource;

pub const DEFAULT_RESEED_BYTES: u64 = 3200;
pub const DEFAULT_RESEED_TIME: Duration = Duration::from_secs(7 * 24 * 60 * 60);

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

#[derive(Debug)]
pub struct ReseedingRng<S: RandomBlockSource> {
    source: S,
    config: ReseedConfig,
    drbg: ChaCha20Rng,
    bytes_since_reseed: u64,
    last_reseed: Instant,
}

impl<S: RandomBlockSource> ReseedingRng<S> {
    pub fn new(source: S) -> Result<Self, CtrngError> {
        Self::with_config(source, ReseedConfig::default())
    }

    pub fn with_config(mut source: S, config: ReseedConfig) -> Result<Self, CtrngError> {
        let block = source.next_block()?;
        let seed = Sha256::digest([b"crypto-ctrng-drbg-seed-v1", block.as_slice()].concat());
        let drbg = ChaCha20Rng::from_seed(seed.into());

        Ok(Self {
            source,
            config,
            drbg,
            bytes_since_reseed: 0,
            last_reseed: Instant::now(),
        })
    }

    pub fn with_prediction_resistance(source: S) -> Result<Self, CtrngError> {
        Self::with_config(source, ReseedConfig::prediction_resistant())
    }

    pub fn needs_reseed(&self) -> bool {
        self.bytes_since_reseed >= self.config.reseed_interval_bytes
            || self.last_reseed.elapsed() >= self.config.reseed_interval_time
    }

    pub fn reseed(&mut self) -> Result<(), CtrngError> {
        let block = self.source.next_block()?;
        let seed = Sha256::digest([b"crypto-ctrng-drbg-seed-v1", block.as_slice()].concat());
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

    fn reseed_if_needed(&mut self) -> Result<(), CtrngError> {
        if self.needs_reseed() {
            self.reseed()?;
        }
        Ok(())
    }

    fn fill_bytes_internal(&mut self, dest: &mut [u8]) -> Result<(), CtrngError> {
        self.reseed_if_needed()?;
        self.drbg.fill_bytes(dest);
        self.bytes_since_reseed += dest.len() as u64;
        Ok(())
    }

    pub fn try_fill_bytes_fallible(&mut self, dest: &mut [u8]) -> Result<(), CtrngError> {
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
        self.drbg = ChaCha20Rng::from_seed([0u8; 32]);
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
        assert_eq!(config.reseed_interval_time, Duration::from_secs(7 * 24 * 60 * 60));
    }

    #[test]
    fn prediction_resistant_config() {
        let config = ReseedConfig::prediction_resistant();
        assert_eq!(config.reseed_interval_bytes, 0);
        assert_eq!(config.reseed_interval_time, Duration::ZERO);
    }
}
