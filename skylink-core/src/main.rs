mod beast;
mod mode_s;
mod aircraft;
mod api;
mod bincraft;

use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let store = Arc::new(aircraft::Store::new());

    let ingest_port: u16 = env("INGEST_PORT", 39004);
    let api_port: u16 = env("API_PORT", 19180);

    info!("skylink-core v2 starting (ingest:{} api:{})", ingest_port, api_port);

    // JSON pre-builder: serialize aircraft.json every 1s into a shared buffer
    let s = store.clone();
    tokio::spawn(async move { api::json_builder::run(s).await; });

    // Reaper
    let s = store.clone();
    tokio::spawn(async move { aircraft::reaper(s).await; });

    // Beast ingest
    let s = store.clone();
    tokio::spawn(async move { beast::serve_ingest(s, ingest_port).await; });

    // HTTP API
    api::serve(store, api_port).await;
}

fn env(key: &str, default: u16) -> u16 {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}
