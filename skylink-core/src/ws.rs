/// WebSocket endpoint — push snapshots every 1s
/// Client sends: "box:S,N,W,E" for bbox, "all" for everything
/// Format is GeoJSON text (for FE) — lightweight for MapLibre setData()

use std::sync::Arc;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use tokio::time::{interval, Duration};
use tracing::info;

use crate::aircraft::Store;
use crate::geojson;

pub async fn ws_handler(ws: WebSocketUpgrade, State(store): State<Arc<Store>>) -> Response {
    ws.on_upgrade(move |socket| handle_ws(socket, store))
}

async fn handle_ws(mut socket: WebSocket, store: Arc<Store>) {
    info!("ws: client connected");
    let mut bbox: Option<(f64, f64, f64, f64)> = None;
    let mut tick = interval(Duration::from_secs(1));

    loop {
        tokio::select! {
            _ = tick.tick() => {
                let data = match bbox {
                    Some((s, n, w, e)) => geojson::build_filtered(&store, s, n, w, e),
                    None => store.geojson_cache.read().to_vec(),
                };
                let text = unsafe { String::from_utf8_unchecked(data) };
                if socket.send(Message::Text(text.into())).await.is_err() {
                    break;
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        let t = text.trim();
                        if t == "all" {
                            bbox = None;
                        } else if t.starts_with("box:") {
                            let parts: Vec<f64> = t[4..].split(',').filter_map(|s| s.parse().ok()).collect();
                            if parts.len() == 4 {
                                bbox = Some((parts[0], parts[1], parts[2], parts[3]));
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }
    info!("ws: client disconnected");
}
