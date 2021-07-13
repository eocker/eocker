use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time};
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

#[derive(Serialize, Deserialize, Debug)]
pub struct ConfigFile {
    architecture: String,
    author: Option<String>,
    container: Option<String>,
    created: Option<chrono::DateTime<chrono::Utc>>,
    docker_version: Option<String>,
    history: Option<Vec<History>>,
    os: String,
    rootfs: RootFS,
    config: Config,
    #[serde(rename = "os.version")]
    os_version: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct History {
    author: Option<String>,
    created: Option<chrono::DateTime<chrono::Utc>>,
    created_by: Option<String>,
    comment: Option<String>,
    empty_layer: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RootFS {
    #[serde(rename = "type")]
    root_fs_type: String,
    diff_ids: Vec<digest::Hash>,
}


#[derive(Serialize, Deserialize, Debug)]
pub struct HealthConfig {
    test: Option<Vec<String>>,
    interval: Option<time::Duration>,
    timeout: Option<time::Duration>,
    start_period: Option<time::Duration>,
    retries: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Config {
    attach_stderr: Option<bool>,
    attach_stdin: Option<bool>,
    attach_stdout: Option<bool>,
    cmd: Option<Vec<String>>,
    healthcheck: Option<HealthConfig>,
    domainnname: Option<String>,
    entrypoint: Option<Vec<String>>,
    env: Option<Vec<String>>,
    hostname: Option<String>,
    image: Option<String>,
    labels: Option<HashMap<String, String>>,
    on_build: Option<Vec<String>>,
    open_stdin: Option<bool>,
    stdin_once: Option<bool>,
    tty: Option<bool>,
    user: Option<String>,
    volumes: Option<HashMap<String,serde_json::value::Value>>,
    working_dir: Option<String>,
    exposed_ports: Option<HashMap<String,serde_json::value::Value>>,
    args_escaped: Option<bool>,
    network_disabled: Option<bool>,
    mac_address: Option<String>,
    stop_signal: Option<String>,
    shell: Option<Vec<String>>
}
