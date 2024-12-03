use std::env;

use camino::Utf8PathBuf;
use clap::Parser;
use tracing::debug;
use udrome::{api::serve, indexer::Indexer, options};
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = options::Args::parse();
    debug!("{args:?}");

    let ixr = Indexer::new(&args);
    ixr.run().await;

    let db = ixr.into_db();
    serve(db).await;
    Ok(())
}
