use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum VersionType {
    Release,
    Snapshot,
    OldBeta,
    OldAlpha,
}

#[derive(Deserialize, Serialize)]
pub struct LatestVersions {
    pub release: String,
    pub snapshot: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FullVersionInfo {
    pub id: String,
    pub r#type: VersionType,
    pub url: String,
    pub time: String,
    pub release_time: String,
    pub sha1: String,
    pub compliance_level: usize,
}

#[derive(Deserialize, Serialize)]
pub struct VersionInfo {
    pub r#type: VersionType,
    pub url: String,
    pub time: String,
    pub release_time: String,
    pub sha1: String,
    pub compliance_level: usize,
}

#[derive(Deserialize, Serialize)]
pub struct VersionList {
    pub latest: LatestVersions,
    pub versions: Vec<FullVersionInfo>,
}