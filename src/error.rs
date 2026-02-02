/// Error type for entropy source operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SourceError {
    /// Error from the cosmic TRNG (IPFS beacon).
    #[error("cTRNG error: {0}")]
    Ctrng(String),

    /// Error from the OS randomness source.
    #[error("OS RNG error: {0}")]
    Os(String),
}

impl SourceError {
    pub fn ctrng(msg: impl Into<String>) -> Self {
        Self::Ctrng(msg.into())
    }

    pub fn os(msg: impl Into<String>) -> Self {
        Self::Os(msg.into())
    }
}

impl From<SourceError> for rand_core::Error {
    fn from(err: SourceError) -> Self {
        rand_core::Error::new(err.to_string())
    }
}
