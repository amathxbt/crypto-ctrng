use crypto_ctrng::RandomBlockSource;

const BEACON_KEY: &str = "k2k4r8lvomw737sajfnpav0dpeernugnryng50uheyk1k39lursmn09f";

#[test]
fn test_e2e_mixed_is_32_bytes() {
    let ipfs = crypto_ctrng::Ctrng::ipfs(BEACON_KEY, None);
    let mut client = crypto_ctrng::MixedCtrng::new(ipfs).unwrap();
    let block = client.next_block().unwrap();
    assert_eq!(block.len(), 32);
}

#[test]
fn test_e2e_each_source_is_different() {
    let mut ipfs = crypto_ctrng::Ctrng::ipfs(BEACON_KEY, None);
    let ipfs2 = crypto_ctrng::Ctrng::ipfs(BEACON_KEY, None);
    let mut mixed = crypto_ctrng::MixedCtrng::new(ipfs2).unwrap();

    let ipfs_block = ipfs.next_block().unwrap();
    let mixed_block = mixed.next_block().unwrap();

    assert_ne!(ipfs_block, mixed_block);
}

#[test]
fn test_e2e_mixed_unique_blocks() {
    let ipfs = crypto_ctrng::Ctrng::ipfs(BEACON_KEY, None);
    let mut client = crypto_ctrng::MixedCtrng::new(ipfs).unwrap();
    let a = client.next_block().unwrap();
    let b = client.next_block().unwrap();
    assert_ne!(a, b);
}
