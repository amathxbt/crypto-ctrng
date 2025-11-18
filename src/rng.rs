use crate::error::CtrngError;
use crate::traits::RandomBlockSource;
use rand_core::{CryptoRng, Error, RngCore, impls};
use zeroize::Zeroize;

#[derive(Debug)]
pub struct CtrngRng<C> {
    client: C,
    buffer: [u8; 32],
    cursor: usize,
}

impl<C: RandomBlockSource> CtrngRng<C> {
    pub fn new(mut client: C) -> Result<Self, CtrngError> {
        let buffer = client.next_block()?;
        Ok(Self {
            client,
            buffer,
            cursor: 0,
        })
    }

    pub fn client_mut(&mut self) -> &mut C {
        &mut self.client
    }

    fn refill(&mut self) -> Result<(), CtrngError> {
        self.buffer.zeroize();
        self.buffer = self.client.next_block()?;
        self.cursor = 0;
        Ok(())
    }

    fn fill_bytes_internal(&mut self, dest: &mut [u8]) -> Result<(), CtrngError> {
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

    pub fn try_fill_bytes_fallible(&mut self, dest: &mut [u8]) -> Result<(), CtrngError> {
        self.fill_bytes_internal(dest)
    }
}

impl<C: RandomBlockSource> RngCore for CtrngRng<C> {
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

impl<C: RandomBlockSource> CryptoRng for CtrngRng<C> {}

impl<C> Drop for CtrngRng<C> {
    fn drop(&mut self) {
        self.buffer.zeroize();
        self.cursor.zeroize();
    }
}
