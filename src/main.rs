use anyhow::Result;
use lsp_bridge::cli;

#[tokio::main]
async fn main() -> Result<()> {
    cli::run_cli().await
}
