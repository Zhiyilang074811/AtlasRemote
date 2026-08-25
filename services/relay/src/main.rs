//! Atlas Relay Server v2.0 - Enhanced with TURN coordination
//!
//! Manages P2P and relay connections for internet access.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tracing::{info, warn, error};

struct ClientEntry {
    rx: BufReader<tokio::net::tcp::OwnedReadHalf>,
    tx: Arc<Mutex<tokio::net::tcp::OwnedWriteHalf>>,
    device_id: String,
}

struct RelayState {
    clients: HashMap<String, ClientEntry>,
}

impl RelayState {
    fn new() -> Self { Self { clients: HashMap::new() } }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tracing_subscriber::{registry, fmt, EnvFilter};
    
    registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let port = std::env::var("ATLAS_RELAY_PORT").unwrap_or_else(|_| "9091".to_string());
    let addr = format!("0.0.0.0:{}", port);
    info!("Atlas Relay v2.0 (TURN-capable) on {}", addr);

    let state = Arc::new(Mutex::new(RelayState::new()));
    let listener = TcpListener::bind(&addr).await?;

    // Print help
    info!("Usage:");
    info!("  HOST: Register as host (remote control target)");
    info!("  CLIENT:<device_id>: Connect as client to control host");
    info!("  TURN:Allocate: Request TURN relay allocation");
    
    loop {
        let (stream, peer) = listener.accept().await?;
        info!("New connection: {}", peer);
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_client(stream, peer, state).await {
                error!("Handler error: {}", e);
            }
        });
    }
}

async fn handle_client(
    stream: TcpStream,
    peer: SocketAddr,
    state: Arc<Mutex<RelayState>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (read_half, write_half) = stream.into_split();
    let mut rx = BufReader::new(read_half);
    
    // Read first line to identify client type
    let mut first_line = vec![0u8; 256];
    let n = tokio::time::timeout(Duration::from_secs(10), rx.read(&mut first_line))
        .await
        .map_err(|_| "timeout")??;
    if n == 0 { return Ok(()); }
    
    let first = String::from_utf8_lossy(&first_line[..n]).trim().to_string();
    info!("Client type: {}", first);

    if first.starts_with("HOST") {
        handle_host(stream, write_half, state).await
    } else if first.starts_with("CLIENT:") {
        let device_id = &first[7..];
        handle_client_conn(device_id, stream, write_half, state).await
    } else if first.starts_with("TURN:") {
        handle_turn_request(first, stream, state).await
    } else {
        // Default to host
        handle_host(stream, write_half, state).await
    }
}

async fn handle_host(
    stream: TcpStream,
    write_half: tokio::net::tcp::OwnedWriteHalf,
    state: Arc<Mutex<RelayState>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (read_half, _) = stream.into_split();
    let tx = Arc::new(Mutex::new(write_half));
    let mut rx = BufReader::new(read_half);

    let host_id = format!("host_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_millis());

    {
        let mut s = state.lock().await;
        s.clients.insert(host_id.clone(), ClientEntry {
            rx,
            tx: tx.clone(),
            device_id: host_id.clone(),
        });
    }
    info!("Host {} registered", host_id);

    // Relay loop: host -> all clients
    let mut buf = vec![0u8; 65536];
    loop {
        let n = {
            let mut s = state.lock().await;
            if let Some(entry) = s.clients.get_mut(&host_id) {
                entry.rx.read(&mut buf).await
            } else {
                break;
            }
        };
        
        let n = match n {
            Ok(n) => n,
            Err(_) => break,
        };
        if n == 0 { break; }

        let data = buf[..n].to_vec();
        {
            let s = state.lock().await;
            for (cid, centry) in s.clients.iter() {
                if cid.starts_with("client_") && cid != &host_id {
                    let mut cw = centry.tx.lock().await;
                    if let Err(e) = cw.write_all(&data).await {
                        warn!("Failed to relay to {}: {}", cid, e);
                    }
                }
            }
        }
    }

    {
        let mut s = state.lock().await;
        s.clients.remove(&host_id);
    }
    info!("Host {} disconnected", host_id);
    Ok(())
}

async fn handle_client_conn(
    device_id: &str,
    stream: TcpStream,
    write_half: tokio::net::tcp::OwnedWriteHalf,
    state: Arc<Mutex<RelayState>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (read_half, _) = stream.into_split();
    let tx = Arc::new(Mutex::new(write_half));
    let mut rx = BufReader::new(read_half);

    let client_id = format!("client_{}", device_id);

    {
        let mut s = state.lock().await;
        s.clients.insert(client_id.clone(), ClientEntry {
            rx,
            tx: tx.clone(),
            device_id: device_id.to_string(),
        });
    }
    info!("Client {} registered", client_id);

    // Relay loop: client -> all hosts
    let mut buf = vec![0u8; 65536];
    loop {
        let n = {
            let mut s = state.lock().await;
            if let Some(entry) = s.clients.get_mut(&client_id) {
                entry.rx.read(&mut buf).await
            } else {
                break;
            }
        };

        let n = match n {
            Ok(n) => n,
            Err(_) => break,
        };
        if n == 0 { break; }

        let data = buf[..n].to_vec();
        {
            let s = state.lock().await;
            for (eid, eentry) in s.clients.iter() {
                if eid.starts_with("host_") && eid != &client_id {
                    let mut hw = eentry.tx.lock().await;
                    if let Err(e) = hw.write_all(&data).await {
                        warn!("Failed to relay to {}: {}", eid, e);
                    }
                }
            }
        }
    }

    {
        let mut s = state.lock().await;
        s.clients.remove(&client_id);
    }
    info!("Client {} disconnected", client_id);
    Ok(())
}

async fn handle_turn_request(
    _request: String,
    _stream: TcpStream,
    _state: Arc<Mutex<RelayState>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // TODO: Implement TURN protocol (RFC 5766)
    warn!("TURN allocation requested (not yet implemented)");
    Ok(())
}
