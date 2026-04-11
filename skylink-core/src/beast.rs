/// Beast binary protocol: TCP ingest + frame parser

use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tracing::{info, warn};

use crate::aircraft::Store;
use crate::mode_s;
use crate::output::OutputChannels;

pub struct BeastFrame {
    pub signal: u8,
    pub payload: Vec<u8>,
}

/// Format a u64 receiver ID as UUID-style hex string (readsb format)
fn format_receiver_id(id: u64) -> String {
    format!("{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        (id >> 32) as u32,
        (id >> 16) as u16 & 0xffff,
        id as u16,
        0u16, 0u64)
}

/// Extract Beast frames from buffer, also parsing 0x1a 0xe3 receiver ID messages.
/// Returns (frames, bytes_consumed, Option<new_receiver_id>).
pub fn extract_frames(buf: &[u8]) -> (Vec<BeastFrame>, usize, Option<u64>) {
    let mut frames = Vec::new();
    let mut pos = 0;
    let mut receiver_id: Option<u64> = None;

    while pos < buf.len() {
        if buf[pos] != 0x1a { pos += 1; continue; }
        if pos + 1 >= buf.len() { break; }

        let msg_type = buf[pos + 1];

        // 0x1a 0xe3: receiver ID message (8 bytes, escaped)
        if msg_type == 0xe3 {
            let start = pos;
            pos += 2;
            let mut rid: u64 = 0;
            let mut ok = true;
            for _ in 0..8 {
                if pos >= buf.len() { ok = false; break; }
                let mut b = buf[pos]; pos += 1;
                if b == 0x1a {
                    if pos >= buf.len() { ok = false; break; }
                    b = buf[pos]; pos += 1;
                    if b != 0x1a { ok = false; break; }
                }
                rid = rid << 8 | (b as u64);
            }
            if !ok { pos = start; break; }
            if rid != 0 { receiver_id = Some(rid); }
            continue;
        }

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
    (frames, pos, receiver_id)
}

pub async fn serve_ingest(store: Arc<Store>, channels: Arc<OutputChannels>, port: u16) {
    if let Ok(upstream) = std::env::var("BEAST_CONNECT") {
        tokio::spawn({
            let store = store.clone();
            let ch = channels.clone();
            async move { connect_upstream(store, ch, upstream).await; }
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
        let ch = channels.clone();
        let addr_str = addr.to_string();
        tokio::spawn(async move {
            info!("feeder connected: {}", addr);
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64();
            // Use addr as initial UUID; will be replaced if feeder sends 0xe3 receiver ID
            let initial_uuid = format!("feeder-{}", addr_str.replace([':', '.', '[', ']'], "-"));
            store.clients.write().push(crate::aircraft::Receiver::new(initial_uuid.clone(), addr_str.clone(), now));
            handle_feeder(socket, store.clone(), ch, initial_uuid.clone()).await;
            store.clients.write().retain(|r| r.addr != addr_str);
            info!("feeder disconnected: {}", addr);
        });
    }
}

async fn connect_upstream(store: Arc<Store>, channels: Arc<OutputChannels>, addr: String) {
    loop {
        info!("connecting to upstream Beast: {}", addr);
        match tokio::net::TcpStream::connect(&addr).await {
            Ok(socket) => {
                info!("upstream connected: {}", addr);
                let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64();
                let initial_uuid = format!("upstream-{}", addr.replace([':', '.', '[', ']'], "-"));
                store.clients.write().push(crate::aircraft::Receiver::new(initial_uuid.clone(), addr.clone(), now));
                handle_feeder(socket, store.clone(), channels.clone(), initial_uuid.clone()).await;
                store.clients.write().retain(|r| r.addr != addr);
                warn!("upstream disconnected: {}", addr);
            }
            Err(e) => warn!("upstream connect failed: {} — {}", addr, e),
        }
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}

async fn handle_feeder(mut socket: tokio::net::TcpStream, store: Arc<Store>, channels: Arc<OutputChannels>, initial_uuid: String) {
    let mut buf = vec![0u8; 64 * 1024];
    let mut carry = Vec::new();
    let addr = { store.clients.read().iter().find(|r| r.uuid == initial_uuid).map(|r| r.addr.clone()).unwrap_or_default() };
    let mut current_uuid = initial_uuid;

    loop {
        let n = match socket.read(&mut buf).await {
            Ok(0) | Err(_) => return,
            Ok(n) => n,
        };

        // Forward raw Beast bytes to beast output subscribers
        let _ = channels.beast.send(buf[..n].to_vec());

        let mut data = Vec::with_capacity(carry.len() + n);
        data.extend_from_slice(&carry);
        data.extend_from_slice(&buf[..n]);

        let (frames, consumed, new_rid) = extract_frames(&data);
        carry = data[consumed..].to_vec();

        // If we got a receiver ID from the stream, update the UUID
        if let Some(rid) = new_rid {
            let new_uuid = format_receiver_id(rid);
            if new_uuid != current_uuid {
                let mut receivers = store.clients.write();
                if let Some(r) = receivers.iter_mut().find(|r| r.uuid == current_uuid) {
                    info!("feeder {} receiver ID: {}", addr, new_uuid);
                    r.uuid = new_uuid.clone();
                }
                current_uuid = new_uuid;
            }
        }

        let mut positions: Vec<(f64, f64)> = Vec::new();
        for frame in frames {
            if let Some(msg) = mode_s::decode(&frame.payload) {
                let raw_line = frame.payload.iter()
                    .map(|b| format!("{:02X}", b))
                    .collect::<String>() + "\n";
                let _ = channels.raw.send(raw_line.into_bytes());

                let had_pos = msg.cpr_lat.is_some() && msg.cpr_lon.is_some();
                store.update_from_message(&msg, frame.signal);
                if had_pos {
                    if let Some(entry) = store.map.get(&msg.icao) {
                        if let (Some(lat), Some(lon)) = (entry.value().lat, entry.value().lon) {
                            positions.push((lat, lon));
                        }
                    }
                }
            }
        }

        if !positions.is_empty() {
            let mut receivers = store.clients.write();
            if let Some(r) = receivers.iter_mut().find(|r| r.uuid == current_uuid) {
                for (lat, lon) in &positions {
                    r.record_position(*lat, *lon);
                }
            }
        }
    }
}
