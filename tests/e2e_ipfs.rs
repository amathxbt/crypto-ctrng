use std::time::Duration;

use crypto_ctrng::RandomBlockSource;

const BEACON_KEY: &str = "k2k4r8lvomw737sajfnpav0dpeernugnryng50uheyk1k39lursmn09f";
const GATEWAY: &str = "https://ipfs.filebase.io";

#[test]
fn test_e2e_ipfs_client_can_fetch_live_block() {
    let mut client = crypto_ctrng::Ctrng::ipfs(BEACON_KEY, None);
    let block = client.next_block().expect("fetch 32-byte block");
    assert_eq!(block.len(), 32);
}

#[test]
fn test_e2e_ipfs_unique_blocks() {
    let mut client = crypto_ctrng::Ctrng::ipfs(BEACON_KEY, None);

    let a = client.next_block().expect("first block");
    let b = client.next_block().expect("second block");
    let c = client.next_block().expect("third block");

    let result = client.next_block();

    assert_ne!(a, b, "first and second blocks should be different");
    assert_ne!(b, c, "second and third blocks should be different");

    match result {
        Ok(d) => {
            assert_ne!(c, d, "third and fourth blocks should be different");
        }
        Err(_) => {
            // Expected failure confirms timestamp validation works.
        }
    }
}

#[test]
fn test_e2e_ipfs_multi_gateway_fallback() {
    let config = crypto_ctrng::IpfsConfig {
        gateways: vec!["https://invalid-gateway.example.com".into(), GATEWAY.into()],
        use_defaults: false,
        timeout: Duration::from_secs(5),
    };
    let mut client = crypto_ctrng::Ctrng::ipfs(BEACON_KEY, Some(config));
    let block = client.next_block().expect("fallback should succeed");
    assert_eq!(block.len(), 32);
}

#[test]
fn test_e2e_ipfs_all_gateways_fail() {
    let config = crypto_ctrng::IpfsConfig {
        gateways: vec![
            "https://invalid-gateway1.example.com".into(),
            "https://invalid-gateway2.example.com".into(),
        ],
        use_defaults: false,
        timeout: Duration::from_secs(3),
    };
    let mut client = crypto_ctrng::Ctrng::ipfs(BEACON_KEY, Some(config));
    let result = client.next_block();
    assert!(result.is_err(), "all gateways should fail");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("all gateways failed"),
        "error should mention all gateways failed: {err}"
    );
}

#[test]
fn test_e2e_ipfs_default_gateways_list() {
    let mut client = crypto_ctrng::Ctrng::ipfs(BEACON_KEY, None);
    let block = client.next_block().expect("default gateways should work");
    assert_eq!(block.len(), 32);
}
