use std::path::Path;
use std::sync::Arc;

use agent_orchestrator::{Orchestrator, OrchestratorConfig};
use anyhow::Context;
use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

mod api;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing()?;

    info!("starting jisi-code server");
    let config_path = resolve_orchestrator_config_path();
    info!(path = %config_path, "loading orchestrator config");
    let config = OrchestratorConfig::from_file(&config_path)
        .with_context(|| format!("failed to load orchestrator config from {}", config_path))?;

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

    // 创建统一的应用状态
    let app_state = Arc::new(api::AppState::new(orchestrator));

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // 文件系统 API 路由
    let filesystem_router = api::create_filesystem_router();

    let app = Router::new()
        // WebSocket 路由
        .route("/ws", axum::routing::get(api::websocket_handler))
        // 文件系统 API 路由
        .merge(filesystem_router)
        .with_state(app_state)
        .layer(cors);

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

fn resolve_orchestrator_config_path() -> String {
    if let Ok(path) = std::env::var("JISI_AGENTS_CONFIG") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    let local_path = "agents.local.toml";
    if Path::new(local_path).exists() {
        return local_path.to_string();
    }

    "agents.toml".to_string()
}
