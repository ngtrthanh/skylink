use std::sync::Arc;
use std::time::Duration;
use tracing::info;

use super::AircraftStore;

fn now_secs() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64()
}

pub async fn run(store: Arc<AircraftStore>) {
    let timeout = 300.0; // seconds
    loop {
        tokio::time::sleep(Duration::from_secs(30)).await;
        let now = now_secs();
        let before = store.map.len();
        store.map.retain(|_, ac| now - ac.last_update < timeout);
        let removed = before - store.map.len();
        if removed > 0 {
            info!("reaper: removed {} stale aircraft, {} remaining", removed, store.map.len());
        }
    }
}
