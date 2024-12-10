use std::env;

use tokio::spawn;
use udrome::{api::serve, config::Config, indexer::Indexer};
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
