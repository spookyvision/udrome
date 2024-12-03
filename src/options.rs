use std::path::PathBuf;

use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Path to your music collection
    #[arg(short, long)]
    pub media_path: PathBuf,

    /// Path that will contain `udrome.sqlite`
    #[arg(short, long)]
    pub db_path: PathBuf,

    /// Shall the indexer skip files with metadata?
    #[arg(short, long, default_value_t = true)]
    pub skip_tagged: bool,
}
