use crate::error::CtrngError;
use crate::traits::RandomBlockSource;
use crate::types::BeaconResponse;

#[derive(Debug)]
pub struct IpfsCtrngClient {
    gateway_base: String,
    beacon_key: String,
    cache: Vec<[u8; 32]>,
    cursor: usize,
}

impl IpfsCtrngClient {
    pub fn new(gateway_base: impl Into<String>, beacon_key: impl Into<String>) -> Self {
        Self { 
            gateway_base: gateway_base.into(), 
            beacon_key: beacon_key.into(), 
            cache: Vec::new(), 
            cursor: 0 
        }
    }

    fn refill_cache(&mut self) -> Result<(), CtrngError> {
        let url = format!("{}/ipns/{}", self.gateway_base.trim_end_matches('/'), self.beacon_key);
        let resp = reqwest::blocking::get(&url).map_err(|e| CtrngError::backend(format!("ipfs fetch failed: {e}")))?;
        if !resp.status().is_success() {
            return Err(CtrngError::backend(format!("ipfs returned HTTP {}", resp.status())));
        }
        let bytes = resp.bytes().map_err(|e| CtrngError::backend(format!("ipfs body read failed: {e}")))?;
        let parsed: BeaconResponse = serde_json::from_slice(&bytes)
            .map_err(|e| CtrngError::backend(format!("ipfs json parse failed: {e}")))?;

        let mut new_cache = Vec::with_capacity(parsed.data.ctrng.len());
        for hex_str in parsed.data.ctrng.iter() {
            let raw = hex::decode(hex_str).map_err(|e| CtrngError::backend(format!("invalid hex in ctrng entry: {e}")))?;
            if raw.len() != 32 {
                return Err(CtrngError::backend(format!("ctrng entry length is {}, expected 32", raw.len())));
            }
            let mut block = [0u8; 32];
            block.copy_from_slice(&raw);
            new_cache.push(block);
        }
        if new_cache.is_empty() {
            return Err(CtrngError::backend("ipfs beacon returned empty ctrng list"));
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
            return Ok(block);
        }
        self.refill_cache()?;
        let block = self.cache[self.cursor];
        self.cursor += 1;
        Ok(block)
    }
}
