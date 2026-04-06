/// Beast binary protocol: TCP ingest + frame parser

use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tracing::{info, warn};

use crate::aircraft::Store;
use crate::mode_s;

pub struct BeastFrame {
    pub signal: u8,
    pub payload: Vec<u8>,
}

/// Extract Beast frames from buffer. Returns (frames, bytes_consumed).
pub fn extract_frames(buf: &[u8]) -> (Vec<BeastFrame>, usize) {
    let mut frames = Vec::new();
    let mut pos = 0;

    while pos < buf.len() {
        if buf[pos] != 0x1a { pos += 1; continue; }
        if pos + 1 >= buf.len() { break; }

        let msg_type = buf[pos + 1];
        let payload_len = match msg_type {
            b'1' => 2, b'2' => 7, b'3' => 14,
            0x1a => { pos += 2; continue; }
            _ => { pos += 2; continue; }
        };

        let start = pos;
        pos += 2;

        // Skip 6-byte timestamp (with escape handling)
        let mut ok = true;
        for _ in 0..6 {
            if pos >= buf.len() { ok = false; break; }
            if buf[pos] == 0x1a { pos += 1; if pos >= buf.len() || buf[pos] != 0x1a { ok = false; break; } }
            pos += 1;
        }
        if !ok { pos = start; break; }

        // Signal byte
        if pos >= buf.len() { pos = start; break; }
        let mut signal = buf[pos]; pos += 1;
        if signal == 0x1a { if pos >= buf.len() { pos = start; break; } signal = buf[pos]; pos += 1; }

        // Payload
        let mut payload = Vec::with_capacity(payload_len);
        let mut pay_ok = true;
        for _ in 0..payload_len {
            if pos >= buf.len() { pay_ok = false; break; }
            let mut b = buf[pos]; pos += 1;
            if b == 0x1a { if pos >= buf.len() { pay_ok = false; break; } b = buf[pos]; pos += 1; if b != 0x1a { pay_ok = false; break; } }
            payload.push(b);
        }
        if !pay_ok { pos = start; break; }

        frames.push(BeastFrame { signal, payload });
    }
    (frames, pos)
}

pub async fn serve_ingest(store: Arc<Store>, port: u16) {
    // Check for BEAST_CONNECT=host,port (client mode — connect to upstream)
    if let Ok(upstream) = std::env::var("BEAST_CONNECT") {
        tokio::spawn({
            let store = store.clone();
            async move { connect_upstream(store, upstream).await; }
        });
    }

    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await
        .expect("failed to bind ingest port");
    info!("Beast ingest on :{}", port);

    loop {
        let (socket, addr) = match listener.accept().await {
            Ok(s) => s, Err(e) => { warn!("accept: {}", e); continue; }
        };
        let store = store.clone();
        tokio::spawn(async move {
            info!("feeder connected: {}", addr);
            handle_feeder(socket, store).await;
            info!("feeder disconnected: {}", addr);
        });
    }
}

async fn connect_upstream(store: Arc<Store>, addr: String) {
    loop {
        info!("connecting to upstream Beast: {}", addr);
        match tokio::net::TcpStream::connect(&addr).await {
            Ok(socket) => {
                info!("upstream connected: {}", addr);
                handle_feeder(socket, store.clone()).await;
                warn!("upstream disconnected: {}", addr);
            }
            Err(e) => warn!("upstream connect failed: {} — {}", addr, e),
        }
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}

async fn handle_feeder(mut socket: tokio::net::TcpStream, store: Arc<Store>) {
    let mut buf = vec![0u8; 64 * 1024];
    let mut carry = Vec::new();

    loop {
        let n = match socket.read(&mut buf).await {
            Ok(0) | Err(_) => return,
            Ok(n) => n,
        };

        let mut data = Vec::with_capacity(carry.len() + n);
        data.extend_from_slice(&carry);
        data.extend_from_slice(&buf[..n]);

        let (frames, consumed) = extract_frames(&data);
        carry = data[consumed..].to_vec();

        for frame in frames {
            if let Some(msg) = mode_s::decode(&frame.payload) {
                store.update_from_message(&msg, frame.signal);
            }
        }
    }
}
