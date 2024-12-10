use std::{process, str::from_utf8};

use camino::Utf8Path;
use serde::{
    de::{self, DeserializeOwned},
    Deserialize, Deserializer,
};
use serde_json::{Map, Value};
use thiserror::Error;
use tracing::warn;

#[derive(Debug, Deserialize)]
pub(crate) struct Metadata {
    format: Tags,
}

impl Metadata {
    pub(crate) fn into_tag(self) -> Tag {
        self.format.tags
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct Tags {
    #[serde(deserialize_with = "case_insensitive")]
    tags: Tag,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Tag {
    pub(crate) title: String,
    pub(crate) artist: Option<String>,
    pub(crate) album: Option<String>,
    pub(crate) genre: Option<String>,
    // hurray it can be "01" or "2/14", so no uint here 🙄
    pub(crate) track: Option<String>,
}

fn case_insensitive<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: DeserializeOwned,
    D: Deserializer<'de>,
{
    let map = Map::<String, Value>::deserialize(deserializer)?;
    let lower = map
        .into_iter()
        .map(|(k, v)| (k.to_lowercase(), v))
        .collect();
    T::deserialize(Value::Object(lower)).map_err(de::Error::custom)
}

#[derive(Error, Debug)]
pub(crate) enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Deser(#[from] serde_json::Error),
}
pub(crate) fn metadata(f: impl AsRef<Utf8Path>) -> Result<Metadata, Error> {
    // TODO either preprocess all metadata to UTF8 or use json=sv=ignore + chardetng + recode
    let md = process::Command::new("ffprobe")
        .args([
            "-loglevel",
            "error",
            "-show_entries",
            "stream_tags:format_tags",
            "-of",
            "json",
            f.as_ref().as_str(),
        ])
        .output()?
        .stdout;

    if let Err(_e) = from_utf8(&md) {
        warn!("TODO handle/convert non-utf8");
    }

    let md: Metadata = serde_json::from_slice(&md)?;
    Ok(md)
}
