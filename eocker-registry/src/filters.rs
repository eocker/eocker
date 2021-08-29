use futures::Stream;
use futures::StreamExt;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use uuid::Uuid;
use warp::Filter;

use super::handlers::{
    blob_exists, get_blob, get_manifest, manifest_exists, store_blob, store_chunk, store_manifest,
};

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

fn with_tx(
    tx: broadcast::Sender<String>,
) -> impl Filter<Extract = (broadcast::Sender<String>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || tx.clone())
}

fn with_rx(
    tx: broadcast::Sender<String>,
) -> impl Filter<Extract = (broadcast::Receiver<String>,), Error = std::convert::Infallible> + Clone
{
    warp::any().map(move || tx.subscribe())
}

pub fn registry(
    manifests: ManifestStore,
    blobs: BlobStore,
    uploads: UploadStore,
    tx: broadcast::Sender<String>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    events(tx.clone())
        .or(support(tx))
        .or(pull_manifest(manifests.clone()))
        .or(pull_blob(blobs.clone()))
        .or(check_manifest(manifests.clone()))
        .or(check_blob(blobs.clone()))
        .or(blob_location())
        .or(upload_chunk(uploads.clone()))
        .or(push_blob_location())
        .or(push_blob(blobs, uploads))
        .or(push_manifest(manifests))
}

pub fn events(
    tx: broadcast::Sender<String>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path("events")
        .and(warp::get())
        .and(with_rx(tx))
        .map(|rx: broadcast::Receiver<String>| {
            warp::sse::reply(convert_broadcast(BroadcastStream::new(rx)))
        })
}

fn convert_broadcast(
    s: tokio_stream::wrappers::BroadcastStream<String>,
) -> impl Stream<Item = Result<warp::sse::Event, warp::Error>> + Send + 'static {
    // Convert broadcast stream messages into server side events.
    s.map(|msg| Ok(warp::sse::Event::default().data(msg.unwrap())))
}

// --- Support

// Specification Support
// GET /v2/
// Does not currently support authn / authz checks.
pub fn support(
    tx: broadcast::Sender<String>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("v2")
        .and(warp::get())
        .and(with_tx(tx))
        .map(|tx: broadcast::Sender<String>| {
            tx.send("support".to_string()).unwrap();
            warp::reply()
        })
}

// --- Pull

// Pull Manifest
// GET /v2/<name>/manifests/<reference>
pub fn pull_manifest(
    store: ManifestStore,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("v2" / String / "manifests" / String)
        .and(warp::get())
        .and(with_manifest_store(store))
        .and_then(get_manifest)
}

// Pull Blob
// GET /v2/<name>/blobs/<digest>
pub fn pull_blob(
    store: BlobStore,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("v2" / String / "blobs" / String)
        .and(warp::get())
        .and(with_blob_store(store))
        .and_then(get_blob)
}

// Check Manifest
// HEAD /v2/<name>/manifests/<reference>
pub fn check_manifest(
    store: ManifestStore,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("v2" / String / "manifests" / String)
        .and(warp::head())
        .and(with_manifest_store(store))
        .and_then(manifest_exists)
}

// Check Blob
// HEAD /v2/<name>/blobs/<digest>
pub fn check_blob(
    store: BlobStore,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("v2" / String / "blobs" / String)
        .and(warp::head())
        .and(with_blob_store(store))
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
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("v2" / String / "blobs" / "uploads" / Uuid)
        .and(warp::patch())
        .and(warp::header::optional::<String>("Content-Length"))
        .and(warp::header::optional::<String>("Content-Range"))
        .and(warp::body::bytes())
        .and(with_upload_store(store))
        .and_then(store_chunk)
}

// Push Blob
// Could be committing chunked upload or doing monolithic push.
// PUT /v2/<name>/blobs/uploads/<uuid>?digest=<digest>
pub fn push_blob(
    blob_store: BlobStore,
    upload_store: UploadStore,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("v2" / String / "blobs" / "uploads" / Uuid)
        .and(warp::put())
        .and(warp::header("Content-Length"))
        .and(warp::query::<PushQuery>())
        .and(warp::body::bytes())
        .and(with_blob_store(blob_store))
        .and(with_upload_store(upload_store))
        .and_then(store_blob)
}

// Push Manifest
// PUT /v2/<name>/manifests/<reference>
pub fn push_manifest(
    store: ManifestStore,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("v2" / String / "manifests" / String)
        .and(warp::put())
        .and(warp::header("Content-Type"))
        .and(warp::body::bytes())
        .and(with_manifest_store(store))
        .and_then(store_manifest)
}
