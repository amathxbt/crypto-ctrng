#[derive(Debug, serde::Deserialize)]
pub struct BeaconData {
    pub sequence: u64,
    pub timestamp: u64,
    pub ctrng: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct BeaconResponse {
    pub previous: String,
    pub data: BeaconData,
}
