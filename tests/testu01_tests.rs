#[cfg(feature = "testu01")]
use crypto_ctrng::{CtrngRng, IpfsCtrngClient, MockCtrngClient, RandomBlockSource};
#[cfg(feature = "testu01")]
use testu01_runner::bbattery_SmallCrush;
#[cfg(feature = "testu01")]
use testu01_runner::{delete_unif01_gen, make_unif01_gen, register_rng};

#[cfg(feature = "testu01")]
fn build_ipfs_rng() -> CtrngRng<MockCtrngClient> {
    let gateway = "https://ipfs.io";
    let beacon_key = "k2k4r8lvomw737sajfnpav0dpeernugnryng50uheyk1k39lursmn09f";
    let mut ctrng = IpfsCtrngClient::new(gateway, beacon_key);
    let seed = ctrng.next_block().expect("failed to fetch IPFS block");
    crypto_ctrng::rng_from_seed_block(seed)
}

#[test]
#[cfg(feature = "testu01")]
fn test_ctrng_rng_smallcrush() {
    let rng = build_ipfs_rng();
    register_rng(rng);

    unsafe {
        let generator = make_unif01_gen("crypto-ctrng-mock");

        println!("=== SmallCrush ===");
        bbattery_SmallCrush(generator);

        delete_unif01_gen(generator);
    }
}

// Crush and BigCrush tests removed - TestU01 only allows one external generator at a time
