use crate::error::CtrngError;
use crate::traits::RandomBlockSource;
use crate::types::{BeaconResponse, CtrngBlock};

#[derive(Debug)]
pub struct IpfsCtrngClient {
    gateway_base: String,
    beacon_key: String,
    cache: Vec<CtrngBlock>,
    cursor: usize,
    last_served: Option<CtrngBlock>,
}

impl IpfsCtrngClient {
    pub fn new(gateway_base: impl Into<String>, beacon_key: impl Into<String>) -> Self {
        Self {
            gateway_base: gateway_base.into(),
            beacon_key: beacon_key.into(),
            cache: Vec::new(),
            cursor: 0,
            last_served: None,
        }
    }

    pub fn last_served_block(&self) -> Option<CtrngBlock> {
        self.last_served
    }

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

        let mut new_cache = Vec::with_capacity(parsed.data.ctrng.len());
        let sequence = parsed.data.sequence;
        let timestamp = parsed.data.timestamp;
        for hex_str in parsed.data.ctrng.iter() {
            let raw = hex::decode(hex_str)
                .map_err(|e| CtrngError::backend(format!("invalid hex in ctrng entry: {e}")))?;
            if raw.len() != 32 {
                return Err(CtrngError::backend(format!(
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
            return Err(CtrngError::backend("ipfs beacon returned empty ctrng list"));
        }
        if let (Some(last), Some(first)) = (self.last_served, new_cache.first().copied()) {
            if first.timestamp <= last.timestamp {
                return Err(CtrngError::backend(format!(
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

impl RandomBlockSource for IpfsCtrngClient {
    fn next_block(&mut self) -> Result<[u8; 32], CtrngError> {
        if self.cursor < self.cache.len() {
            let block = self.cache[self.cursor];
            self.cursor += 1;

            if let Some(last) = self.last_served {
                if block.data == last.data {
                    return Err(CtrngError::backend(format!(
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
