use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tracing::{info, warn};

use crate::state::AircraftStore;

/// Beast output: accepts subscribers, broadcasts raw Beast frames
/// For now, sends heartbeat to keep connections alive.
/// Full Beast re-encoding from state is a future enhancement.
pub async fn serve(_store: Arc<AircraftStore>, port: u16) {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .expect("failed to bind beast output port");

    info!("Beast output listening on :{}", port);

    let (tx, _) = broadcast::channel::<Vec<u8>>(1024);

    // Heartbeat sender
    let tx_hb = tx.clone();
    tokio::spawn(async move {
        let heartbeat: Vec<u8> = vec![0x1a, 0x31, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            let _ = tx_hb.send(heartbeat.clone());
        }
    });

    loop {
        let (mut socket, addr) = match listener.accept().await {
            Ok(s) => s,
            Err(e) => { warn!("accept error: {}", e); continue; }
        };

        let mut rx = tx.subscribe();
        info!("beast output subscriber: {}", addr);

        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(data) => {
                        if socket.write_all(&data).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            info!("beast output subscriber disconnected: {}", addr);
        });
    }
}
