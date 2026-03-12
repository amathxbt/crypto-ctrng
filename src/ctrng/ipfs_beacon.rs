use std::time::Duration;

use crate::error::SourceError;
use crate::traits::RandomBlockSource;

use super::types::{BeaconResponse, CtrngBlock};

pub const DEFAULT_GATEWAYS: &[&str] = &[
    "https://ipfs.io",
    "https://ipfs.filebase.io",
    "https://dweb.link",
];

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// Configuration for IPFS-backed cTRNG.
#[derive(Debug, Clone)]
pub struct IpfsConfig {
    pub gateways: Vec<String>,
    pub use_defaults: bool,
    pub timeout: Duration,
}

impl Default for IpfsConfig {
    fn default() -> Self {
        Self {
            gateways: Vec::new(),
            use_defaults: true,
            timeout: DEFAULT_TIMEOUT,
        }
    }
}

#[derive(Debug, Clone)]
pub struct IpfsGateway(String);

impl IpfsGateway {
    pub fn new(url: impl Into<String>) -> Self {
        Self(url.into().trim_end_matches('/').to_string())
    }

    pub fn ipfs(&self, key: &str) -> String {
        format!("{}/ipfs/{}", self.0, key)
    }

    pub fn ipns(&self, key: &str) -> String {
        format!("{}/ipns/{}", self.0, key)
    }
}

impl std::fmt::Display for IpfsGateway {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PartialEq for IpfsGateway {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

/// cTRNG backed by an IPFS beacon.
#[derive(Debug)]
pub struct IpfsBeacon {
    gateways: Vec<IpfsGateway>,
    beacon_key: String,
    timeout: Duration,
    cache: Vec<CtrngBlock>,
    cursor: usize,
    last_served: Option<CtrngBlock>,
}

impl IpfsBeacon {
    pub fn new(beacon_key: impl Into<String>, config: Option<IpfsConfig>) -> Self {
        let config = config.unwrap_or_default();
        let mut gw_list: Vec<IpfsGateway> =
            config.gateways.into_iter().map(IpfsGateway::new).collect();
        if config.use_defaults {
            for default in DEFAULT_GATEWAYS {
                let gw = IpfsGateway::new(*default);
                if !gw_list.contains(&gw) {
                    gw_list.push(gw);
                }
            }
        }
        Self {
            gateways: gw_list,
            beacon_key: beacon_key.into(),
            timeout: config.timeout,
            cache: Vec::new(),
            cursor: 0,
            last_served: None,
        }
    }

    pub fn last_served_block(&self) -> Option<CtrngBlock> {
        self.last_served
    }

    fn try_gateway(&self, gateway: &IpfsGateway) -> Result<Vec<u8>, String> {
        let url = gateway.ipns(&self.beacon_key);

        let client = reqwest::blocking::Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|e| format!("{gateway}: client build failed: {e}"))?;

        let resp = client
            .get(&url)
            .send()
            .map_err(|e| format!("{gateway}: fetch failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("{gateway}: HTTP {}", resp.status()));
        }

        let bytes = resp
            .bytes()
            .map_err(|e| format!("{gateway}: body read failed: {e}"))?;

        Ok(bytes.to_vec())
    }

    fn refill_cache(&mut self) -> Result<(), SourceError> {
        let mut errors = Vec::new();

        for gateway in &self.gateways {
            match self.try_gateway(gateway) {
                Ok(bytes) => {
                    return self.parse_and_store(&bytes);
                }
                Err(e) => {
                    errors.push(e);
                }
            }
        }

        Err(SourceError::ctrng(format!(
            "all gateways failed: [{}]",
            errors.join(", ")
        )))
    }

    fn parse_and_store(&mut self, bytes: &[u8]) -> Result<(), SourceError> {
        let parsed: BeaconResponse = serde_json::from_slice(bytes)
            .map_err(|e| SourceError::ctrng(format!("ipfs json parse failed: {e}")))?;

        let mut new_cache = Vec::with_capacity(parsed.data.ctrng.len());
        let sequence = parsed.data.sequence;
        let timestamp = parsed.data.timestamp;
        for hex_str in parsed.data.ctrng.iter() {
            let raw = hex::decode(hex_str)
                .map_err(|e| SourceError::ctrng(format!("invalid hex in ctrng entry: {e}")))?;
            if raw.len() != 32 {
                return Err(SourceError::ctrng(format!(
                    "ctrng entry length is {}, expected 32",
                    raw.len()
                )));
            }
            let mut block = [0u8; 32];
            block.copy_from_slice(&raw);
            new_cache.push(CtrngBlock {
                sequence,
                timestamp,
                data: block,
            });
        }
        if new_cache.is_empty() {
            return Err(SourceError::ctrng("ipfs beacon returned empty ctrng list"));
        }
        if let (Some(last), Some(first)) = (self.last_served, new_cache.first().copied()) {
            if first.timestamp <= last.timestamp {
                return Err(SourceError::ctrng(format!(
                    "ipfs beacon timestamp {} is not greater than last served {}",
                    first.timestamp, last.timestamp
                )));
            }
        }
        self.cache = new_cache;
        self.cursor = 0;
        Ok(())
    }
}

impl RandomBlockSource for IpfsBeacon {
    fn next_block(&mut self) -> Result<[u8; 32], SourceError> {
        if self.cursor < self.cache.len() {
            let block = self.cache[self.cursor];
            self.cursor += 1;

            if let Some(last) = self.last_served {
                if block.data == last.data {
                    return Err(SourceError::ctrng(format!(
                        "ipfs beacon repeated randomness for timestamp {}",
                        block.timestamp
                    )));
                }
            }

            self.last_served = Some(block);
            Ok(block.data)
        } else {
            self.refill_cache()?;
            self.next_block()
        }
    }
}
