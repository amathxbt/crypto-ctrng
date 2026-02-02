use crate::error::SourceError;

/// Source of 32-byte random blocks.
pub trait RandomBlockSource {
    fn next_block(&mut self) -> Result<[u8; 32], SourceError>;
}
