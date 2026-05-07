mod api;
mod auth;
mod models;
mod player;
mod ui;

use anyhow::Result;
use ui::app::run_app;

#[tokio::main]
async fn main() -> Result<()> {
    // Enable ANSI colors on Windows
    #[cfg(windows)]
    let _ = ansi_term::enable_ansi_support();

    run_app().await
}
