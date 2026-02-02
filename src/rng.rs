use rand_core::{CryptoRng, Error, RngCore, impls};
use zeroize::Zeroize;

use crate::error::SourceError;
use crate::traits::RandomBlockSource;

/// RNG that buffers 32-byte blocks from a RandomBlockSource.
#[derive(Debug)]
pub struct BlockRng<C> {
    source: C,
    buffer: [u8; 32],
    cursor: usize,
}

impl<C: RandomBlockSource> BlockRng<C> {
    pub fn new(mut source: C) -> Result<Self, SourceError> {
        let buffer = source.next_block()?;
        Ok(Self {
            source,
            buffer,
            cursor: 0,
        })
    }

    pub fn source_mut(&mut self) -> &mut C {
        &mut self.source
    }

    fn refill(&mut self) -> Result<(), SourceError> {
        self.buffer.zeroize();
        self.buffer = self.source.next_block()?;
        self.cursor = 0;
        Ok(())
    }

    fn fill_bytes_internal(&mut self, dest: &mut [u8]) -> Result<(), SourceError> {
        let mut filled = 0;
        while filled < dest.len() {
            if self.cursor == self.buffer.len() {
                self.refill()?;
            }
            let available = self.buffer.len() - self.cursor;
            let needed = dest.len() - filled;
            let to_copy = available.min(needed);
            dest[filled..filled + to_copy]
                .copy_from_slice(&self.buffer[self.cursor..self.cursor + to_copy]);
            self.cursor += to_copy;
            filled += to_copy;
        }
        Ok(())
    }

    /// Like `try_fill_bytes` but returns `SourceError` for precise error handling.
    pub fn try_fill_bytes_source(&mut self, dest: &mut [u8]) -> Result<(), SourceError> {
        self.fill_bytes_internal(dest)
    }
}

impl<C: RandomBlockSource> RngCore for BlockRng<C> {
    fn next_u32(&mut self) -> u32 {
        impls::next_u32_via_fill(self)
    }
    fn next_u64(&mut self) -> u64 {
        impls::next_u64_via_fill(self)
    }
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        if let Err(err) = self.fill_bytes_internal(dest) {
            panic!("cTRNG error: {err:?}");
        }
    }
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
        self.fill_bytes_internal(dest).map_err(Error::from)
    }
}

impl<C: RandomBlockSource> CryptoRng for BlockRng<C> {}

impl<C> Drop for BlockRng<C> {
    fn drop(&mut self) {
        self.buffer.zeroize();
        self.cursor.zeroize();
    }
}
