/// Unified WebSocket — pushes both aircraft + vessels in a single connection
/// Frame format: 1-byte type header + zstd payload
///   0x01 = aircraft binCraft zstd
///   0x02 = vessel binVessel zstd
/// Alternates: even ticks → aircraft, odd ticks → vessels

use std::sync::Arc;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use tokio::time::{interval, Duration};
use tracing::info;

use crate::aircraft::Store;
use crate::ais::vessel::VesselStore;

pub struct UnifiedState {
    pub aircraft: Option<Arc<Store>>,
    pub vessels: Option<Arc<VesselStore>>,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    axum::extract::State(state): axum::extract::State<Arc<UnifiedState>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_ws(socket, state))
}

async fn handle_ws(mut socket: WebSocket, state: Arc<UnifiedState>) {
    info!("ws/unified: client connected");
    let mut bbox: Option<(f64, f64, f64, f64)> = None;
    let mut tick = interval(Duration::from_millis(500)); // 2Hz: alternate ac/vessel
    let mut frame = 0u32;

    loop {
        tokio::select! {
            _ = tick.tick() => {
                let data = if frame % 2 == 0 {
                    // Aircraft frame
                    if let Some(ref store) = state.aircraft {
                        let raw = match bbox {
                            Some((s, n, w, e)) => {
                                let cache = store.bincraft_cache.read().clone();
                                crate::bincraft::build_filtered_from_cache(&cache, s, n, w, e)
                            }
                            None => store.bincraft_cache.read().to_vec(),
                        };
                        let compressed = zstd::encode_all(raw.as_slice(), 3).unwrap_or(raw);
                        let mut out = Vec::with_capacity(1 + compressed.len());
                        out.push(0x01);
                        out.extend_from_slice(&compressed);
                        Some(out)
                    } else { None }
                } else {
                    // Vessel frame
                    if let Some(ref store) = state.vessels {
                        let raw = match bbox {
                            Some((s, n, w, e)) => {
                                let cache = store.binvessel_cache.read().clone();
                                crate::binvessel::build_filtered_from_cache(&cache, s, n, w, e)
                            }
                            None => store.binvessel_cache.read().to_vec(),
                        };
                        let compressed = zstd::encode_all(raw.as_slice(), 3).unwrap_or(raw);
                        let mut out = Vec::with_capacity(1 + compressed.len());
                        out.push(0x02);
                        out.extend_from_slice(&compressed);
                        Some(out)
                    } else { None }
                };

                if let Some(d) = data {
                    if socket.send(Message::Binary(d.into())).await.is_err() { break; }
                }
                frame += 1;
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
    info!("ws/unified: client disconnected");
}
