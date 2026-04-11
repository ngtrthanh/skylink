/// Feed generator: converts decoded messages into output formats
/// and broadcasts to subscribers via output channels.

use std::sync::Arc;
use std::time::Duration;

use crate::aircraft::Store;
use crate::output::OutputChannels;

/// Generate SBS (BaseStation) output from aircraft state, every 1s
pub async fn run_sbs(store: Arc<Store>, channels: Arc<OutputChannels>) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    loop {
        interval.tick().await;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64();

        for entry in store.map.iter() {
            let ac = entry.value();
            // Only emit recently-seen aircraft
            if now - ac.last_update > 5.0 { continue; }

            // SBS format: MSG,3 for position, MSG,4 for velocity
            if let (Some(lat), Some(lon)) = (ac.lat, ac.lon) {
                let line = format!(
                    "MSG,3,1,1,{},1,,,,,{},{},{},,,,,,,\r\n",
                    ac.hex.to_uppercase(),
                    ac.alt_baro.unwrap_or(0),
                    lat, lon
                );
                let _ = channels.sbs.send(line.into_bytes());
            }

            if let Some(gs) = ac.gs {
                let line = format!(
                    "MSG,4,1,1,{},1,,,,,,{},{},{},,,,,\r\n",
                    ac.hex.to_uppercase(),
                    gs,
                    ac.track.unwrap_or(0.0),
                    ac.baro_rate.unwrap_or(0)
                );
                let _ = channels.sbs.send(line.into_bytes());
            }
        }
    }
}

/// Generate JSON position output, every 1s
pub async fn run_json_pos(store: Arc<Store>, channels: Arc<OutputChannels>) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    loop {
        interval.tick().await;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64();

        for entry in store.map.iter() {
            let ac = entry.value();
            if now - ac.last_update > 5.0 { continue; }
            if let (Some(lat), Some(lon)) = (ac.lat, ac.lon) {
                let line = format!(
                    "{{\"hex\":\"{}\",\"lat\":{:.6},\"lon\":{:.6},\"alt\":{},\"track\":{}}}\n",
                    ac.hex,
                    lat, lon,
                    ac.alt_baro.unwrap_or(0),
                    ac.track.unwrap_or(0.0)
                );
                let _ = channels.json_pos.send(line.into_bytes());
            }
        }
    }
}
