mod config;
mod daemon;
mod db;
mod dl;
mod error;
mod file;
mod gui;

use owo_colors::OwoColorize;

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();
    let d = match daemon::Daemon::start().await {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}: {}", "Failed to start daemon".red(), e);
            return;
        }
    };
    if let Err(e) = d.run().await {
        eprintln!("{}: {}", "Daemon error".red(), e);
    }
}
