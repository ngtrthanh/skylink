use std::sync::Arc;
use axum::{extract::State, response::Json, routing::get, Router};
use tower_http::cors::CorsLayer;
use tracing::info;

use crate::state::AircraftStore;

fn now_secs() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64()
}

async fn aircraft_json(State(store): State<Arc<AircraftStore>>) -> Json<serde_json::Value> {
    let now = now_secs();
    Json(store.aircraft_json(now))
}

async fn stats(State(store): State<Arc<AircraftStore>>) -> Json<serde_json::Value> {
    let total = store.map.len();
    let with_pos = store.map.iter().filter(|e| e.value().lat.is_some()).count();
    Json(serde_json::json!({
        "aircraft_total": total,
        "aircraft_with_pos": with_pos,
    }))
}

pub async fn serve(store: Arc<AircraftStore>, port: u16) {
    let app = Router::new()
        .route("/data/aircraft.json", get(aircraft_json))
        .route("/stats", get(stats))
        .layer(CorsLayer::permissive())
        .with_state(store);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .expect("failed to bind API port");

    info!("API serving on :{}", port);
    axum::serve(listener, app).await.unwrap();
}
