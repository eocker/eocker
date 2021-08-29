use uuid::Uuid;
use warp::Filter;

use super::handlers::{
    blob_exists, get_blob, get_manifest, manifest_exists, send_events, store_blob, store_chunk,
    store_manifest,
};

use super::channel::ChannelMap;
use super::store::{BlobStore, ManifestStore, PushQuery, UploadStore};

fn with_blob_store(
    store: BlobStore,
) -> impl Filter<Extract = (BlobStore,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || store.clone())
}

fn with_upload_store(
    store: UploadStore,
) -> impl Filter<Extract = (UploadStore,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || store.clone())
}

fn with_manifest_store(
    store: ManifestStore,
) -> impl Filter<Extract = (ManifestStore,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || store.clone())
}

fn with_cm(
    cm: ChannelMap,
) -> impl Filter<Extract = (ChannelMap,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || cm.clone())
}

pub fn registry(
    manifests: ManifestStore,
    blobs: BlobStore,
    uploads: UploadStore,
    cm: ChannelMap,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    events(cm.clone())
        .or(support())
        .or(pull_manifest(manifests.clone(), cm.clone()))
        .or(pull_blob(blobs.clone(), cm.clone()))
        .or(check_manifest(manifests.clone(), cm.clone()))
        .or(check_blob(blobs.clone(), cm.clone()))
        .or(blob_location())
        .or(upload_chunk(uploads.clone(), cm.clone()))
        .or(push_blob_location())
        .or(push_blob(blobs, uploads, cm.clone()))
        .or(push_manifest(manifests, cm))
}

pub fn events(
    cm: ChannelMap,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("events" / String)
        .and(warp::get())
        .and(with_cm(cm))
        .and_then(send_events)
}

// --- Support

// Specification Support
// GET /v2/
// Does not currently support authn / authz checks.
pub fn support() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("v2").and(warp::get()).map(|| warp::reply())
}

// --- Pull

// Pull Manifest
// GET /v2/<name>/manifests/<reference>
pub fn pull_manifest(
    store: ManifestStore,
    cm: ChannelMap,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("v2" / String / "manifests" / String)
        .and(warp::get())
        .and(with_manifest_store(store))
        .and(with_cm(cm))
        .and_then(get_manifest)
}

// Pull Blob
// GET /v2/<name>/blobs/<digest>
pub fn pull_blob(
    store: BlobStore,
    cm: ChannelMap,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("v2" / String / "blobs" / String)
        .and(warp::get())
        .and(with_blob_store(store))
        .and(with_cm(cm))
        .and_then(get_blob)
}

// Check Manifest
// HEAD /v2/<name>/manifests/<reference>
pub fn check_manifest(
    store: ManifestStore,
    cm: ChannelMap,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("v2" / String / "manifests" / String)
        .and(warp::head())
        .and(with_manifest_store(store))
        .and(with_cm(cm))
        .and_then(manifest_exists)
}

// Check Blob
// HEAD /v2/<name>/blobs/<digest>
pub fn check_blob(
    store: BlobStore,
    cm: ChannelMap,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("v2" / String / "blobs" / String)
        .and(warp::head())
        .and(with_blob_store(store))
        .and(with_cm(cm))
        .and_then(blob_exists)
}

// --- Push
// Currently only support monolithic POST / PUT and chunked upload

// Blob Location
// POST /v2/<name>/blobs/uploads
pub fn blob_location() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("v2" / String / "blobs" / "uploads")
        .and(warp::post())
        .map(|name: String| {
            warp::reply::with_header(
                (|| warp::http::StatusCode::ACCEPTED)(),
                "Location",
                format!("/v2/{}/blobs/uploads/{}", name, Uuid::new_v4()),
            )
        })
}

// Push Blob Location
// Redirects single POST blob upload to PUT.
// POST /v2/<name>/blobs/uploads/?digest=<digest>
pub fn push_blob_location(
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("v2" / String / "blobs" / "uploads")
        .and(warp::post())
        .and(warp::header("Content-Length"))
        .and(warp::header::exact(
            "Content-Type",
            "application/octet-stream",
        ))
        .and(warp::query::<PushQuery>())
        .map(|name: String, _: String, _: PushQuery| {
            warp::reply::with_header(
                (|| warp::http::StatusCode::ACCEPTED)(),
                "Location",
                format!("/v2/{}/blobs/uploads/{}", name, Uuid::new_v4()),
            )
        })
}

// Upload Chunk
// PATCH /v2/<name>/blobs/uploads/<uuid>
pub fn upload_chunk(
    store: UploadStore,
    cm: ChannelMap,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("v2" / String / "blobs" / "uploads" / Uuid)
        .and(warp::patch())
        .and(warp::header::optional::<String>("Content-Length"))
        .and(warp::header::optional::<String>("Content-Range"))
        .and(warp::body::bytes())
        .and(with_upload_store(store))
        .and(with_cm(cm))
        .and_then(store_chunk)
}

// Push Blob
// Could be committing chunked upload or doing monolithic push.
// PUT /v2/<name>/blobs/uploads/<uuid>?digest=<digest>
pub fn push_blob(
    blob_store: BlobStore,
    upload_store: UploadStore,
    cm: ChannelMap,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("v2" / String / "blobs" / "uploads" / Uuid)
        .and(warp::put())
        .and(warp::header("Content-Length"))
        .and(warp::query::<PushQuery>())
        .and(warp::body::bytes())
        .and(with_blob_store(blob_store))
        .and(with_upload_store(upload_store))
        .and(with_cm(cm))
        .and_then(store_blob)
}

// Push Manifest
// PUT /v2/<name>/manifests/<reference>
pub fn push_manifest(
    store: ManifestStore,
    cm: ChannelMap,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("v2" / String / "manifests" / String)
        .and(warp::put())
        .and(warp::header("Content-Type"))
        .and(warp::body::bytes())
        .and(with_manifest_store(store))
        .and(with_cm(cm))
        .and_then(store_manifest)
}
