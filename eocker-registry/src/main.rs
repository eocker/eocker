use std::convert::TryFrom;
use warp::Filter;
use eocker::types;
use eocker::types::Media;
use urlencoding::decode;

#[tokio::main]
async fn main() {
    let is_image = warp::path!("is_image" / String)
        .map(|name: String| {
            let d = decode(&name).unwrap().into_owned();
            println!("{:?}", d);
            let v = types::MediaType::try_from(d.as_str()).unwrap();
            format!("Is image?: {}", v.is_image())
        });

    warp::serve(is_image)
        .run(([127, 0, 0, 1], 8080))
        .await;
}
