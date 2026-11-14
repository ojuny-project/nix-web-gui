use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "nix-web-gui", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Serve(ServeArgs),
    Popout(PopoutArgs),
    Worker,
}

#[derive(Args, Debug, Clone)]
pub struct ServeArgs {
    #[arg(long, default_value = "127.0.0.1:8787")]
    pub listen: SocketAddr,
    #[arg(long)]
    pub static_dir: Option<PathBuf>,
    #[arg(long)]
    pub theme_file: Option<PathBuf>,
    #[arg(long)]
    pub flake: Option<PathBuf>,
    #[arg(long)]
    pub page: Option<String>,
    #[arg(long, default_value = "nix-web-gui")]
    pub pam_service: String,
    #[arg(long)]
    pub allowed_group: Vec<String>,
    #[arg(long)]
    pub dev_auth: bool,
    #[arg(long)]
    pub dev_password: Option<String>,
    #[arg(long, default_value_t = 43_200)]
    pub session_ttl_secs: u64,
    #[arg(long, default_value_t = 16)]
    pub max_sessions: usize,
    #[arg(long)]
    pub secure_cookie: bool,
}

#[derive(Args, Debug, Clone)]
pub struct PopoutArgs {
    #[arg(long)]
    pub flake: Option<PathBuf>,
    #[arg(long)]
    pub page: Option<String>,
    #[arg(long)]
    pub port: Option<u16>,
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub listen: SocketAddr,
    pub static_dir: PathBuf,
    pub theme_file: PathBuf,
    pub flake: Option<String>,
    pub startup_page: Option<String>,
    pub pam_service: String,
    pub allowed_groups: Vec<String>,
    pub dev_auth: bool,
    pub dev_password: Option<String>,
    pub session_ttl: Duration,
    pub max_sessions: usize,
    pub secure_cookie: bool,
}

impl ServeArgs {
    pub fn into_config(self) -> AppConfig {
        AppConfig {
            listen: self.listen,
            static_dir: self.static_dir.unwrap_or_else(default_static_dir),
            theme_file: self.theme_file.unwrap_or_else(default_theme_file),
            flake: self.flake.map(|path| path.display().to_string()),
            startup_page: self.page,
            pam_service: self.pam_service,
            allowed_groups: self.allowed_group,
            dev_auth: self.dev_auth,
            dev_password: self.dev_password,
            session_ttl: Duration::from_secs(self.session_ttl_secs),
            max_sessions: self.max_sessions,
            secure_cookie: self.secure_cookie,
        }
    }
}

fn default_static_dir() -> PathBuf {
    if let Some(path) = std::env::var_os("NIX_WEB_GUI_STATIC_DIR") {
        return PathBuf::from(path);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/dist")
}

fn default_theme_file() -> PathBuf {
    if let Some(path) = std::env::var_os("NIX_WEB_GUI_THEME_FILE") {
        return PathBuf::from(path);
    }
    PathBuf::from("/etc/nix-web-gui/theme")
}
