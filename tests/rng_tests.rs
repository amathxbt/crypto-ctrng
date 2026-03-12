use crypto_ctrng::{BlockRng, RandomBlockSource, SourceError};
use rand_core::RngCore;

#[test]
fn flaky_source_propagates_error() {
    struct Flaky {
        ok_block: [u8; 32],
        fail_every: usize,
        calls: usize,
    }

    impl RandomBlockSource for Flaky {
        fn next_block(&mut self) -> Result<[u8; 32], SourceError> {
            self.calls += 1;
            if self.calls % self.fail_every == 0 {
                Err(SourceError::ctrng(format!(
                    "backend failure after {} calls",
                    self.calls
                )))
            } else {
                Ok(self.ok_block)
            }
        }
    }

    let mut rng = BlockRng::new(Flaky {
        ok_block: [0x11; 32],
        fail_every: 3,
        calls: 0,
    })
    .expect("first block should succeed");

    // first fill works
    let mut buf = [0u8; 64];
    rng.try_fill_bytes_source(&mut buf)
        .expect("second block ok");

    // second fill triggers failure
    let mut buf2 = [0u8; 64];
    let err = rng
        .try_fill_bytes_source(&mut buf2)
        .expect_err("third call must error");
    assert!(matches!(err, SourceError::Ctrng(_)));

    // rand_core::try_fill_bytes should surface the same error code on a failing backend.
    let mut rng_trait = BlockRng::new(Flaky {
        ok_block: [0x22; 32],
        fail_every: 2,
        calls: 0,
    })
    .expect("first block should succeed");
    let mut buf3 = [0u8; 64];
    assert!(rng_trait.try_fill_bytes(&mut buf3).is_err());
}
