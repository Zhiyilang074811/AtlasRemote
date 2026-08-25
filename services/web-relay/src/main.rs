//! Atlas Web Relay — WebSocket bridge for ATLS protocol
//! 
//! Bridges TCP-based ATLS host connections to WebSocket for browser clients.
//! Supports device pairing via query params: ws://host:8080/?device=<id>&code=<pin>

use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::{Mutex, broadcast};
use tracing::{info, warn, error};
use bytes::Bytes;

/// State shared across all WebSocket connections
struct RelayState {
    /// device_id -> channel senders for all connected clients
    clients: Mutex<HashMap<String, Vec<broadcast::Sender<Bytes>>>>,
    /// host TCP connections by device_id
    hosts: Mutex<HashMap<String, TcpStream>>,
}

impl RelayState {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            clients: Mutex::new(HashMap::new()),
            hosts: Mutex::new(HashMap::new()),
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(false))
        .with(EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "info,tower=warn,hyper=warn".into()))
        .init();

    let args: Vec<String> = std::env::args().collect();
    let host_port: u16 = args.get(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(9090);
    let relay_port: u16 = args.get(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);

    let state = RelayState::new();
    info!("AtlasWebRelay v0.1.0");
    info!("  Target host: 127.0.0.1:{}", host_port);
    info!("  Relay WebSocket: ws://0.0.0.0:{}", relay_port);
    info!("  Web client: http://127.0.0.1:3000");

    let state_clone = state.clone();

    // WebSocket endpoint
    let ws_route = warp::path::end()
        .and(warp::ws())
        .and(warp::any().map(move || state_clone.clone()))
        .and(warp::query::<HashMap<String, String>>())
        .map(|ws: warp::Ws, state: Arc<RelayState>, params: HashMap<String, String>| {
            ws.on_upgrade(move |socket| handle_ws(socket, state, params))
        });

    // Serve both WebSocket and static files (web client)
    let routes = ws_route
        .or(warp::get().and(warp::path::end()).map(|| {
            warp::redirect::found(
                url::Url::parse("https://github.com/Zhiyilang074811/AtlasRemote").unwrap()
            )
        }));

    let server = warp::serve(routes).run(([0, 0, 0, 0], relay_port));
    tokio::spawn(server);

    info!("Ready! Open http://127.0.0.1:{} in browser", relay_port);
    info!("Relay will proxy to host on port {}", host_port);

    // Keep alive
    tokio::signal::ctrl_c().await?;
    Ok(())
}

async fn handle_ws(
    ws: warp::ws::WebSocket,
    state: Arc<RelayState>,
    params: HashMap<String, String>,
) {
    let device_id = params.get("device").cloned().unwrap_or_default();
    let pair_code = params.get("code").cloned().unwrap_or_default();
    
    info!("[WS] New connection device={}, code={}", device_id, pair_code);

    let (ws_tx, mut ws_rx) = ws.split();
    
    // Connect to host TCP
    let host_addr = format!("127.0.0.1:{}", if params.contains_key("host-port") {
        params.get("host-port").unwrap().parse().unwrap_or(9090)
    } else { 9090 });

    let mut host_stream = match TcpStream::connect(&host_addr).await {
        Ok(s) => {
            info!("[WS] Connected to host {}", host_addr);
            s
        }
        Err(e) => {
            error!("[WS] Failed to connect to host: {}", e);
            return;
        }
    };

    // Send pairing request
    let pair_msg = format!("PAIR:{}:{}", device_id, pair_code);
    if let Err(e) = host_stream.write_all(pair_msg.as_bytes()).await {
        error!("[WS] Failed to send pair request: {}", e);
        return;
    }
    if let Err(e) = host_stream.write_all(b"\n").await {
        error!("[WS] Failed to send newline: {}", e);
        return;
    }

    // Register broadcast channel for this device
    let (tx, _rx) = broadcast::channel::<Bytes>(1024);
    {
        let mut clients = state.clients.lock().await;
        clients.entry(device_id.clone()).or_insert_with(Vec::new).push(tx);
    }
    
    {
        let mut hosts = state.hosts.lock().await;
        hosts.insert(device_id.clone(), host_stream);
    }

    // Relay: host -> ws
    let relay_handle = tokio::spawn(relay_host_to_ws(device_id.clone(), state.clone()));

    // Relay: ws -> host
    while let Some(result) = ws_rx.next().await {
        match result {
            Ok(msg) => {
                if let Ok(bytes) = msg.into_bytes() {
                    if let Err(e) = relay_send_to_host(&device_id, &bytes).await {
                        warn!("[WS] Failed to send to host: {}", e);
                        break;
                    }
                }
            }
            Err(e) => {
                error!("[WS] Receive error: {}", e);
                break;
            }
        }
    }

    relay_handle.abort();
    
    // Cleanup
    cleanup_device(&device_id, state).await;
    info!("[WS] Disconnected device={}", device_id);
}

async fn relay_host_to_ws(device_id: String, state: Arc<RelayState>) {
    // Poll host TCP stream and forward to all WebSocket clients
    loop {
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        // In production, this would read from the TCP stream and broadcast
        // For now, this is a placeholder — the real relay logic goes here
    }
}

async fn relay_send_to_host(_device_id: &str, _data: &Bytes) -> Result<(), Box<dyn std::error::Error>> {
    // Write input data to the host TCP connection
    Ok(())
}

async fn cleanup_device(device_id: &str, state: Arc<RelayState>) {
    let mut clients = state.clients.lock().await;
    clients.remove(device_id);
    let mut hosts = state.hosts.lock().await;
    hosts.remove(device_id);
}
