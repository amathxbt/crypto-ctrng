use crate::error::SourceError;
use crate::traits::RandomBlockSource;

use super::types::{BeaconResponse, CtrngBlock};

/// cTRNG backed by an IPFS beacon.
#[derive(Debug)]
pub struct IpfsCtrng {
    gateway_url: String,
    beacon_key: String,
    cache: Vec<CtrngBlock>,
    cursor: usize,
    last_served: Option<CtrngBlock>,
}

impl IpfsCtrng {
    pub fn new(gateway_url: impl Into<String>, beacon_key: impl Into<String>) -> Self {
        Self {
            gateway_url: gateway_url.into(),
            beacon_key: beacon_key.into(),
            cache: Vec::new(),
            cursor: 0,
            last_served: None,
        }
    }

    pub fn last_served_block(&self) -> Option<CtrngBlock> {
        self.last_served
    }

    fn refill_cache(&mut self) -> Result<(), SourceError> {
        let url = format!(
            "{}/ipns/{}",
            self.gateway_url.trim_end_matches('/'),
            self.beacon_key
        );
        let resp = reqwest::blocking::get(&url)
            .map_err(|e| SourceError::ctrng(format!("ipfs fetch failed: {e}")))?;
        if !resp.status().is_success() {
            return Err(SourceError::ctrng(format!(
                "ipfs returned HTTP {}",
                resp.status()
            )));
        }
        let bytes = resp
            .bytes()
            .map_err(|e| SourceError::ctrng(format!("ipfs body read failed: {e}")))?;
        let parsed: BeaconResponse = serde_json::from_slice(&bytes)
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

impl RandomBlockSource for IpfsCtrng {
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
