pub mod nmea;
pub mod decoder;
pub mod vessel;

use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpStream;
use tracing::{info, warn};
use vessel::VesselStore;

/// Connect to AIS-catcher NMEA TCP output and feed vessel store
pub async fn ingest(store: Arc<VesselStore>, host: String) {
    loop {
        info!("ais: connecting to {host}");
        match TcpStream::connect(&host).await {
            Ok(stream) => {
                info!("ais: connected to {host}");
                let mut reader = BufReader::new(stream);
                let mut line = String::new();
                let mut collector = nmea::NmeaCollector::new();

                loop {
                    line.clear();
                    match reader.read_line(&mut line).await {
                        Ok(0) => { warn!("ais: connection closed"); break; }
                        Ok(_) => {
                            if let Some(bits) = collector.feed(&line) {
                                if let Some(update) = decoder::decode(&bits) {
                                    store.update(update);
                                }
                            }
                        }
                        Err(e) => { warn!("ais: read error: {e}"); break; }
                    }
                }
            }
            Err(e) => { warn!("ais: connect failed: {e}"); }
        }
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}
