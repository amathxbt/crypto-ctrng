use sha2::{Digest, Sha256};

/// Derives a personalized seed using SHA-256 domain separation.
pub fn derive_seed(
    execution_id: &[u8],
    party_id: u16,
    counter: u64,
    ctrng_block: &[u8; 32],
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(execution_id);
    hasher.update(party_id.to_be_bytes());
    hasher.update(counter.to_be_bytes());
    hasher.update(ctrng_block);
    hasher.finalize().into()
}
