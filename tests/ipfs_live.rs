use crypto_ctrng::RandomBlockSource;

#[test]
#[ignore]
fn ipfs_client_can_fetch_live_block() {
    let mut client = crypto_ctrng::IpfsCtrngClient::new(
        "https://ipfs.io",
        "k2k4r8lvomw737sajfnpav0dpeernugnryng50uheyk1k39lursmn09f",
    );
    let block = client.next_block().expect("fetch 32-byte block");
    assert_eq!(block.len(), 32);
}
