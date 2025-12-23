use crate::error::CtrngError;
use crate::local::LocalCtrngClient;
use crate::traits::RandomBlockSource;

/// A client that mixes randomness from a remote source with local OS randomness.
/// This provides defense in depth: even if the remote source is compromised,
/// the local entropy ensures unpredictability.
#[derive(Debug)]
pub struct MixedCtrngClient<R: RandomBlockSource> {
    remote_client: R,
    local_client: LocalCtrngClient,
}

impl<R: RandomBlockSource> MixedCtrngClient<R> {
    pub fn new(remote_client: R) -> Result<Self, CtrngError> {
        let local_client = LocalCtrngClient::new()?;
        Ok(Self {
            remote_client,
            local_client,
        })
    }
}

impl<R: RandomBlockSource> RandomBlockSource for MixedCtrngClient<R> {
    fn next_block(&mut self) -> Result<[u8; 32], CtrngError> {
        let remote_block = self.remote_client.next_block()?;
        let local_block = self.local_client.next_block()?;

        let mut mixed_block = [0u8; 32];
        for i in 0..32 {
            mixed_block[i] = remote_block[i] ^ local_block[i];
        }
        Ok(mixed_block)
    }
}

