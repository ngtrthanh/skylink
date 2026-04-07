mod beast;
mod mode_s;
mod aircraft;
mod api;
mod bincraft;
mod output;
mod feed;
mod pb;
mod compact;
mod ws;
mod mcp;

use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let store = Arc::new(aircraft::Store::new());
    let channels = Arc::new(output::OutputChannels::new());

    let ingest_port: u16 = env("INGEST_PORT", 39004);
    let api_port: u16 = env("API_PORT", 19180);
    let base_port: u16 = env("BASE_PORT", 39000);

    info!("skylink-core v2 starting (ingest:{} api:{} outputs:{}-{})",
        ingest_port, api_port, base_port + 2, base_port + 47);

    // Output TCP listeners (beast, raw, sbs, json-pos)
    let ch = channels.clone();
    tokio::spawn(async move { output::start_all(&ch, base_port).await; });

    // JSON + binCraft pre-builder
    let s = store.clone();
    tokio::spawn(async move { api::json_builder::run(s).await; });

    // SBS feed generator
    let s = store.clone();
    let ch = channels.clone();
    tokio::spawn(async move { feed::run_sbs(s, ch).await; });

    // JSON position feed generator
    let s = store.clone();
    let ch = channels.clone();
    tokio::spawn(async move { feed::run_json_pos(s, ch).await; });

    // Reaper
    let s = store.clone();
    tokio::spawn(async move { aircraft::reaper(s).await; });

    // Beast ingest
    let s = store.clone();
    let ch = channels.clone();
    tokio::spawn(async move { beast::serve_ingest(s, ch, ingest_port).await; });

    // HTTP API (includes MCP endpoints)
    api::serve(store, api_port).await;
}

fn env(key: &str, default: u16) -> u16 {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}
