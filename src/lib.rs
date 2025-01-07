use std::sync::atomic::{AtomicU32, Ordering};

use camino::Utf8Path;
use tracing::{debug, error};
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

        let ep = entry.into_path();
        let Some(path) = Utf8Path::from_path(&ep) else {
            error!("skipping non UTF-8 path: {ep:?}");
            return;
        };

        let val: u32 = count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if val % 100 == 0 {
            debug!("indexer:: {val}");
        }

        // TODO symlinks yes no maybe
        // TODO hardcoded mp3 extension
        if path.is_file()
            && path.extension().map(|ext| ext.to_lowercase()) == Some("mp3".to_string())
        {
            count.fetch_add(1, Ordering::Relaxed);
            action.visit(path).await;
        }
    }
}
