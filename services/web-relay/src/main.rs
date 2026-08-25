//! Atlas Web Relay - Full ATLS TCP to WebSocket bridge
//! Bridges TCP-based ATLS host connections to WebSocket for browser clients.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{Mutex, mpsc};
use tracing::{info, warn, error};
use warp::{Filter, ws::{Ws, Message}};

const ATLS_MAGIC: [u8; 4] = [0x41, 0x54, 0x4C, 0x53];
const ATLS_FULL_HEADER: usize = 36;
const INPUT_MAGIC: [u8; 4] = [0x49, 0x4E, 0x50, 0x54];
const INPUT_HEADER_SIZE: usize = 13;

struct DeviceState {
    ws_senders: HashMap<String, Vec<mpsc::Sender<Message>>>,
    host_streams: HashMap<String, TcpStream>,
}

impl DeviceState {
    fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self { ws_senders: HashMap::new(), host_streams: HashMap::new() }))
    }
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};
    tracing_subscriber::registry()
        .with(fmt::layer().with_target(false))
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();
    let args: Vec<String> = std::env::args().collect();
    let host_port: u16 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(9090);
    let relay_port: u16 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(8080);
    let state = DeviceState::new();
    info!("AtlasWebRelay v0.2.0 host={} relay={}", host_port, relay_port);
    let state_clone = state.clone();
    let ws_route = warp::path::end()
        .and(warp::ws())
        .and(warp::any().map(move || state_clone.clone()))
        .and(warp::query::<HashMap<String, String>>())
        .map(|ws: Ws, state: Arc<Mutex<DeviceState>>, params| {
            ws.on_upgrade(move |socket| handle_ws(socket, state, params))
        });
    let serve_static = warp::fs::dir("./dist".to_string());
    let routes = ws_route.or(serve_static).or(
        warp::get().and(warp::path::end()).map(|| {
            warp::redirect::found(url::Url::parse("https://github.com/Zhiyilang074811/AtlasRemote").unwrap())
        })
    );
    let server = warp::serve(routes).run(([0,0,0,0], relay_port));
    tokio::spawn(server);
    info!("Ready! http://127.0.0.1:{}", relay_port);
    tokio::signal::ctrl_c().await?;
    Ok(())
}
async fn handle_ws(
    ws: Ws,
    state: Arc<Mutex<DeviceState>>,
    params: HashMap<String, String>,
) {
    let device_id = params.get("device").cloned().unwrap_or_default();
    let pair_code = params.get("code").cloned().unwrap_or_default();
    info!("[WS] New connection device={} code={}", device_id, pair_code);
    let host_port = if params.contains_key("host-port") {
        params.get("host-port").unwrap().parse().unwrap_or(9090)
    } else { 9090 };
    let host_addr = format!("127.0.0.1:{}", host_port);
    let mut host_stream = match TcpStream::connect(&host_addr).await {
        Ok(s) => { info!("[WS] Connected to host {}", host_addr); s },
        Err(e) => { error!("[WS] Failed: {}", e); return; }
    };
    let pair_msg = format!("PAIR:{}:{}\n", device_id, pair_code);
    if let Err(e) = host_stream.write_all(pair_msg.as_bytes()).await {
        error!("[WS] Pair failed: {}", e); return;
    }
    let (ws_tx, mut ws_rx) = mpsc::channel::<Message>(64);
    {
        let mut s = state.lock().await;
        s.ws_senders.entry(device_id.clone()).or_default().push(ws_tx);
        s.host_streams.insert(device_id.clone(), host_stream);
    }
    let relay_handle = tokio::spawn(relay_host_to_ws(device_id.clone(), state.clone()));
    while let Some(msg) = ws_rx.recv().await {
        if msg.is_text() || msg.is_close() {
            if let Ok(text) = msg.to_str() {
                if let Err(e) = send_to_host(&device_id, text, &state).await {
                    warn!("[WS] Input relay failed: {}", e); break;
                }
            }
        }
    }
    relay_handle.abort();
    cleanup_device(&device_id, state).await;
    info!("[WS] Disconnected device={}", device_id);
}
async fn relay_host_to_ws(device_id: String, state: Arc<Mutex<DeviceState>>) {
    let mut read_buf = Vec::new();
    loop {
        let stream = {
            let s = state.lock().await;
            s.host_streams.get(&device_id).cloned()
        };
        let mut stream = match stream {
            Some(s) => s,
            None => { warn!("[RELAY] No host stream for {}", device_id); break; }
        };
        let mut tmp = [0u8; 8192];
        let n = match tokio::time::timeout(
            tokio::time::Duration::from_millis(200),
            stream.read(&mut tmp)
        ).await {
            Ok(Ok(0)) => break,
            Ok(Ok(n)) => n,
            Ok(Err(e)) => { error!("[RELAY] Read err: {}", e); break; }
            Err(_) => continue,
        };
        if n > 0 { read_buf.extend_from_slice(&tmp[..n]); }
        while read_buf.len() >= ATLS_FULL_HEADER {
            if read_buf[0..4] == ATLS_MAGIC {
                let payload_len = u32::from_be_bytes([read_buf[8], read_buf[9], read_buf[10], read_buf[11]]) as usize;
                let total_size = ATLS_FULL_HEADER + payload_len;
                if read_buf.len() >= total_size {
                    let pkt_type = u16::from_be_bytes([read_buf[4], read_buf[5]]);
                    let w = u32::from_be_bytes([read_buf[12], read_buf[13], read_buf[14], read_buf[15]]);
                    let h = u32::from_be_bytes([read_buf[16], read_buf[17], read_buf[18], read_buf[19]]);
                    let codec = u16::from_be_bytes([read_buf[20], read_buf[21]]);
                    info!("[RELAY] Frame: type={} w={} h={} codec={}", pkt_type, w, h, codec);
                    let frame_data = read_buf[..total_size].to_vec();
                    read_buf.drain(..total_size);
                    broadcast_frame(&device_id, &state, frame_data).await;
                    continue;
                }
            }
            if read_buf[0..4] == INPUT_MAGIC && read_buf.len() >= INPUT_HEADER_SIZE {
                let plen = u16::from_le_bytes([read_buf[6], read_buf[7]]) as usize;
                let total = INPUT_HEADER_SIZE + plen;
                if read_buf.len() >= total { read_buf.drain(..total); continue; }
            }
            if let Some(pos) = read_buf.iter().position(|&b| b == b'\n') {
                let line = String::from_utf8_lossy(&read_buf[..pos]).to_string();
                read_buf.drain(..pos + 1);
                info!("[RELAY] Host text: {}", line);
                broadcast_text(&device_id, &state, &line).await;
                continue;
            }
            break;
        }
    }
}
async fn broadcast_frame(device_id: &str, state: &Arc<Mutex<DeviceState>>, data: Vec<u8>) {
    let mut senders = Vec::new();
    {
        let s = state.lock().await;
        if let Some(list) = s.ws_senders.get(device_id) {
            senders.extend(list.iter());
        }
    }
    let msg = Message::binary(data);
    for sender in senders {
        if let Err(e) = sender.send(msg.clone()).await {
            warn!("[RELAY] Frame broadcast failed: {}", e);
        }
    }
}

