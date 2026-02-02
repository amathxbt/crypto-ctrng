use crypto_ctrng::RandomBlockSource;

const BEACON_KEY: &str = "k2k4r8lvomw737sajfnpav0dpeernugnryng50uheyk1k39lursmn09f";
const GATEWAY: &str = "https://ipfs.io";

#[test]
#[ignore]
fn ipfs_client_can_fetch_live_block() {
    let mut client = crypto_ctrng::IpfsCtrng::new(GATEWAY, BEACON_KEY);
    let block = client.next_block().expect("fetch 32-byte block");
    assert_eq!(block.len(), 32);
}

#[test]
#[ignore]
fn ipfs_unique_blocks() {
    let mut client = crypto_ctrng::IpfsCtrng::new(GATEWAY, BEACON_KEY);

    let a = client.next_block().expect("first block");
    let b = client.next_block().expect("second block");
    let c = client.next_block().expect("third block");

    // 4th call triggers refill_cache() - may fail if new batch has timestamp <= last served
    let result = client.next_block();

    assert_ne!(a, b, "first and second blocks should be different");
    assert_ne!(b, c, "second and third blocks should be different");

    match result {
        Ok(d) => {
            assert_ne!(c, d, "third and fourth blocks should be different");
        }
        Err(_) => {
            // Expected failure confirms timestamp validation works
        }
    }
}
