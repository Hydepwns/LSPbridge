mod core;
mod capture;
mod privacy;
mod format;
mod export;
mod cli;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    cli::run_cli().await
}
