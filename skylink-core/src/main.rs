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
mod binvessel;
mod ws_unified;
mod mcp_vessel;
mod nmea_out;

use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let mut cfg = config::Config::load();
    cfg.apply_cli();

    info!("skylink-core v4 starting (adsb={} ais={})", cfg.modules.adsb, cfg.modules.ais);

    let aircraft_store = if cfg.modules.adsb { Some(Arc::new(aircraft::Store::new())) } else { None };
    let vessel_store = if cfg.modules.ais { Some(Arc::new(ais::vessel::VesselStore::new())) } else { None };

    // --- ADS-B module ---
    if let Some(ref store) = aircraft_store {
        let channels = Arc::new(output::OutputChannels::new());
        let base_port: u16 = std::env::var("BASE_PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(39000);

        let ch = channels.clone();
        tokio::spawn(async move { output::start_all(&ch, base_port).await; });

        let s = store.clone();
        tokio::spawn(async move { api::json_builder::run(s).await; });

        let s = store.clone(); let ch = channels.clone();
        tokio::spawn(async move { feed::run_sbs(s, ch).await; });

        let s = store.clone(); let ch = channels.clone();
        tokio::spawn(async move { feed::run_json_pos(s, ch).await; });

        let s = store.clone();
        tokio::spawn(async move { aircraft::reaper(s).await; });

        let s = store.clone(); let ch = channels.clone();
        let port = cfg.adsb.ingest_port;
        tokio::spawn(async move { beast::serve_ingest(s, ch, port).await; });

        info!("adsb: beast ingest on port {}", cfg.adsb.ingest_port);
    }

    // --- AIS module ---
    let nmea_tx = if cfg.modules.ais { Some(nmea_out::new_channel()) } else { None };
    let state_path = std::env::var("AIS_STATE_PATH").unwrap_or_else(|_| "/data/vessels.state".into());

    if let Some(ref store) = vessel_store {
        // Load persisted state
        if std::path::Path::new(&state_path).exists() {
            match store.load(&state_path) {
                Ok(n) => info!("ais: restored {n} vessels from disk"),
                Err(e) => tracing::warn!("ais: failed to load state: {e}"),
            }
        }

        let s = store.clone();
        let host = cfg.ais.nmea_host.clone();
        let tx = nmea_tx.clone();
        tokio::spawn(async move { ais::ingest(s, host, tx).await; });

        let s = store.clone();
        tokio::spawn(async move { ais::vessel::cache_loop(s).await; });

        let s = store.clone();
        tokio::spawn(async move { ais::vessel::reaper(s).await; });

        // Periodic state save (every 5 min)
        let s = store.clone();
        let sp = state_path.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
            loop { interval.tick().await; let _ = s.save(&sp); }
        });

        // NMEA TCP output
        if let Some(ref tx) = nmea_tx {
            let t = tx.clone();
            let nmea_port: u16 = std::env::var("NMEA_OUT_PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(10111);
            tokio::spawn(async move { nmea_out::serve(t, nmea_port).await; });
        }

        info!("ais: nmea ingest from {}, nmea output on :10111", cfg.ais.nmea_host);
    }

    // Graceful shutdown — save state
    let vs_for_shutdown = vessel_store.clone();
    let sp_for_shutdown = state_path.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        if let Some(ref store) = vs_for_shutdown {
            info!("shutting down — saving vessel state");
            let _ = store.save(&sp_for_shutdown);
        }
        std::process::exit(0);
    });

    // --- HTTP API ---
    api::serve(aircraft_store, vessel_store, cfg).await;
}
