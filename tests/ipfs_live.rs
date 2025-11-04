use crypto_ctrng::RandomBlockSource;

#[cfg(feature = "ipfs")]
#[test]
#[ignore]
fn ipfs_client_can_fetch_live_block() {
    let mut client = crypto_ctrng::IpfsCtrngClient::new(
        "https://ipfs.io",
        "k2k4r8pigrw8i34z63om8f015tt5igdq0c46xupq8spp1bogt35k5vhe",
    );
    let block = client.next_block().expect("fetch 32-byte block");
    assert_eq!(block.len(), 32);
}
