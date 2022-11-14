mod config;
mod daemon;
mod db;
mod dl;
mod error;
mod file;
mod gui;
mod media;
mod source;

use owo_colors::OwoColorize;
use tracing_subscriber::{filter::EnvFilter, fmt, prelude::*};

#[tokio::main]
async fn main() {
    let filter = EnvFilter::builder()
        .with_default_directive("drainpipe=INFO".parse().unwrap())
        .with_env_var("DRAINPIPE_LOG")
        .from_env()
        .unwrap();
    tracing_subscriber::registry()
        .with(fmt::layer().with_target(false))
        .with(filter)
        .init();

    let d = match daemon::Daemon::start().await {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}: {}", "Failed to start daemon".red().bold(), e);
            return;
        }
    };
    if let Err(e) = d.run().await {
        eprintln!("{}: {}", "Daemon error".red().bold(), e);
    }
}
