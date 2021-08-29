use bytes::{BufMut, Bytes};
use std::{collections::HashMap, convert::Infallible};
use uuid::Uuid;
use warp::http::StatusCode;
use std::io::Write;

use super::store::{BlobStore, Manifest, ManifestStore, PushQuery};

pub async fn store_chunk(
    name: String,
    id: Uuid,
    _: String,
    content_length: String,
    content: Bytes,
    store: BlobStore,
) -> Result<impl warp::Reply, Infallible> {
    // NOTE(hasheddan): chunks are currently stored at global scope
    let spl = content_length.split("-").collect::<Vec<&str>>();
    if spl.len() != 2 {
        return Ok(warp::http::Response::builder()
            .status(StatusCode::RANGE_NOT_SATISFIABLE)
            .body(bytes::Bytes::new()));
    }
    let start = match spl[0].parse::<usize>() {
        Ok(start) => start,
        Err(_) => {
            return Ok(warp::http::Response::builder()
                .status(StatusCode::RANGE_NOT_SATISFIABLE)
                .body(bytes::Bytes::new()))
        }
    };
    let mut s = store.lock().await;
    let id_string = id.to_string();
    match s.get_mut(id_string.as_str()) {
        None => {
            // Make sure content range begins with 0
            if start != 0 {
                return Ok(warp::http::Response::builder()
                    .status(StatusCode::RANGE_NOT_SATISFIABLE)
                    .body(bytes::Bytes::new()));
            }
            // Insert first chunk into upload store
            s.insert(id_string, content);
            Ok(warp::http::Response::builder()
                .status(StatusCode::ACCEPTED)
                .header("Location", format!("/v2/{}/blobs/uploads/{}", name, id))
                .body(bytes::Bytes::new()))
        }
        Some(b) => {
            // Ensure that content start equals length of previously uploaded
            // chunks
            if start != b.len() {
                return Ok(warp::http::Response::builder()
                    .status(StatusCode::RANGE_NOT_SATISFIABLE)
                    .body(bytes::Bytes::new()));
            }
            let mut buf = vec![].writer();
            // BufMut operations are infallible so we can unwrap these writes
            // safely
            buf.write(b).unwrap();
            buf.write(&content).unwrap();
            *b = buf.into_inner().into();
            Ok(warp::http::Response::builder()
                .status(StatusCode::ACCEPTED)
                .header("Location", format!("/v2/{}/blobs/uploads/{}", name, id))
                .body(bytes::Bytes::new()))
        }
    }
}

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
