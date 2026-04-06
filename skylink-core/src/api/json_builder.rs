use std::sync::Arc;
use std::time::Duration;
use tracing::info;

use crate::aircraft::Store;

/// Periodically rebuild the JSON cache (every 1s)
/// This is how readsb does it — pre-build, serve from memory
pub async fn run(store: Arc<Store>) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    loop {
        interval.tick().await;
        let t0 = std::time::Instant::now();
        store.rebuild_json();
        let elapsed = t0.elapsed();
        let size = store.json_cache.read().len();
        if elapsed.as_millis() > 50 {
            info!("json_builder: {}ms, {} bytes, {} aircraft", elapsed.as_millis(), size, store.map.len());
        }
    }
}
