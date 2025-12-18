use crate::error::CtrngError;
use crate::traits::RandomBlockSource;
use getrandom::getrandom;

#[derive(Debug)]
pub struct LocalCtrngClient;

impl LocalCtrngClient {
    pub fn new() -> Result<Self, CtrngError> {
        Ok(Self)
    }
}

impl RandomBlockSource for LocalCtrngClient {
    fn next_block(&mut self) -> Result<[u8; 32], CtrngError> {
        let mut block = [0u8; 32];
        getrandom(&mut block)
            .map_err(|e| CtrngError::backend(format!("failed to get OS randomness: {e}")))?;
        Ok(block)
    }
}

