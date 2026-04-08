#[cfg(feature = "testu01")]
use crypto_ctrng::{Ctrng, RandomBlockSource};
#[cfg(feature = "testu01")]
use rand_chacha::ChaCha20Rng;
#[cfg(feature = "testu01")]
use rand_core::SeedableRng;
#[cfg(feature = "testu01")]
use rng_statistical_tests::testu01::{
    bbattery_SmallCrush, delete_unif01_gen, make_unif01_gen, register_rng,
};

#[cfg(feature = "testu01")]
fn build_ipfs_seeded_rng() -> ChaCha20Rng {
    let beacon_key = "k2k4r8lvomw737sajfnpav0dpeernugnryng50uheyk1k39lursmn09f";
    let mut ctrng = Ctrng::ipfs(beacon_key, None);
    let seed = ctrng.next_block().expect("failed to fetch IPFS block");
    ChaCha20Rng::from_seed(seed)
}

#[test]
#[cfg(feature = "testu01")]
fn testu01_smallcrush() {
    let rng = build_ipfs_seeded_rng();
    register_rng(rng);

    unsafe {
        let generator = make_unif01_gen("crypto-ctrng-deterministic");
        println!("=== SmallCrush ===");
        bbattery_SmallCrush(generator);
        delete_unif01_gen(generator);
    }
}

#[test]
#[cfg(feature = "practrand")]
fn practrand_ctrng_seeded() {
    use crypto_ctrng::{Ctrng, RandomBlockSource};
    use rand_chacha::ChaCha20Rng;
    use rand_core::SeedableRng;
    use rng_statistical_tests::practrand;

    let beacon_key = "k2k4r8lvomw737sajfnpav0dpeernugnryng50uheyk1k39lursmn09f";
    let mut ctrng = Ctrng::ipfs(beacon_key, None);
    let seed = ctrng.next_block().expect("failed to fetch IPFS block");
    let rng = ChaCha20Rng::from_seed(seed);

    let result = practrand::run_test(
        rng,
        practrand::Config {
            test_size_kb: Some(1024),
            ..Default::default()
        },
    )
    .expect("failed to run PractRand");

    assert!(result.passed, "PractRand reported failure");
}
