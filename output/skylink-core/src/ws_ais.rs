/// WebSocket endpoint for vessel data — push binVessel zstd binary every 1s
use std::sync::Arc;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use tokio::time::{interval, Duration};
use tracing::info;
use crate::ais::vessel::VesselStore;
use crate::binvessel;

pub async fn ws_handler(ws: WebSocketUpgrade, State(store): State<Arc<VesselStore>>) -> Response {
    ws.on_upgrade(move |socket| handle_ws(socket, store))
}

async fn handle_ws(mut socket: WebSocket, store: Arc<VesselStore>) {
    info!("ws/ais: client connected");
    let mut bbox: Option<(f64, f64, f64, f64)> = None;
    let mut tick = interval(Duration::from_secs(1));

    loop {
        tokio::select! {
            _ = tick.tick() => {
                let data = match bbox {
                    Some((s, n, w, e)) => {
                        let cache = store.binvessel_cache.read().clone();
                        let raw = binvessel::build_filtered_from_cache(&cache, s, n, w, e);
                        zstd::encode_all(raw.as_slice(), 3).unwrap_or(raw)
                    }
                    None => store.binvessel_zstd_cache.read().to_vec(),
                };
                if socket.send(Message::Binary(data.into())).await.is_err() {
                    break;
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        let t = text.trim();
                        if t == "all" { bbox = None; }
                        else if t.starts_with("box:") {
                            let parts: Vec<f64> = t[4..].split(',').filter_map(|s| s.parse().ok()).collect();
                            if parts.len() == 4 { bbox = Some((parts[0], parts[1], parts[2], parts[3])); }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }
    info!("ws/ais: client disconnected");
}
