use axum::{Router, routing::get};
use std::net::SocketAddr;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "jisi_code_server=info,tower_http=info".into()),
        )
        .init();

    let app = Router::new().route("/health", get(health));
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    info!(%addr, "starting server");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health() -> &'static str {
    "ok"
}
