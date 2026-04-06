mod beast;
mod decode;
mod state;
mod api;
mod output;

use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let store = Arc::new(state::AircraftStore::new());

    let ingest_port: u16 = std::env::var("INGEST_PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(39004);
    let output_port: u16 = std::env::var("OUTPUT_PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(39005);
    let api_port: u16 = std::env::var("API_PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(19180);

    info!("skylink-core starting (ingest:{} output:{} api:{})", ingest_port, output_port, api_port);

    let s = store.clone();
    tokio::spawn(async move { beast::ingest::serve(s, ingest_port).await; });

    let s = store.clone();
    tokio::spawn(async move { output::beast_out::serve(s, output_port).await; });

    let s = store.clone();
    tokio::spawn(async move { state::reaper::run(s).await; });

    api::serve(store, api_port).await;
}
