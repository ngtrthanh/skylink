use std::sync::Arc;
use std::time::Duration;

use crate::aircraft::Store;
use crate::bincraft;

fn zstd3(data: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut out = Vec::with_capacity(data.len() / 2);
    let mut encoder = zstd::Encoder::new(&mut out, 3).unwrap();
    encoder.set_pledged_src_size(Some(data.len() as u64)).unwrap();
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap();
    out
}

/// Periodically rebuild all caches (every 1s)
pub async fn run(store: Arc<Store>) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    loop {
        interval.tick().await;

        store.rebuild_json();
        let json = store.json_cache.read().clone();
        *store.json_zstd_cache.write() = bytes::Bytes::from(zstd3(&json));

        let bin = bincraft::build(&store);
        *store.bincraft_zstd_cache.write() = bytes::Bytes::from(zstd3(&bin));
        *store.bincraft_cache.write() = bytes::Bytes::from(bin);

        let pb = crate::pb::build_aircraft_pb(&store);
        *store.pb_zstd_cache.write() = bytes::Bytes::from(zstd3(&pb));
        *store.pb_cache.write() = bytes::Bytes::from(pb);

        let cpt = crate::compact::build(&store);
        *store.compact_zstd_cache.write() = bytes::Bytes::from(zstd3(&cpt));
        *store.compact_cache.write() = bytes::Bytes::from(cpt);

        let gj = crate::geojson::build(&store);
        *store.geojson_zstd_cache.write() = bytes::Bytes::from(zstd3(&gj));
        *store.geojson_cache.write() = bytes::Bytes::from(gj);

        // Rebuild receivers cache from connected clients
        let clients = store.clients.read().clone();
        let rcv: Vec<serde_json::Value> = clients.iter().enumerate().map(|(i, c)| {
            serde_json::json!({ "uid": format!("feeder-{i}"), "addr": c.addr, "connected": c.connected_at })
        }).collect();
        *store.receivers_cache.write() = bytes::Bytes::from(serde_json::to_vec(&rcv).unwrap_or_default());
    }
}
