use std::time::{Duration, Instant};
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
        if let Some(board) = self.db_client.get_board(&job.board).await
            .map_err(|e| {error!("Failed to get board info for file job: /{}/{}: {}", job.board, job.no, e);})?
        {
            if job.thumbnail_sha256.is_none() && board.archive {
                self.process_single_image(&job.board, job.no, &job.thumbnail_url, &".jpg".to_string(), true).await?;
            }
    
            // If full_images is enabled for the board (and the board is still enabled), download the full image
            if job.file_sha256.is_none() && board.full_images && board.archive {
                self.process_single_image(&job.board, job.no, &job.url, &job.ext, false).await?;
            }
        }
        self.db_client.delete_image_job(job.id).await
        .map_err(|e| {error!("Failed to delete file job {} from backlog: {}", job.id, e);})?;
        Ok(())
    }

    async fn process_single_image(
        &self,
        board: &String,
        no: i64,
        url: &String,
        ext: &String,
        is_thumb: bool
    )
    -> Result<(),()> {
        let sha256 = self.http_client
            .download_file_checksum(
                url,
                ext,
                is_thumb
        ).await?;
        if is_thumb {
            counter!("thumbnails_fetched", 1);
        } else {
            counter!("files_fetched", 1);
        }
        info!("Processed file (thumb: {}) for /{}/{}", is_thumb, board, no);
        self.db_client
            .add_post_file(
                board,
                no,
                0,
                &sha256,
                ext,
                true
            ).await
            .map_err(|e| {error!("Failed to update file for post: /{}/{}: {}", board, no, e);})?;
        self.handle_blacklist(board, no, &sha256, ext, is_thumb)
            .await.map_err(|e| {error!("Failed to check file blacklist for post: /{}/{}: {}", board, no, e);})?;
        Ok(())
    }

    async fn handle_blacklist(&self, board_name: &String, no: i64, sha256: &String, ext: &String, is_thumb: bool) -> anyhow::Result<()> {
        if !self.db_client.is_file_blacklisted(sha256).await? {
            return Ok(());
        }
        // File is blacklisted, hide the image and delete it
        warn!("Blacklisted file on /{}/{} ({}) detected, hiding and deleting", board_name, no, sha256);
        self.db_client.set_post_hidden_status(board_name, no, false, false, true).await?;
        self.http_client.delete_downloaded_file(sha256, ext, is_thumb).await?;
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