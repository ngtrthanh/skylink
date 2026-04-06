use std::sync::Arc;
use std::time::Duration;
use tracing::info;

use crate::aircraft::Store;
use crate::bincraft;

/// Periodically rebuild JSON + binCraft caches (every 1s)
pub async fn run(store: Arc<Store>) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    loop {
        interval.tick().await;
        let t0 = std::time::Instant::now();

        store.rebuild_json();

        let bin = bincraft::build(&store);
        *store.bincraft_cache.write() = bytes::Bytes::from(bin);

        let pb = crate::pb::build_aircraft_pb(&store);
        *store.pb_cache.write() = bytes::Bytes::from(pb);

        let elapsed = t0.elapsed();
        if elapsed.as_millis() > 50 {
            let json_size = store.json_cache.read().len();
            let bin_size = store.bincraft_cache.read().len();
            info!("cache rebuild: {}ms, json={}B, bin={}B, ac={}", elapsed.as_millis(), json_size, bin_size, store.map.len());
        }
    }
}
