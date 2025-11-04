#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CtrngError {
    #[error("cTRNG backend failure: {0}")]
    Backend(String),
}

impl CtrngError {
    pub fn backend(msg: impl Into<String>) -> Self { 
        Self::Backend(msg.into()) 
    }
}

impl From<CtrngError> for rand_core::Error {
    fn from(err: CtrngError) -> Self { 
        rand_core::Error::new(err.to_string()) 
    }
}
