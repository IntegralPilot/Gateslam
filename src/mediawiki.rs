use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub enum IPDataType {
    #[serde(rename = "confirmed-vpngate-egress")]
    ConfimedVpngateEgress,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct IPDataEntry {
    pub ip: String,
    #[serde(rename = "type")]
    pub type_: IPDataType,
    #[serde(rename = "last-sighting")]
    pub last_sighting: u64,
    pub sightings: u32,
}