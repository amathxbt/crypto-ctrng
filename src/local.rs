use crate::error::SourceError;
use crate::traits::RandomBlockSource;
use getrandom::getrandom;

/// OS-backed RNG using getrandom
#[derive(Debug)]
pub struct LocalRng;

impl LocalRng {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LocalRng {
    fn default() -> Self {
        Self::new()
    }
}

impl RandomBlockSource for LocalRng {
    fn next_block(&mut self) -> Result<[u8; 32], SourceError> {
        let mut block = [0u8; 32];
        getrandom(&mut block)
            .map_err(|e| SourceError::os(format!("failed to get OS randomness: {e}")))?;
        Ok(block)
    }
}
