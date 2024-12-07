use std::env;

use camino::Utf8PathBuf;
use tokio::spawn;
use tracing::debug;
use udrome::{
    api::serve,
    config::{self, Config},
    indexer::Indexer,
};
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config = Config::new(env::args().nth(1))?;

    let ixr = Indexer::new(&config).await?;
    let db = ixr.db();
    spawn(async move { ixr.run().await });

    serve(db, &config).await;
    Ok(())
}
