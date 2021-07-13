use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use types::MediaType;

pub mod digest;
pub mod types;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    schema_version: i64,
    media_type: Option<MediaType>,
    config: Descriptor,
    layers: Vec<Descriptor>,
    annotations: Option<HashMap<String, String>>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct IndexManifest {
    schema_version: i64,
    media_type: Option<MediaType>,
    manifests: Vec<Descriptor>,
    annotations: Option<HashMap<String, String>>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Descriptor {
    media_type: MediaType,
    size: i64,
    digest: digest::Hash,
    urls: Option<Vec<String>>,
    annotations: Option<HashMap<String, String>>,
    platform: Option<Platform>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Platform {
    architecture: String,
    os: String,
    #[serde(rename = "os.version")]
    os_version: Option<String>,
    #[serde(rename = "os.features")]
    os_features: Option<Vec<String>>,
    variant: Option<String>,
    features: Option<Vec<String>>,
}
