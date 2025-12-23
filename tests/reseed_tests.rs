use std::time::Duration;

use crypto_ctrng::{
    CtrngError, LocalCtrngClient, MockCtrngClient, RandomBlockSource, ReseedConfig, ReseedingRng,
    DEFAULT_RESEED_BYTES, DEFAULT_RESEED_TIME,
};
use rand_core::RngCore;

struct TestSource {
    call_count: u32,
    fail_on_call: Option<u32>,
}

impl TestSource {
    fn new() -> Self {
        Self {
            call_count: 0,
            fail_on_call: None,
        }
    }

    fn failing_on(call: u32) -> Self {
        Self {
            call_count: 0,
            fail_on_call: Some(call),
        }
    }
}

impl RandomBlockSource for TestSource {
    fn next_block(&mut self) -> Result<[u8; 32], CtrngError> {
        self.call_count += 1;
        if self.fail_on_call == Some(self.call_count) {
            return Err(CtrngError::backend("simulated failure"));
        }
        let mut block = [0u8; 32];
        block[0..4].copy_from_slice(&self.call_count.to_le_bytes());
        Ok(block)
    }
}

#[test]
fn default_constants() {
    assert_eq!(DEFAULT_RESEED_BYTES, 3200);
    assert_eq!(DEFAULT_RESEED_TIME, Duration::from_secs(7 * 24 * 60 * 60));
}

#[test]
fn with_local_source() {
    let local = LocalCtrngClient::new().unwrap();
    let mut rng = ReseedingRng::new(local).unwrap();

    let mut buf1 = [0u8; 32];
    let mut buf2 = [0u8; 32];
    rng.fill_bytes(&mut buf1);
    rng.fill_bytes(&mut buf2);

    assert_ne!(buf1, buf2);
}

#[test]
fn with_mock_source() {
    let mock = MockCtrngClient::from_seed([42u8; 32]);
    let mut rng = ReseedingRng::new(mock).unwrap();

    let mut buf = [0u8; 64];
    rng.fill_bytes(&mut buf);

    assert!(buf.iter().any(|&b| b != 0));
}

#[test]
fn reseed_interval_bytes_triggers_reseed() {
    let source = TestSource::new();
    let config = ReseedConfig::new(64, Duration::from_secs(3600));
    let mut rng = ReseedingRng::with_config(source, config).unwrap();

    assert_eq!(rng.source_mut().call_count, 1);

    for _ in 0..8 {
        let mut buf = [0u8; 8];
        rng.fill_bytes(&mut buf);
    }
    assert_eq!(rng.bytes_since_reseed(), 64);
    assert_eq!(rng.source_mut().call_count, 1);

    let mut buf = [0u8; 1];
    rng.fill_bytes(&mut buf);
    assert_eq!(rng.source_mut().call_count, 2);
    assert_eq!(rng.bytes_since_reseed(), 1);
}

#[test]
fn prediction_resistance_reseeds_every_call() {
    let source = TestSource::new();
    let mut rng = ReseedingRng::with_prediction_resistance(source).unwrap();

    assert_eq!(rng.source_mut().call_count, 1);

    for i in 2..=10 {
        let mut buf = [0u8; 1];
        rng.fill_bytes(&mut buf);
        assert_eq!(rng.source_mut().call_count, i);
    }
}

#[test]
fn manual_reseed() {
    let source = TestSource::new();
    let mut rng = ReseedingRng::new(source).unwrap();

    assert_eq!(rng.source_mut().call_count, 1);

    let mut buf = [0u8; 100];
    rng.fill_bytes(&mut buf);
    assert_eq!(rng.bytes_since_reseed(), 100);

    let mut before = [0u8; 32];
    rng.fill_bytes(&mut before);

    rng.reseed().unwrap();
    assert_eq!(rng.source_mut().call_count, 2);
    assert_eq!(rng.bytes_since_reseed(), 0);

    let mut after = [0u8; 32];
    rng.fill_bytes(&mut after);
    assert_eq!(rng.bytes_since_reseed(), 32);
}

#[test]
fn try_fill_bytes_propagates_errors() {
    let source = TestSource::failing_on(2);
    let config = ReseedConfig::new(32, Duration::from_secs(3600));
    let mut rng = ReseedingRng::with_config(source, config).unwrap();

    let mut buf = [0u8; 32];
    rng.fill_bytes(&mut buf);

    let result = rng.try_fill_bytes_fallible(&mut buf);
    assert!(result.is_err());
}

#[test]
fn config_accessors() {
    let source = TestSource::new();
    let config = ReseedConfig::new(1000, Duration::from_secs(600));
    let rng = ReseedingRng::with_config(source, config).unwrap();

    assert_eq!(rng.config().reseed_interval_bytes, 1000);
    assert_eq!(rng.config().reseed_interval_time, Duration::from_secs(600));
}

#[test]
fn time_since_reseed_increases() {
    let source = TestSource::new();
    let mut rng = ReseedingRng::new(source).unwrap();

    let t1 = rng.time_since_reseed();
    std::thread::sleep(Duration::from_millis(10));
    let t2 = rng.time_since_reseed();

    assert!(t2 > t1);

    rng.reseed().unwrap();
    let t3 = rng.time_since_reseed();
    assert!(t3 < t2);
}

#[test]
fn large_fill() {
    let source = TestSource::new();
    let config = ReseedConfig::new(100, Duration::from_secs(3600));
    let mut rng = ReseedingRng::with_config(source, config).unwrap();

    assert_eq!(rng.source_mut().call_count, 1);

    let mut buf = [0u8; 256];
    rng.fill_bytes(&mut buf);

    assert_eq!(rng.bytes_since_reseed(), 256);
    assert_eq!(rng.source_mut().call_count, 1);

    assert!(rng.needs_reseed());
    let mut small = [0u8; 1];
    rng.fill_bytes(&mut small);
    assert_eq!(rng.source_mut().call_count, 2);
}

#[test]
fn next_u32_and_u64() {
    let source = TestSource::new();
    let mut rng = ReseedingRng::new(source).unwrap();

    let v1 = rng.next_u32();
    let v2 = rng.next_u32();
    let v3 = rng.next_u64();
    let v4 = rng.next_u64();

    assert!(v1 != 0 || v2 != 0);
    assert!(v3 != 0 || v4 != 0);
}
