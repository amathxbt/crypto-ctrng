use crate::error::SourceError;
use crate::local::LocalRng;
use crate::traits::RandomBlockSource;

/// Mixes remote and local entropy (XOR).
/// This provides defense in depth: even if the remote source is compromised,
/// the local entropy ensures unpredictability.
#[derive(Debug)]
pub struct MixedCtrng<R: RandomBlockSource> {
    remote: R,
    local: LocalRng,
}

impl<R: RandomBlockSource> MixedCtrng<R> {
    pub fn new(remote: R) -> Result<Self, SourceError> {
        Ok(Self {
            remote,
            local: LocalRng::new(),
        })
    }
}

impl<R: RandomBlockSource> RandomBlockSource for MixedCtrng<R> {
    fn next_block(&mut self) -> Result<[u8; 32], SourceError> {
        let remote_block = self.remote.next_block()?;
        let local_block = self.local.next_block()?;

        let mut mixed_block = [0u8; 32];
        for i in 0..32 {
            mixed_block[i] = remote_block[i] ^ local_block[i];
        }
        Ok(mixed_block)
    }
}
