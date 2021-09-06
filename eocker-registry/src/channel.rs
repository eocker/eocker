use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::Mutex;
use warp::http::{Method, StatusCode};

pub type ChannelMap = Arc<Mutex<HashMap<String, broadcast::Sender<Event>>>>;

// TODO(hasheddan): channels are not cleaned up when no one is subscribed.
pub fn new_channel_map() -> ChannelMap {
    Arc::new(Mutex::new(HashMap::new()))
}

pub async fn send(
    ns: &str,
    data_type: String,
    method: Method,
    status: StatusCode,
    identifier: String,
    sm: ChannelMap,
) {
    let st = sm.lock().await;
    match st.get(ns) {
        // If channel exists we will send on it.
        None => (),
        Some(tx) => {
            tx.send(Event {
                data_type: data_type,
                method: method.to_string(),
                status: status.as_str().to_string(),
                repo: ns.to_string(),
                identifier: identifier,
            })
            .unwrap();
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    // TODO(hasheddan): use eocker lib types
    data_type: String,
    method: String,
    status: String,
    repo: String,
    identifier: String,
}