async fn broadcast_text(device_id: &str, state: &Arc<Mutex<DeviceState>>, text: &str) {
    let mut senders = Vec::new();
    {
        let s = state.lock().await;
        if let Some(list) = s.ws_senders.get(device_id) {
            senders.extend(list.iter());
        }
    }
    let msg = Message::text(text.to_string());
    for sender in senders {
        if let Err(e) = sender.send(msg.clone()).await {
            warn!("[RELAY] Text broadcast failed: {}", e);
        }
    }
}
async fn send_to_host(device_id: &str, text: &str, state: &Arc<Mutex<DeviceState>>) -> Result<(), Box<dyn std::error::Error>> {
    let text = text.trim();
    if text.is_empty() { return Ok(()); }
    let json: serde_json::Value = serde_json::from_str(text)?;
    let msg_type = json.get("type").and_then(|v| v.as_str()).unwrap_or("");
    let cmd = match msg_type {
        "mouse_move" => {
            let x = json.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let y = json.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
            format!("MOUSE_MOVE:{:.6}:{:.6}", x, y)
        }
        "mouse_click" => {
            let button = json.get("button").and_then(|v| v.as_str()).unwrap_or("left");
            let pressed = json.get("pressed").and_then(|v| v.as_bool()).unwrap_or(true);
            let btn = if button == "right" { "2" } else { "1" };
            let st = if pressed { "DOWN" } else { "UP" };
            format!("MOUSE_CLICK:{}:{}", btn, st)
        }
        "wheel" => {
            let delta = json.get("delta").and_then(|v| v.as_i64()).unwrap_or(0);
            format!("SCROLL:{}", delta)
        }
        "key" => {
            let code = json.get("code").and_then(|v| v.as_str()).unwrap_or("");
            let pressed = json.get("pressed").and_then(|v| v.as_bool()).unwrap_or(true);
            let action = if pressed { "DOWN" } else { "UP" };
            format!("KEY_{}:{}", action, code)
        }
        "double_click" => "DOUBLE_CLICK".to_string(),
        "pair_accept" => {
            let code = json.get("code").and_then(|v| v.as_str()).unwrap_or("");
            format!("PAIR_ACCEPT:{}", code)
        }
        _ => { warn!("[RELAY] Unknown input type: {}", msg_type); return Ok(()); }
    };
    let mut stream = {
        let s = state.lock().await;
        s.host_streams.get(device_id).cloned()
    };
    match stream {
        Some(mut s) => {
            s.write_all(cmd.as_bytes()).await?;
            s.write_all(b"\n").await?;
            s.flush().await?;
            info!("[RELAY] -> Host: {}", cmd);
            Ok(())
        }
        None => { warn!("[RELAY] No host stream for {}", device_id); Ok(()) }
    }
}
async fn cleanup_device(device_id: &str, state: Arc<Mutex<DeviceState>>) {
    let mut s = state.lock().await;
    s.ws_senders.remove(device_id);
    s.host_streams.remove(device_id);
}
