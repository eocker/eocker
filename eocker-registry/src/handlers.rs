use bytes::Bytes;
use std::{collections::HashMap, convert::Infallible};
use uuid::Uuid;
use warp::http::StatusCode;

use super::store::{BlobStore, Manifest, ManifestStore, PushQuery};

pub async fn store_blob(
    _: String,
    _: Uuid,
    _: String,
    query: PushQuery,
    content: Bytes,
    store: BlobStore,
) -> Result<impl warp::Reply, Infallible> {
    // NOTE(hasheddan): blobs are currently stored at global scope
    let mut s = store.lock().await;
    s.insert(query.digest, content);
    Ok(StatusCode::OK)
}

pub async fn get_blob(
    _: String,
    digest: String,
    store: BlobStore,
) -> Result<impl warp::Reply, Infallible> {
    let s = store.lock().await;
    match s.get(digest.as_str()) {
        None => Ok(warp::http::Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(bytes::Bytes::new())),
        Some(b) => Ok(warp::http::Response::builder()
            .status(StatusCode::OK)
            .header("Docker-Content-Digest", digest)
            .header("Content-Length", b.len())
            .body(b.clone())),
    }
}

pub async fn blob_exists(
    _: String,
    digest: String,
    store: BlobStore,
) -> Result<impl warp::Reply, Infallible> {
    let s = store.lock().await;
    if s.contains_key(digest.as_str()) {
        return Ok(StatusCode::OK);
    }
    Ok(StatusCode::NOT_FOUND)
}

pub async fn store_manifest(
    repo: String,
    reference: String,
    content: Bytes,
    store: ManifestStore,
) -> Result<impl warp::Reply, Infallible> {
    // TODO(hasheddan): consider only locking nested repo manifest hash map
    let mut s = store.lock().await;
    let e = s.entry(repo).or_insert_with(|| HashMap::new());
    e.insert(
        reference,
        Manifest {
            content_type: "application/vnd.oci.image.manifest.v1+json".to_string(),
            content: content,
        },
    );
    Ok(StatusCode::OK)
}

pub async fn get_manifest(
    repo: String,
    reference: String,
    store: ManifestStore,
) -> Result<impl warp::Reply, Infallible> {
    // TODO(hasheddan): consider only locking nested repo manifest hash map
    let s = store.lock().await;
    match s.get(repo.as_str()) {
        None => Ok(warp::http::Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(bytes::Bytes::new())),
        Some(r) => match r.get(reference.as_str()) {
            None => Ok(warp::http::Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(bytes::Bytes::new())),
            Some(m) => Ok(warp::http::Response::builder()
                .status(StatusCode::OK)
                // TODO(hasheddan): set Docker-Content-Digest header
                .header("Content-Type", m.content_type.clone())
                .header("Content-Length", m.content.len())
                .body(m.content.clone())),
        },
    }
}

pub async fn manifest_exists(
    repo: String,
    reference: String,
    store: ManifestStore,
) -> Result<impl warp::Reply, Infallible> {
    // TODO(hasheddan): consider only locking nested repo manifest hash map
    let s = store.lock().await;
    let e = s.get(repo.as_str());
    match e {
        None => Ok(StatusCode::NOT_FOUND),
        Some(m) => {
            if m.contains_key(reference.as_str()) {
                return Ok(StatusCode::OK);
            }
            Ok(StatusCode::NOT_FOUND)
        }
    }
}
