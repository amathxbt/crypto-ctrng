use crypto_ctrng::{CtrngRng, CtrngError, RandomBlockSource, derive_seed, rng_from_seed_block};
use crypto_ctrng::ctrng::mock::MockCtrngClient;
use rand_core::RngCore;
use std::collections::HashSet;

#[test]
    fn mock_ctrng_emits_unique_blocks() {
        let mut client = MockCtrngClient::from_seed([0x42; 32]);
        let first = client.next_block().unwrap();
        let second = client.next_block().unwrap();
        assert_ne!(first, second);
    }

    #[test]
    fn ctrng_rng_does_not_repeat_blocks_under_mock() {
        let mut rng = CtrngRng::new(MockCtrngClient::from_seed([0x33; 32]))
            .expect("mock should yield first block");
        let mut seen = HashSet::new();
        for idx in 0..128 {
            let mut block = [0u8; 32];
            rng.try_fill_bytes_fallible(&mut block)
                .expect("mock should not fail");
            assert!(seen.insert(block), "duplicate block observed at draw {idx}");
        }
    }

    #[test]
    fn flaky_source_propagates_error() {
        struct Flaky {
            ok_block: [u8; 32],
            fail_every: usize,
            calls: usize,
        }

        impl RandomBlockSource for Flaky {
            fn next_block(&mut self) -> Result<[u8; 32], CtrngError> {
                self.calls += 1;
                if self.calls % self.fail_every == 0 {
                    Err(CtrngError::backend(format!(
                        "backend failure after {} calls",
                        self.calls
                    )))
                } else {
                    Ok(self.ok_block)
                }
            }
        }

        let mut rng = CtrngRng::new(Flaky {
            ok_block: [0x11; 32],
            fail_every: 3,
            calls: 0,
        })
        .expect("first block should succeed");

        // first fill works
        let mut buf = [0u8; 64];
        rng.try_fill_bytes_fallible(&mut buf)
            .expect("second block ok");

        // second fill triggers failure
        let mut buf2 = [0u8; 64];
        let err = rng
            .try_fill_bytes_fallible(&mut buf2)
            .expect_err("third call must error");
        assert!(matches!(err, CtrngError::Backend(_)));

        // rand_core::try_fill_bytes should surface the same error code on a failing backend.
        let mut rng_trait = CtrngRng::new(Flaky {
            ok_block: [0x22; 32],
            fail_every: 2,
            calls: 0,
        })
        .expect("first block should succeed");
        let mut buf3 = [0u8; 64];
        assert!(rng_trait.try_fill_bytes(&mut buf3).is_err());
    }

    #[test]
    fn derive_seed_yields_unique_material() {
        let exec = b"derive-seed-exec-id";
        let trng = [0xAA; 32];
        let mut seen = HashSet::new();
        for ctr in 0..256u64 {
            let seed = derive_seed(exec, 1, ctr, &trng);
            assert!(
                seen.insert(seed),
                "duplicate seed for counter {ctr}"
            );
        }
    }

    #[test]
    fn rng_from_seed_block_is_deterministic_for_same_seed() {
        let seed = [0x11; 32];

        let mut rng1 = rng_from_seed_block(seed);
        let mut rng2 = rng_from_seed_block(seed);

        let mut buf1 = [0u8; 64];
        let mut buf2 = [0u8; 64];

        rng1.try_fill_bytes_fallible(&mut buf1).unwrap();
        rng2.try_fill_bytes_fallible(&mut buf2).unwrap();

        assert_eq!(buf1, buf2, "same seed should give identical output");
    }

    #[test]
    fn rng_from_seed_block_differs_for_different_seeds() {
        let mut rng_a = rng_from_seed_block([0x11; 32]);
        let mut rng_b = rng_from_seed_block([0x22; 32]);

        let mut buf_a = [0u8; 64];
        let mut buf_b = [0u8; 64];

        rng_a.try_fill_bytes_fallible(&mut buf_a).unwrap();
        rng_b.try_fill_bytes_fallible(&mut buf_b).unwrap();

        assert_ne!(buf_a, buf_b, "different seeds should give different output");
    }

