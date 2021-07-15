use sha2::{Digest, Sha256};
use flate2::{write::GzEncoder, Compression};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, error::Error, io::Write, time};
use types::MediaType;

pub mod digest;
pub mod types;

#[derive(Debug)]
pub struct Image {
    pub manifest: Manifest,
    pub config: ConfigFile,
    pub layers: Vec<Layer>,
}

impl Image {
    pub fn new_from_layer(layer: Layer) -> Result<Image, Box<dyn Error>> {
        // build config file
        let config = ConfigFile {
            rootfs: RootFS {
                diff_ids: vec![layer.diff_id.clone()],
                ..Default::default()
            },
            ..Default::default()
        };
        // serialize to JSON
        let serial = serde_json::to_string(&config)?;
        let raw_config = serial.as_bytes();
        let mut c = Sha256::new();
        c.update(&raw_config);
        let manifest = Manifest {
            schema_version: 2,
            media_type: Some(MediaType::DockerManifestSchema2),
            config: Descriptor {
                media_type: MediaType::DockerConfigJSON,
                size: raw_config.len() as i64,
                digest: digest::Hash {
                    algorithm: "sha256".to_string(),
                    hex: format!("{:x}", c.finalize()),
                },
                urls: None,
                annotations: None,
                platform: None,
            },
            layers: vec![layer.descriptor.clone()],
            annotations: None,
        };
        Ok(Image {
            manifest: manifest,
            config: config,
            layers: vec![layer],
        })
    }

    pub fn get_manifest(&self) -> &Manifest {
        &self.manifest
    }

    pub fn get_config(&self) -> &ConfigFile {
        &self.config
    }
}

#[derive(Debug)]
pub struct Layer {
    pub content: Vec<u8>,
    pub diff_id: digest::Hash,
    pub descriptor: Descriptor,
}

impl Layer {
    pub fn new_for_content(content: &[u8], path: &str) -> Result<Layer, Box<dyn Error>> {
        let mut header = tar::Header::new_gnu();
        header.set_path(path).unwrap();
        header.set_size(content.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        // create tarball
        let mut tar = tar::Builder::new(Vec::new());
        tar.append(&header, content)?;
        let t = tar.into_inner()?;
        // get diff ID from uncompressed archive
        let mut u = Sha256::new();
        u.update(&t);
        // gzip tarball
        let mut enc = GzEncoder::new(Vec::new(), Compression::fast());
        enc.write_all(&t)?;
        let tar_gz = enc.finish()?;
        // get digest from compressed archive
        let mut c = Sha256::new();
        c.update(&tar_gz);
        Ok(Layer {
            descriptor: Descriptor {
                media_type: MediaType::DockerLayer,
                size: tar_gz.len() as i64,
                digest: digest::Hash {
                    algorithm: "sha256".to_string(),
                    hex: format!("{:x}", c.finalize()),
                },
                urls: None,
                annotations: None,
                platform: None,
            },
            content: tar_gz,
            diff_id: digest::Hash {
                algorithm: "sha256".to_string(),
                hex: format!("{:x}", u.finalize()),
            },
        })
    }
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub schema_version: i64,
    pub media_type: Option<MediaType>,
    pub config: Descriptor,
    pub layers: Vec<Descriptor>,
    pub annotations: Option<HashMap<String, String>>,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct IndexManifest {
    pub schema_version: i64,
    pub media_type: Option<MediaType>,
    pub manifests: Vec<Descriptor>,
    pub annotations: Option<HashMap<String, String>>,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Descriptor {
    pub media_type: MediaType,
    pub size: i64,
    pub digest: digest::Hash,
    pub urls: Option<Vec<String>>,
    pub annotations: Option<HashMap<String, String>>,
    pub platform: Option<Platform>,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Platform {
    pub architecture: String,
    pub os: String,
    #[serde(rename = "os.version")]
    pub os_version: Option<String>,
    #[serde(rename = "os.features")]
    pub os_features: Option<Vec<String>>,
    pub variant: Option<String>,
    pub features: Option<Vec<String>>,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ConfigFile {
    pub architecture: String,
    pub author: Option<String>,
    pub container: Option<String>,
    pub created: Option<chrono::DateTime<chrono::Utc>>,
    pub docker_version: Option<String>,
    pub history: Option<Vec<History>>,
    pub os: String,
    pub rootfs: RootFS,
    pub config: Option<Config>,
    #[serde(rename = "os.version")]
    pub os_version: Option<String>,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct History {
    pub author: Option<String>,
    pub created: Option<chrono::DateTime<chrono::Utc>>,
    pub created_by: Option<String>,
    pub comment: Option<String>,
    pub empty_layer: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RootFS {
    #[serde(rename = "type")]
    pub root_fs_type: String,
    pub diff_ids: Vec<digest::Hash>,
}

impl Default for RootFS {
    fn default() -> Self {
        RootFS {
            root_fs_type: "layers".to_string(),
            diff_ids: vec![],
        }
    }
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct HealthConfig {
    pub test: Option<Vec<String>>,
    pub interval: Option<time::Duration>,
    pub timeout: Option<time::Duration>,
    pub start_period: Option<time::Duration>,
    pub retries: Option<i32>,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Config {
    pub attach_stderr: Option<bool>,
    pub attach_stdin: Option<bool>,
    pub attach_stdout: Option<bool>,
    pub cmd: Option<Vec<String>>,
    pub healthcheck: Option<HealthConfig>,
    pub domainnname: Option<String>,
    pub entrypoint: Option<Vec<String>>,
    pub env: Option<Vec<String>>,
    pub hostname: Option<String>,
    pub image: Option<String>,
    pub labels: Option<HashMap<String, String>>,
    pub on_build: Option<Vec<String>>,
    pub open_stdin: Option<bool>,
    pub stdin_once: Option<bool>,
    pub tty: Option<bool>,
    pub user: Option<String>,
    pub volumes: Option<HashMap<String, serde_json::value::Value>>,
    pub working_dir: Option<String>,
    pub exposed_ports: Option<HashMap<String, serde_json::value::Value>>,
    pub args_escaped: Option<bool>,
    pub network_disabled: Option<bool>,
    pub mac_address: Option<String>,
    pub stop_signal: Option<String>,
    pub shell: Option<Vec<String>>,
}
