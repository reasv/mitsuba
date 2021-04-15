use futures::future::FutureExt;
use std::panic::AssertUnwindSafe;
use std::time::{Instant, Duration};

#[allow(unused_imports)]
use log::{info, warn, error, debug};
#[allow(unused_imports)]
use metrics::{gauge, increment_gauge, decrement_gauge, counter, histogram};

use crate::archiver::Archiver;

impl Archiver {
    async fn metrics_cycle(&self) -> anyhow::Result<(),()> {
        let fbacklog = self.db_client.get_image_backlog_size(0).await
        .map_err(|e| {error!("Error getting backlog size: {}", e)})? as f64;
        gauge!("file_backlog_size", fbacklog);
        let fbacklog_live = self.db_client.get_image_backlog_size(1).await
        .map_err(|e| {error!("Error getting backlog size: {}", e)})? as f64;
        gauge!("file_backlog_size_live", fbacklog_live);
        let tbacklog =  self.db_client.get_thread_backlog_size(0).await
        .map_err(|e| {error!("Error getting backlog size: {}", e)})? as f64;
        gauge!("thread_backlog_size", tbacklog);
        let tbacklog_live = self.db_client.get_thread_backlog_size(1).await
        .map_err(|e| {error!("Error getting backlog size: {}", e)})? as f64;
        gauge!("thread_backlog_size_live", tbacklog_live);

        let stored_files = self.db_client.get_stored_files().await
        .map_err(|e| {error!("Error getting backlog size: {}", e)})? as f64;
        gauge!("files_stored", stored_files);
        let stored_thumbnails = self.db_client.get_stored_thumbnails().await
        .map_err(|e| {error!("Error getting backlog size: {}", e)})? as f64;
        gauge!("thumbnails_stored", stored_thumbnails);
        let missing_thumbnails = self.db_client.get_missing_thumbnails().await
        .map_err(|e| {error!("Error getting backlog size: {}", e)})? as f64;
        gauge!("thumbnails_missing", missing_thumbnails);

        Ok(())
    }
    pub fn run_metrics_cycle(&self) -> tokio::task::JoinHandle<()> {
        let c = self.clone();
        tokio::task::spawn(async move {
            loop {
                let s = Instant::now();
                AssertUnwindSafe(c.metrics_cycle()).catch_unwind().await.ok();
                histogram!("metrics_scan_duration", s.elapsed().as_millis() as f64);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        })
    }

}