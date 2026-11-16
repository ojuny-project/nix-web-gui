mod auth;
mod config;
mod http;
mod rpc;
mod worker;
mod worker_process;

use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use nix_web_gui_engine::theme::ThemeStore;
use tokio::net::TcpListener;

use crate::auth::session::SessionStore;
use crate::config::{Cli, Command, PopoutArgs, ServeArgs};
use crate::http::{AppState, EngineMode};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "nix_web_gui_server=info,tower_http=warn".into()),
        )
        .init();

    match Cli::parse().command {
        Command::Serve(args) => serve(args).await,
        Command::Popout(args) => popout(args).await,
        Command::Worker => worker::run_stdio(),
    }
}

async fn serve(args: ServeArgs) -> Result<(), Box<dyn Error>> {
    let config = Arc::new(args.into_config());
    let listener = TcpListener::bind(config.listen).await?;
    let address = listener.local_addr()?;
    tracing::info!("listening on http://{address}");
    eprintln!("nix-web-gui listening on http://{address}");

    let state = build_state(config.clone());
    let cleanup_state = state.clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(300));
        loop {
            tick.tick().await;
            cleanup_state.sessions.purge_expired();
            http::purge_stale_engine(&cleanup_state).await;
        }
    });
    axum::serve(listener, http::router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn popout(args: PopoutArgs) -> Result<(), Box<dyn Error>> {
    let listen = match args.port {
        Some(port) => format!("127.0.0.1:{port}").parse()?,
        None => "127.0.0.1:0".parse()?,
    };
    let serve_args = ServeArgs {
        listen,
        static_dir: None,
        theme_file: None,
        flake: args.flake,
        page: args.page,
        pam_service: "nix-web-gui".to_string(),
        allowed_group: Vec::new(),
        dev_auth: false,
        dev_password: None,
        session_ttl_secs: 43_200,
        max_sessions: 16,
        secure_cookie: false,
    };
    let config = Arc::new(serve_args.into_config());
    let listener = TcpListener::bind(config.listen).await?;
    let address = listener.local_addr()?;
    let url = format!("http://{address}");
    eprintln!("nix-web-gui popout on {url}");

    if std::env::var_os("NIX_WEB_GUI_NO_OPEN").is_none() {
        let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
    }

    let state = build_state(config);
    axum::serve(listener, http::router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

fn build_state(config: Arc<config::AppConfig>) -> AppState {
    let theme = ThemeStore::new(config.theme_file.clone());
    let theme_task = theme.clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(2));
        loop {
            tick.tick().await;
            theme_task.refresh();
        }
    });

    let engine = if config.dev_auth {
        EngineMode::InProcess(Arc::new(tokio::sync::Mutex::new(
            std::collections::HashMap::new(),
        )))
    } else {
        EngineMode::Workers(Arc::new(tokio::sync::Mutex::new(
            std::collections::HashMap::new(),
        )))
    };

    let sessions = Arc::new(SessionStore::default());
    let (progress, _) = tokio::sync::broadcast::channel(256);

    AppState {
        config,
        sessions,
        theme,
        engine,
        throttle: Arc::new(crate::auth::LoginThrottle::new()),
        progress,
        pids: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
    }
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
