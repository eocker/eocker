use tokio::sync::broadcast;

mod codes;
mod filters;
mod handlers;
mod store;

#[tokio::main]
async fn main() {
    let (tx, _) = broadcast::channel::<String>(10);
    let blob_store = store::new_blob_store();
    let manifest_store = store::new_manifest_store();

    warp::serve(filters::registry(manifest_store, blob_store, tx))
        .run(([127, 0, 0, 1], 8080))
        .await;
}
