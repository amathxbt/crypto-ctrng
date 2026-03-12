pub(crate) mod ipfs_beacon;
pub mod types;

use crate::error::SourceError;
use crate::traits::RandomBlockSource;
use ipfs_beacon::{IpfsConfig, IpfsBeacon};
use types::CtrngBlock;

/// Unified cTRNG abstraction supporting multiple backends.
#[derive(Debug)]
pub enum Ctrng {
    Ipfs(IpfsBeacon),
    // Future: DataHeaven(DataHeavenCtrng),
}

impl Ctrng {
    /// Create an IPFS cTRNG source.
    ///
    /// Pass `None` for default config (default gateways, 10s timeout).
    pub fn ipfs(beacon_key: impl Into<String>, config: Option<IpfsConfig>) -> Self {
        Self::Ipfs(IpfsBeacon::new(beacon_key, config))
    }

    /// Returns the last block served, if any.
    pub fn last_served_block(&self) -> Option<CtrngBlock> {
        match self {
            Self::Ipfs(ipfs) => ipfs.last_served_block(),
        }
    }
}

impl RandomBlockSource for Ctrng {
    fn next_block(&mut self) -> Result<[u8; 32], SourceError> {
        match self {
            Self::Ipfs(ipfs) => ipfs.next_block(),
        }
    }
}
