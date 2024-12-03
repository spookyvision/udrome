use std::env;

use camino::Utf8PathBuf;
use clap::Parser;
use tokio::spawn;
use tracing::debug;
use udrome::{api::serve, indexer::Indexer, options};
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = options::Args::parse();
    debug!("{args:?}");

    let ixr = Indexer::new(&args).await?;
    let db = ixr.db();
    spawn(async move { ixr.run().await });

    serve(db, args.address).await;
    Ok(())
}
