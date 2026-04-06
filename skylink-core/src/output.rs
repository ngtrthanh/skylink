use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tracing::{info, warn};


/// All broadcast channels for output services
pub struct OutputChannels {
    pub beast: broadcast::Sender<Vec<u8>>,
    pub beast_reduce: broadcast::Sender<Vec<u8>>,
    pub raw: broadcast::Sender<Vec<u8>>,
    pub sbs: broadcast::Sender<Vec<u8>>,
    pub json_pos: broadcast::Sender<Vec<u8>>,
}

impl OutputChannels {
    pub fn new() -> Self {
        Self {
            beast: broadcast::channel(256).0,
            beast_reduce: broadcast::channel(256).0,
            raw: broadcast::channel(256).0,
            sbs: broadcast::channel(256).0,
            json_pos: broadcast::channel(256).0,
        }
    }
}

/// Generic TCP output: accept connections, send broadcast data
async fn serve_output(name: &'static str, port: u16, tx: broadcast::Sender<Vec<u8>>) {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await
        .expect(&format!("failed to bind {} port {}", name, port));
    info!("{} output on :{}", name, port);

    loop {
        let (mut socket, addr) = match listener.accept().await {
            Ok(s) => s, Err(e) => { warn!("{} accept: {}", name, e); continue; }
        };
        let mut rx = tx.subscribe();
        let n = name;
        tokio::spawn(async move {
            info!("{} subscriber: {}", n, addr);
            loop {
                match rx.recv().await {
                    Ok(data) => {
                        if socket.write_all(&data).await.is_err() { break; }
                    }
                    Err(broadcast::error::RecvError::Lagged(skip)) => {
                        warn!("{} {} lagged, dropped {} msgs", n, addr, skip);
                    }
                    Err(_) => break,
                }
            }
            info!("{} subscriber disconnected: {}", n, addr);
        });
    }
}

/// Start all output listeners
pub async fn start_all(channels: &OutputChannels, base_port: u16) {
    // base_port offsets match readsb convention:
    // +2 = raw, +5 = beast, +6 = beast_reduce, +47 = json_pos, +3 = sbs
    let raw_port = base_port + 2;      // 30002
    let sbs_port = base_port + 3;      // 30003
    let beast_port = base_port + 5;    // 30005
    let reduce_port = base_port + 6;   // 30006
    let json_port = base_port + 47;    // 30047

    tokio::spawn(serve_output("Beast", beast_port, channels.beast.clone()));
    tokio::spawn(serve_output("BeastReduce", reduce_port, channels.beast_reduce.clone()));
    tokio::spawn(serve_output("Raw", raw_port, channels.raw.clone()));
    tokio::spawn(serve_output("SBS", sbs_port, channels.sbs.clone()));
    tokio::spawn(serve_output("JSON-pos", json_port, channels.json_pos.clone()));
}
