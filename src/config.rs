use std::{fs::File, io::Read};

use serde::Deserialize;
use thiserror::Error;
use tracing::info;

#[derive(Deserialize)]
pub struct Config {
    pub system: System,
    pub media: Media,
}

#[derive(Deserialize)]
pub struct System {
    pub data_path: String,
    pub cache_mb: u16,
    pub bind_addr: String,
    pub dev: bool,
}

#[derive(Deserialize)]
pub struct Media {
    pub paths: Vec<camino::Utf8PathBuf>,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
}

const DEFAULT_CFG: &str = "udrome.toml";
impl Config {
    pub fn new(path: Option<String>) -> Result<Self, Error> {
        let path = path.unwrap_or_else(|| {
            info!("no config file path provided, using default ({DEFAULT_CFG})");
            DEFAULT_CFG.to_string()
        });

        let mut fh = File::open(path)?;
        let mut data = String::new();
        fh.read_to_string(&mut data)?;

        Ok(toml::from_str(&data)?)
    }
}

// /// Simple program to greet a person
// #[derive(Parser, Debug)]
// #[command(version, about, long_about = None)]
// pub struct Args {
//     /// Path to your music collection
//     #[arg(short, long)]
//     pub media_path: PathBuf,

//     /// Path that will contain `udrome.sqlite`
//     #[arg(short, long)]
//     pub db_path: PathBuf,

//     /// Shall the indexer skip files with metadata?
//     #[arg(short, long, default_value_t = false)]
//     pub skip_tagged: bool,

//     /// Address to listen on
//     #[arg(short, long, default_value_t = String::from("localhost:3000"))]
//     pub address: String,
// }
