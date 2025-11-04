use rand_chacha::ChaCha20Rng;
use rand_core::{RngCore, SeedableRng};
use sha2::{Digest, Sha256};

use crate::rng::CtrngRng;

pub fn derive_seed(execution_id: &[u8], party_id: u16, counter: u64, trng_block: &[u8; 32]) -> [u8; 32] {
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

#[derive(Clone, Debug)]
pub struct MockCtrngClient<Rng = ChaCha20Rng> {
    inner: Rng,
}

impl<Rng> MockCtrngClient<Rng> {
    pub fn new(inner: Rng) -> Self { Self { inner } }
}

impl<Rng: RngCore> crate::traits::RandomBlockSource for MockCtrngClient<Rng> {
    fn next_block(&mut self) -> Result<[u8; 32], crate::error::CtrngError> {
        let mut block = [0u8; 32];
        self.inner.fill_bytes(&mut block);
        Ok(block)
    }
}

impl MockCtrngClient<ChaCha20Rng> {
    pub fn from_seed(seed: [u8; 32]) -> Self { 
        Self::new(ChaCha20Rng::from_seed(seed)) 
    }
}

pub fn rng_from_seed_block(seed_block: [u8; 32]) -> CtrngRng<MockCtrngClient<ChaCha20Rng>> {
    CtrngRng::new(MockCtrngClient::from_seed(seed_block)).expect("MockCtrngClient::from_seed cannot fail")
}
