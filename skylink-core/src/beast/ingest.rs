use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tracing::{info, warn};

use crate::beast::parser;
use crate::decode;
use crate::state::AircraftStore;

pub async fn serve(store: Arc<AircraftStore>, port: u16) {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .expect("failed to bind beast ingest port");

    info!("Beast ingest listening on :{}", port);

    loop {
        let (socket, addr) = match listener.accept().await {
            Ok(s) => s,
            Err(e) => { warn!("accept error: {}", e); continue; }
        };

        let store = store.clone();
        tokio::spawn(async move {
            info!("feeder connected: {}", addr);
            handle_feeder(socket, store).await;
            info!("feeder disconnected: {}", addr);
        });
    }
}

async fn handle_feeder(mut socket: tokio::net::TcpStream, store: Arc<AircraftStore>) {
    let mut buf = vec![0u8; 64 * 1024];
    let mut carry = Vec::new(); // leftover bytes from previous read

    loop {
        let n = match socket.read(&mut buf).await {
            Ok(0) => return, // EOF
            Ok(n) => n,
            Err(_) => return,
        };

        // Prepend carry from last iteration
        let mut data = Vec::with_capacity(carry.len() + n);
        data.extend_from_slice(&carry);
        data.extend_from_slice(&buf[..n]);

        let (frames, consumed) = parser::extract_frames(&data);

        // Save unconsumed bytes for next read
        carry = data[consumed..].to_vec();

        for frame in frames {
            if frame.msg_type == b'2' || frame.msg_type == b'3' {
                decode::process_frame(&frame, &store);
            }
        }
    }
}
