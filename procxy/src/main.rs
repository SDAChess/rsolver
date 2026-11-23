//! The `procxy` server binary.

use std::{net::SocketAddr, path::PathBuf};

use clap::Parser;
use procxy::{
    server,
    service::Service,
    store::Store,
    upstream::{self, Upstream},
};

/// Metadata proxy for crates.io.
#[derive(Parser)]
#[command(version)]
struct Args {
    /// Address to listen on.
    #[arg(long, default_value = "127.0.0.1:8080")]
    listen: SocketAddr,
    /// Path of the SQLite database holding fetched manifests.
    #[arg(long, default_value = "procxy.db")]
    db: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    let client = upstream::client()?;
    let service = Service::new(Store::open(&args.db)?, Upstream::new(client.clone()));
    let listener = tokio::net::TcpListener::bind(args.listen).await?;
    tracing::info!(listen = %listener.local_addr()?, db = %args.db.display(), "procxy started");

    axum::serve(listener, server::router(service, client))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
