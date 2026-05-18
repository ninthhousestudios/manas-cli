use std::sync::Arc;

use anyhow::{Context, Result};
use axum::Router;
use axum::routing::post;
use tokio::net::TcpListener;

use crate::config::ManasConfig;
use crate::hub::mcp::{self, HubState};

pub async fn run(port: u16) -> Result<()> {
    let config = ManasConfig::load()?;

    eprintln!("manas serve listening on 0.0.0.0:{port}");
    eprintln!("  chitta: {}", config.chitta_url);
    eprintln!("  yojana: {}", config.yojana_url);

    let state = Arc::new(HubState {
        chitta_url: config.chitta_url,
        yojana_url: config.yojana_url,
    });

    let app = Router::new()
        .route("/mcp", post(mcp::handle_mcp))
        .with_state(state);

    let addr = format!("0.0.0.0:{port}");

    let listener = TcpListener::bind(&addr)
        .await
        .context(format!("failed to bind {addr}"))?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server error")?;

    eprintln!("manas serve stopped");
    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for ctrl-c");
    eprintln!("\nshutting down...");
}
