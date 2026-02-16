use std::sync::Arc;

use agent_orchestrator::{Orchestrator, OrchestratorConfig};
use anyhow::Context;
use axum::{Router, routing::get};
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing()?;

    info!("starting jisi-code server");
    info!("loading orchestrator config from agents.toml");
    let config = OrchestratorConfig::from_file("agents.toml")
        .context("failed to load orchestrator config from agents.toml")?;

    let orchestrator = Orchestrator::new(config).context("failed to initialize orchestrator")?;
    let agents = orchestrator.available_agents();

    if agents.is_empty() {
        warn!("no enabled agents are available");
    } else {
        info!(count = agents.len(), "available agents loaded");
        for agent in &agents {
            info!(
                id = %agent.id,
                display_name = %agent.display_name,
                agent_type = ?agent.agent_type,
                enabled = agent.enabled,
                "agent available"
            );
        }
    }

    let orchestrator = Arc::new(orchestrator);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/ws", get(agent_orchestrator::ws_api::websocket_handler))
        .layer(cors)
        .with_state(orchestrator);

    let bind_addr = "127.0.0.1:3001";
    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .with_context(|| format!("failed to bind to {}", bind_addr))?;
    info!(addr = %bind_addr, "server listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server error")?;

    info!("server shutdown complete");
    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install Ctrl+C handler");
    info!("shutdown signal received");
}

fn init_tracing() -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env().or_else(|_| EnvFilter::try_new("info"))?;

    tracing_subscriber::fmt().with_env_filter(env_filter).init();
    Ok(())
}
