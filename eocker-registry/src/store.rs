use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Deserialize)]
pub struct PushQuery {
    // TODO(hasheddan): use eocker digest
    pub digest: String,
}

// TODO(hasheddan): consider using a RwLock
pub type BlobStore = Arc<Mutex<HashMap<String, Bytes>>>;

pub fn new_blob_store() -> BlobStore {
    Arc::new(Mutex::new(HashMap::new()))
}

// TODO(hasheddan): consider using a RwLock
pub type UploadStore = Arc<Mutex<HashMap<String, Bytes>>>;

pub fn new_upload_store() -> UploadStore {
    Arc::new(Mutex::new(HashMap::new()))
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Manifest {
    pub content_type: String,
    pub content: Bytes,
}

// TODO(hasheddan): consider using a RwLock
pub type ManifestStore = Arc<Mutex<HashMap<String, HashMap<String, Manifest>>>>;

pub fn new_manifest_store() -> ManifestStore {
    Arc::new(Mutex::new(HashMap::new()))
}
