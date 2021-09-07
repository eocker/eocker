use bytes::{BufMut, Bytes};
use eocker;
use eocker::types::MediaType;
use futures::Stream;
use futures::StreamExt;
use sha2::{Digest, Sha256};
use std::convert::TryFrom;
use std::io::Write;
use std::{collections::HashMap, convert::Infallible};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use uuid::Uuid;
use warp::http::{Method, StatusCode};

use super::channel::{send, ChannelMap, Event, Ref};
use super::store::{BlobStore, Manifest, ManifestStore, PushQuery, UploadStore};

pub async fn store_chunk(
    ns: String,
    id: Uuid,
    _: Option<String>,
    content_range: Option<String>,
    content: Bytes,
    store: UploadStore,
    cm: ChannelMap,
) -> Result<impl warp::Reply, Infallible> {
    // NOTE(hasheddan): chunks are currently stored at global scope
    let start = match content_range {
        None => None,
        Some(content_range) => {
            let spl = content_range.split("-").collect::<Vec<&str>>();
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
            Some(start)
        }
    };
    let mut s = store.lock().await;
    let id_string = id.to_string();
    let content_len = content.len() - 1;
    match s.get_mut(id_string.as_str()) {
        None => {
            match start {
                // If no content range provided, we treat it as 0
                None => {
                    s.insert(id_string, content);
                }
                // If content range is provided, it must start with 0 because we
                // don't have any existing chunks
                Some(start) => {
                    if start != 0 {
                        return Ok(warp::http::Response::builder()
                            .status(StatusCode::RANGE_NOT_SATISFIABLE)
                            .body(bytes::Bytes::new()));
                    }
                    s.insert(id_string, content);
                }
            }
            send(
                &ns,
                "Upload".to_string(),
                Method::PATCH,
                StatusCode::ACCEPTED,
                id.to_string(),
                None,
                cm,
            )
            .await;
            Ok(warp::http::Response::builder()
                .status(StatusCode::ACCEPTED)
                .header("Location", format!("/v2/{}/blobs/uploads/{}", ns, id))
                .header("Range", format!("0-{}", content_len))
                .body(bytes::Bytes::new()))
        }
        Some(b) => {
            // Ensure that content start equals length of previously uploaded
            // chunks
            if start != Some(b.len()) {
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
            send(
                &ns,
                "Upload".to_string(),
                Method::PATCH,
                StatusCode::ACCEPTED,
                id_string,
                None,
                cm,
            )
            .await;
            Ok(warp::http::Response::builder()
                .status(StatusCode::ACCEPTED)
                .header("Location", format!("/v2/{}/blobs/uploads/{}", ns, id))
                .body(bytes::Bytes::new()))
        }
    }
}

pub async fn store_blob(
    ns: String,
    id: Uuid,
    _: String,
    query: PushQuery,
    content: Bytes,
    blob_store: BlobStore,
    upload_store: UploadStore,
    cm: ChannelMap,
) -> Result<impl warp::Reply, Infallible> {
    // NOTE(hasheddan): blobs and uploads are currently stored at global scope
    let mut s = blob_store.lock().await;
    let mut u = upload_store.lock().await;
    let id_string = id.to_string();
    match u.get_mut(id_string.as_str()) {
        None => {
            // Upload store does not have a record for id, so we go ahead and
            // store full blob in blob store
            s.insert(query.digest.clone(), content);
        }
        Some(b) => {
            // Prior upload chunks exist so we append bytes to existing and
            // store result in blob store
            let mut buf = vec![].writer();
            // BufMut operations are infallible so we can unwrap these writes
            // safely
            buf.write(b).unwrap();
            buf.write(&content).unwrap();
            s.insert(query.digest.clone(), buf.into_inner().into());
            // Blob has been uploaded, chunks can be removed from upload store
            u.remove(id_string.as_str());
        }
    }
    send(
        &ns.clone(),
        "Blob".to_string(),
        Method::PUT,
        StatusCode::CREATED,
        query.digest,
        Some(vec![Ref { data_type: "Upload".to_string(), repo: ns, identifier: id.to_string()}]),
        cm,
    )
    .await;
    Ok(StatusCode::CREATED)
}

pub async fn get_blob(
    ns: String,
    digest: String,
    store: BlobStore,
    cm: ChannelMap,
) -> Result<impl warp::Reply, Infallible> {
    let s = store.lock().await;
    send(
        &ns,
        "Blob".to_string(),
        Method::GET,
        StatusCode::OK,
        digest.clone(),
        None,
        cm,
    )
    .await;
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

fn convert_broadcast(
    s: tokio_stream::wrappers::BroadcastStream<Event>,
) -> impl Stream<Item = Result<warp::sse::Event, warp::Error>> + Send + 'static {
    // Convert broadcast stream messages into server side events.
    s.map(|msg| Ok(warp::sse::Event::default().json_data(msg.unwrap()).unwrap()))
}

pub async fn send_events(ns: String, cm: ChannelMap) -> Result<impl warp::Reply, Infallible> {
    let mut c = cm.lock().await;
    // Check if channel exists for namespace and create one if it does not.
    let tx = c.entry(ns).or_insert(broadcast::channel::<Event>(10).0);
    Ok(warp::sse::reply(convert_broadcast(BroadcastStream::new(
        tx.subscribe(),
    ))))
}

pub async fn blob_exists(
    ns: String,
    digest: String,
    store: BlobStore,
    cm: ChannelMap,
) -> Result<impl warp::Reply, Infallible> {
    let s = store.lock().await;
    if s.contains_key(digest.as_str()) {
        send(
            &ns,
            "Blob".to_string(),
            Method::HEAD,
            StatusCode::OK,
            digest.clone(),
            None,
            cm,
        )
        .await;
        return Ok(StatusCode::OK);
    }
    send(
        &ns,
        "Blob".to_string(),
        Method::HEAD,
        StatusCode::NOT_FOUND,
        digest.clone(),
        None,
        cm,
    )
    .await;
    Ok(StatusCode::NOT_FOUND)
}

pub async fn store_manifest(
    ns: String,
    reference: String,
    content_type: String,
    content: Bytes,
    store: ManifestStore,
    cm: ChannelMap,
) -> Result<impl warp::Reply, Infallible> {
    let mut c = Sha256::new();
    c.update(&content);
    let digest = format!("sha256:{:x}", c.finalize());
    // TODO(hasheddan): consider only locking nested repo manifest hash map
    let mut s = store.lock().await;
    let e = s.entry(ns.clone()).or_insert_with(|| HashMap::new());
    let m = Manifest {
        content_type: content_type.clone(),
        content: content.clone(),
    };
    e.insert(reference.clone(), m.clone());
    e.insert(digest.clone(), m);
    let refs: Vec<Ref> = match MediaType::try_from(content_type.as_str()).unwrap() {
        MediaType::OCIImageIndex | MediaType::DockerManifestList => {
            let i: eocker::IndexManifest = serde_json::from_slice(content.as_ref()).unwrap();
            i.manifests
                .iter()
                .map(|l| {
                    return Ref {
                        data_type: "Manifest".to_string(),
                        repo: ns.clone(),
                        identifier: format!("{}:{}", l.digest.algorithm, l.digest.hex),
                    };
                })
                .collect()
        }
        _ => {
            let m: eocker::Manifest = serde_json::from_slice(content.as_ref()).unwrap();
            let mut mrefs: Vec<Ref> = m
                .layers
                .iter()
                .map(|l| {
                    return Ref {
                        data_type: "Blob".to_string(),
                        repo: ns.clone(),
                        identifier: format!("{}:{}", l.digest.algorithm, l.digest.hex),
                    };
                })
                .collect();
            mrefs.push(Ref {
                data_type: "Blob".to_string(),
                repo: ns.clone(),
                identifier: format!("{}:{}", m.config.digest.algorithm, m.config.digest.hex),
            });
            mrefs
        }
    };
    send(
        &ns,
        "Manifest".to_string(),
        Method::PUT,
        StatusCode::OK,
        reference,
        Some(refs),
        cm,
    )
    .await;
    Ok(warp::http::Response::builder()
        .status(StatusCode::CREATED)
        .header("Docker-Content-Digest", digest)
        .body(bytes::Bytes::new()))
}

pub async fn get_manifest(
    ns: String,
    reference: String,
    store: ManifestStore,
    cm: ChannelMap,
) -> Result<impl warp::Reply, Infallible> {
    // TODO(hasheddan): consider only locking nested repo manifest hash map
    let s = store.lock().await;
    match s.get(ns.as_str()) {
        None => {
            send(
                &ns,
                "Manifest".to_string(),
                Method::GET,
                StatusCode::NOT_FOUND,
                reference,
                None,
                cm,
            )
            .await;
            Ok(warp::http::Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(bytes::Bytes::new()))
        }
        Some(r) => match r.get(reference.as_str()) {
            None => {
                send(
                    &ns,
                    "Manifest".to_string(),
                    Method::GET,
                    StatusCode::NOT_FOUND,
                    reference,
                    None,
                    cm,
                )
                .await;
                Ok(warp::http::Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(bytes::Bytes::new()))
            }
            Some(m) => {
                send(
                    &ns,
                    "Manifest".to_string(),
                    Method::GET,
                    StatusCode::OK,
                    reference,
                    None,
                    cm,
                )
                .await;
                Ok(warp::http::Response::builder()
                    .status(StatusCode::OK)
                    // TODO(hasheddan): set Docker-Content-Digest header
                    .header("Content-Type", m.content_type.clone())
                    .header("Content-Length", m.content.len())
                    .body(m.content.clone()))
            }
        },
    }
}

pub async fn manifest_exists(
    ns: String,
    reference: String,
    store: ManifestStore,
    cm: ChannelMap,
) -> Result<impl warp::Reply, Infallible> {
    // TODO(hasheddan): consider only locking nested repo manifest hash map
    let s = store.lock().await;
    let e = s.get(ns.as_str());
    match e {
        None => {
            send(
                &ns,
                "Manifest".to_string(),
                Method::HEAD,
                StatusCode::NOT_FOUND,
                reference,
                None,
                cm,
            )
            .await;
            Ok(StatusCode::NOT_FOUND)
        }
        Some(m) => {
            if m.contains_key(reference.as_str()) {
                send(
                    &ns,
                    "Manifest".to_string(),
                    Method::HEAD,
                    StatusCode::OK,
                    reference,
                    None,
                    cm,
                )
                .await;
                return Ok(StatusCode::OK);
            }
            send(
                &ns,
                "Manifest".to_string(),
                Method::HEAD,
                StatusCode::NOT_FOUND,
                reference,
                None,
                cm,
            )
            .await;
            Ok(StatusCode::NOT_FOUND)
        }
    }
}
