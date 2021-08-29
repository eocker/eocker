mod channel;
mod codes;
mod filters;
mod handlers;
mod store;

#[tokio::main]
async fn main() {
    let blob_store = store::new_blob_store();
    let upload_store = store::new_upload_store();
    let manifest_store = store::new_manifest_store();
    let channel_map = channel::new_channel_map();

    warp::serve(filters::registry(
        manifest_store,
        blob_store,
        upload_store,
        channel_map,
    ))
    .run(([127, 0, 0, 1], 8080))
    .await;
}
