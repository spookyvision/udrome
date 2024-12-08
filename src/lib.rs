use std::sync::atomic::{AtomicU32, Ordering};

use camino::{Utf8Path, Utf8PathBuf};
use thiserror::Error;
use tracing::{debug, error, info, warn};
use walkdir::WalkDir;

// goal: build as much as possible so it can be reused by Fileperson
pub mod indexer;

pub mod query;

pub mod api;

pub mod entity;

pub mod config;

pub(crate) mod util;
pub trait FileVisitor: Clone {
    fn visit(
        &mut self,
        entry: impl AsRef<Utf8Path>,
    ) -> impl std::future::Future<Output = ()> + Send;
}

// TODO not parallel enough!!!
async fn load(root: impl AsRef<Utf8Path>, mut action: impl FileVisitor, count: &AtomicU32) {
    for entry in WalkDir::new(root.as_ref()) {
        let Ok(entry) = entry else {
            error!("?? {entry:?}");
            return;
        };

        let val: u32 = count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        // TODO progress report
        if val % 100 == 0 {
            // info!("(load) {val}");
        }

        let ep = entry.into_path();
        let Some(path) = Utf8Path::from_path(&ep) else {
            error!("?? {ep:?}");
            return;
        };

        // TODO symlinks yes no maybe
        if path.is_file()
            && path.extension().map(|ext| ext.to_lowercase()) == Some("mp3".to_string())
        {
            count.fetch_add(1, Ordering::Relaxed);
            action.visit(path).await;
        }
    }
}
