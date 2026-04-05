use anyhow::{Context, Result};
use clap::Parser;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use yt_plex_common::config::{default_config_path, load_config};
use yt_plex_server::{build_router, create_app_state, worker};

#[derive(Parser)]
#[command(name = "yt-plex", about = "YouTube → Plex download server")]
struct Args {
    #[arg(long, env = "YT_PLEX_CONFIG")]
    config: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let config_path = args.config.unwrap_or_else(default_config_path);

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "warn,yt_plex_server=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = load_config(&config_path)
        .with_context(|| format!("loading config from {config_path}"))?;
    let bind = config.server.bind.clone();

    let state = create_app_state(config, config_path).await?;

    {
        let db = Arc::clone(&state.db);
        let config = Arc::clone(&state.config);
        let hub = state.ws_hub.clone();
        tokio::spawn(async move {
            worker::run(db, config, hub).await;
        });
    }

    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .with_context(|| format!("binding to {bind}"))?;

    tracing::info!("listening on {bind}");
    axum::serve(listener, app).await.context("server error")?;

    Ok(())
}
