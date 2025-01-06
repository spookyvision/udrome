use std::{fs::File, io::Read};

use serde::Deserialize;
use thiserror::Error;
use tracing::info;

#[derive(Deserialize)]
pub struct Config {
    pub system: System,
    pub media: Media,
    pub indexer: Indexer,
}

#[derive(Deserialize)]
pub struct System {
    pub data_path: String,
    pub cache_mb: u16,
    pub bind_addr: String,
    pub base_url: Option<String>,
    pub dev: bool,
}

#[derive(Deserialize)]
pub struct Media {
    pub paths: Vec<camino::Utf8PathBuf>,
}

#[derive(Deserialize, Clone)]
pub struct Indexer {
    pub enable: bool,
    pub exclude: Exclude,
}

#[derive(Deserialize, Clone)]
pub struct Exclude {
    pub files: Vec<String>,
    pub dirs: Vec<String>,
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
