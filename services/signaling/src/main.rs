//! Atlas Signaling Server - Connection coordination

use tracing::info;
use tracing_subscriber::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Atlas Remote Signaling starting...");
    // TODO: Implement signaling logic

    Ok(())
}
