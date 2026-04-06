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

    info!("skylink-core starting");

    // Spawn Beast TCP ingest (port 30004)
    let ingest_store = store.clone();
    tokio::spawn(async move {
        beast::ingest::serve(ingest_store, 30004).await;
    });

    // Spawn Beast TCP output (port 30005)
    let output_store = store.clone();
    tokio::spawn(async move {
        output::beast_out::serve(output_store, 30005).await;
    });

    // Spawn reaper (remove stale aircraft)
    let reaper_store = store.clone();
    tokio::spawn(async move {
        state::reaper::run(reaper_store).await;
    });

    // HTTP API (port 8080) — blocks main task
    info!("API listening on :8080");
    api::serve(store, 8080).await;
}
