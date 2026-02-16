use agent_orchestrator::{Orchestrator, OrchestratorConfig};
use anyhow::Context;
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
        for agent in agents {
            info!(
                id = %agent.id,
                display_name = %agent.display_name,
                agent_type = ?agent.agent_type,
                enabled = agent.enabled,
                "agent available"
            );
        }
    }

    let mut event_stream = orchestrator.subscribe_events();
    info!("subscribed to orchestrator event stream");
    info!("server is ready, press Ctrl+C to shut down");

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("shutdown signal received, stopping server");
                break;
            }
            event = event_stream.recv() => {
                match event {
                    Ok(event) => info!(?event, "orchestrator event"),
                    Err(err) => {
                        warn!(error = %err, "failed to receive orchestrator event");
                        break;
                    }
                }
            }
        }
    }

    info!("server shutdown complete");
    Ok(())
}

fn init_tracing() -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env().or_else(|_| EnvFilter::try_new("info"))?;

    tracing_subscriber::fmt().with_env_filter(env_filter).init();
    Ok(())
}
