mod beast;
mod mode_s;
mod aircraft;
mod db;
mod api;
mod bincraft;
mod output;
mod feed;
mod pb;
mod compact;
mod ws;
mod mcp;
mod geojson;
mod config;
mod ais;
mod ws_ais;

use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let mut cfg = config::Config::load();
    cfg.apply_cli();

    info!("skylink-core v4 starting (adsb={} ais={})", cfg.modules.adsb, cfg.modules.ais);

    // Aircraft store (only if adsb enabled)
    let aircraft_store = if cfg.modules.adsb {
        Some(Arc::new(aircraft::Store::new()))
    } else { None };

    // Vessel store (only if ais enabled)
    let vessel_store = if cfg.modules.ais {
        Some(Arc::new(ais::vessel::VesselStore::new()))
    } else { None };

    // --- ADS-B module ---
    if let Some(ref store) = aircraft_store {
        let channels = Arc::new(output::OutputChannels::new());
        let base_port: u16 = std::env::var("BASE_PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(39000);

        let ch = channels.clone();
        tokio::spawn(async move { output::start_all(&ch, base_port).await; });

        let s = store.clone();
        tokio::spawn(async move { api::json_builder::run(s).await; });

        let s = store.clone();
        let ch = channels.clone();
        tokio::spawn(async move { feed::run_sbs(s, ch).await; });

        let s = store.clone();
        let ch = channels.clone();
        tokio::spawn(async move { feed::run_json_pos(s, ch).await; });

        let s = store.clone();
        tokio::spawn(async move { aircraft::reaper(s).await; });

        let s = store.clone();
        let ch = channels.clone();
        let port = cfg.adsb.ingest_port;
        tokio::spawn(async move { beast::serve_ingest(s, ch, port).await; });

        info!("adsb: beast ingest on port {}", cfg.adsb.ingest_port);
    }

    // --- AIS module ---
    if let Some(ref store) = vessel_store {
        let s = store.clone();
        let host = cfg.ais.nmea_host.clone();
        tokio::spawn(async move { ais::ingest(s, host).await; });

        let s = store.clone();
        tokio::spawn(async move { ais::vessel::cache_loop(s).await; });

        let s = store.clone();
        tokio::spawn(async move { ais::vessel::reaper(s).await; });

        info!("ais: nmea ingest from {}", cfg.ais.nmea_host);
    }

    // --- HTTP API ---
    api::serve(aircraft_store, vessel_store, cfg).await;
}
