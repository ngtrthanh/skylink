/// NMEA TCP output — forward raw NMEA sentences to downstream consumers
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tracing::{info, warn};

pub type NmeaSender = broadcast::Sender<String>;

pub fn new_channel() -> NmeaSender {
    broadcast::channel(1024).0
}

/// Accept TCP clients and forward NMEA lines
pub async fn serve(tx: NmeaSender, port: u16) {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await
        .expect("failed to bind NMEA output port");
    info!("NMEA output on :{}", port);

    loop {
        if let Ok((mut socket, addr)) = listener.accept().await {
            let mut rx = tx.subscribe();
            info!("nmea-out: client connected from {addr}");
            tokio::spawn(async move {
                loop {
                    match rx.recv().await {
                        Ok(line) => {
                            if socket.write_all(line.as_bytes()).await.is_err() { break; }
                            if socket.write_all(b"\r\n").await.is_err() { break; }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            warn!("nmea-out: client lagged {n} messages");
                        }
                        Err(_) => break,
                    }
                }
                info!("nmea-out: client disconnected {addr}");
            });
        }
    }
}
