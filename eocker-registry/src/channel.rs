use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::Mutex;

pub type ChannelMap = Arc<Mutex<HashMap<String, broadcast::Sender<String>>>>;

// TODO(hasheddan): channels are not cleaned up when no one is subscribed.
pub fn new_channel_map() -> ChannelMap {
    Arc::new(Mutex::new(HashMap::new()))
}

pub async fn send(ns: &str, message: String, sm: ChannelMap) {
    let st = sm.lock().await;
    match st.get(ns) {
        // If channel exists we will send on it.
        None => (),
        Some(tx) => {
            tx.send(message).unwrap();
        }
    }
}
