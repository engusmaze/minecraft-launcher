use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Debug)]
pub struct Asset {
    pub hash: String,
    pub size: usize,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct AssetList {
    pub objects: HashMap<String, Asset>,
}
