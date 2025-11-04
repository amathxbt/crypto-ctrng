use crate::error::CtrngError;

pub trait RandomBlockSource {
    fn next_block(&mut self) -> Result<[u8; 32], CtrngError>;
}
