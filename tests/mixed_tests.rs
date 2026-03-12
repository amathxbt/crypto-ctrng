use crypto_ctrng::RandomBlockSource;

#[test]
fn local_is_32_bytes() {
    let mut client = crypto_ctrng::LocalRng::new();
    let block = client.next_block().unwrap();
    assert_eq!(block.len(), 32);
}

#[test]
fn local_unique_blocks() {
    let mut client = crypto_ctrng::LocalRng::new();
    let a = client.next_block().unwrap();
    let b = client.next_block().unwrap();
    assert_ne!(a, b);
}
