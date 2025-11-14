use crate::error::CtrngError;
use crate::traits::RandomBlockSource;
use crate::ctrng::local::LocalCtrngClient;
use crate::ctrng::ipfs::IpfsCtrngClient;

#[derive(Debug)]
pub struct MixedCtrngClient {
    ipfs_client: IpfsCtrngClient,
    local_client: LocalCtrngClient,
}

impl MixedCtrngClient {
    pub fn new(
        gateway_base: impl Into<String>,
        beacon_key: impl Into<String>,
    ) -> Result<Self, CtrngError> {
        let ipfs_client = IpfsCtrngClient::new(gateway_base, beacon_key);
        let local_client = LocalCtrngClient::new()?;
        Ok(Self {
            ipfs_client,
            local_client,
        })
    }
}

impl RandomBlockSource for MixedCtrngClient {
    fn next_block(&mut self) -> Result<[u8; 32], CtrngError> {
        let ipfs_block = self.ipfs_client.next_block()?;
        let local_block = self.local_client.next_block()?;
        
        let mut mixed_block = [0u8; 32];
        for i in 0..32 {
            mixed_block[i] = ipfs_block[i] ^ local_block[i];
        }
        Ok(mixed_block)
    }
}


