mod chunking;
mod commands;
mod embedding;
mod error;
mod prelude;
mod scanner;
mod storage;
mod utils;

use clap::Parser;
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

use commands::{Args, Command, Commands};
use prelude::*;

/// Codebase scanner that uses Tree-sitter to parse code and prepare it for RAG
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing based on verbosity
    let log_level = match Args::parse().verbose {
        0 => tracing::Level::INFO,
        1 => tracing::Level::DEBUG,
        _ => tracing::Level::TRACE,
    };

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_timer(tracing_subscriber::fmt::time::time())
        .init();

    let args = Args::parse();

    match args.command {
        Commands::Scan(cmd) => cmd.execute().await,
        Commands::Query(cmd) => cmd.execute().await,
    }
}
