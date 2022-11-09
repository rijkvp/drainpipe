mod config;
mod daemon;
mod db;
mod dl;
mod error;
mod file;
mod gui;

use owo_colors::OwoColorize;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

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
