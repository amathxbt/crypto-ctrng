#[cfg(feature = "testu01")]
use crypto_ctrng::{Ctrng, RandomBlockSource};
#[cfg(feature = "testu01")]
use rand_chacha::ChaCha20Rng;
#[cfg(feature = "testu01")]
use rand_core::SeedableRng;
#[cfg(feature = "testu01")]
use testu01_runner::bbattery_SmallCrush;
#[cfg(feature = "testu01")]
use testu01_runner::{delete_unif01_gen, make_unif01_gen, register_rng};

#[cfg(feature = "testu01")]
fn build_ipfs_seeded_rng() -> ChaCha20Rng {
    let gateway = "https://ipfs.filebase.io";
    let beacon_key = "k2k4r8lvomw737sajfnpav0dpeernugnryng50uheyk1k39lursmn09f";
    let mut ctrng = Ctrng::ipfs(beacon_key, None);
    let seed = ctrng.next_block().expect("failed to fetch IPFS block");
    ChaCha20Rng::from_seed(seed)
}

#[test]
#[cfg(feature = "testu01")]
fn test_ctrng_rng_smallcrush() {
    let rng = build_ipfs_seeded_rng();
    register_rng(rng);

    unsafe {
        let generator = make_unif01_gen("crypto-ctrng-deterministic");

        println!("=== SmallCrush ===");
        bbattery_SmallCrush(generator);

        delete_unif01_gen(generator);
    }
}

// Crush and BigCrush tests removed - TestU01 only allows one external generator at a time
