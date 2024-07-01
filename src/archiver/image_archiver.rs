use std::time::{Duration, Instant};
use std::collections::HashSet;
use std::collections::HashMap;
use futures::future::FutureExt;
use std::panic::AssertUnwindSafe;

#[allow(unused_imports)]
use log::{info, warn, error, debug};
#[allow(unused_imports)]
use metrics::{gauge, increment_gauge, decrement_gauge, counter, histogram};

use crate::models::ImageJob;
use crate::archiver::Archiver;

impl Archiver {
    pub async fn get_boards_with_full_images(&self) -> Result<HashSet<String>, ()> {
        let boards = self.db_client.get_all_boards().await
        .map_err(|e| {error!("Failed to get boards from database: {}", e);})?;

        let mut full_images = HashSet::new();
        for board in boards {
            if board.full_images{
                full_images.insert(board.name);
            }
        }
        Ok(full_images)
    }
    pub async fn image_cycle(&self) -> Result<(),()> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        let mut running_jobs = HashMap::new();
        loop {
            let mut jobs = self.db_client.get_image_jobs(250).await
                .map_err(|e|{error!("Failed to get new image jobs from database: {}", e);})?;
            
            if jobs.len() == 0 { // No more jobs available
                tokio::time::sleep(Duration::from_secs(10)).await;
                continue;
            };            
            while let Some(job) = jobs.pop() {
                if running_jobs.contains_key(&job.id) { // Don't schedule the same job twice
                    continue;
                }
                running_jobs.insert(job.id, self.dispatch_archive_image(tx.clone(), job));
                if running_jobs.len() < 20 {continue}; // Dispatch up to 20 jobs at once

                if let Some(job_id) = rx.recv().await { // wait for a job to complete before dispatching the next
                    running_jobs.remove(&job_id);
                } else { // All jobs are terminated or dead
                    running_jobs.clear();
                }
                debug!("One image job has completed")
            }
        }
    }
    pub fn dispatch_archive_image(&self, tx: tokio::sync::mpsc::Sender<i64>, job: ImageJob) -> tokio::task::JoinHandle<()> {
        let c = self.clone();
        tokio::task::spawn(
            async move {
                increment_gauge!("file_jobs_running", 1.0);
                let s = Instant::now();
                AssertUnwindSafe(c.archive_image(&job.clone())).catch_unwind().await.ok();
                histogram!("file_job_duration", s.elapsed().as_millis() as f64);
                decrement_gauge!("file_jobs_running", 1.0);
                tx.send(job.id).await.ok();
            }
        )
    }
    pub async fn archive_image(&self, job: &ImageJob) -> Result<(),()> {
        let mut thumbnail_sha256 = job.thumbnail_sha256.clone();
        let mut file_sha256 = job.file_sha256.clone();

        if thumbnail_sha256.is_empty() {
            thumbnail_sha256 = self.http_client.download_file_checksum(&job.thumbnail_url, &".jpg".to_string(), true).await?;
            counter!("thumbnails_fetched", 1);
            info!("Processed thumbnail for /{}/{}", job.board, job.no);
            self.db_client.set_post_files(&job.board, job.no, &file_sha256, &thumbnail_sha256).await
            .map_err(|e| {error!("Failed to update file for post: /{}/{}: {}", job.board, job.no, e);})?;
        }

        if file_sha256.is_empty() {
            if let Some(board) = self.db_client.get_board(&job.board).await
            .map_err(|e| {error!("Failed to get board info for file job: /{}/{}: {}", job.board, job.no, e);})?
            {
                // If full_images is enabled for the board, download the full image
                if board.full_images {
                    file_sha256 = self.http_client.download_file_checksum(&job.url, &job.ext, false).await?;
                    counter!("files_fetched", 1);
                    info!("Processed full image for /{}/{}", job.board, job.no);
                    self.db_client.set_post_files(&job.board, job.no, &file_sha256, &thumbnail_sha256).await
                    .map_err(|e| {error!("Failed to update file for post: /{}/{}: {}", job.board, job.no, e);})?;
                }
            }
        }
        
        self.db_client.delete_image_job(job.id).await
        .map_err(|e| {error!("Failed to delete file job {} from backlog: {}", job.id, e);})?;
        Ok(())
    }
    pub fn run_image_cycle(&self) -> tokio::task::JoinHandle<()> {
        let c = self.clone();
        tokio::task::spawn(async move {
            loop {
                AssertUnwindSafe(c.image_cycle()).catch_unwind().await.ok();
            }
        })
    }
}