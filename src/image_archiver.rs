use std::time::{Duration, Instant};
use std::collections::HashSet;

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
        let mut handles = Vec::new();
        let mut jobs_dispatched = 0;
        loop {
            let s = Instant::now();
            let mut image_jobs = self.db_client.get_image_jobs(250).await
                .map_err(|e|{error!("Failed to get new image jobs from database: {}", e);})?;
            
            if image_jobs.len() == 0 {break}; // No more jobs available
            let full_image_boards = self.get_boards_with_full_images().await?;
            
            while let Some(job) = image_jobs.pop() {
                handles.push(self.dispatch_archive_image(tx.clone(),  full_image_boards.contains(&job.board), job));
                jobs_dispatched+=1;
                if jobs_dispatched < 20 {continue}; // dispatch up to 20 first images in the log first

                rx.recv().await; // wait for a job to complete before dispatching the next
                debug!("One image job has completed")
            }
            while let Some(handle) = handles.pop() {
                handle.await.ok();
            }
            histogram!("file_batch_duration", s.elapsed().as_millis() as f64);
        }
        return Ok(())
    }
    pub fn dispatch_archive_image(&self, tx: tokio::sync::mpsc::Sender<bool>, need_full_image: bool, job: ImageJob) -> tokio::task::JoinHandle<()> {
        let c = self.clone();
        tokio::task::spawn(
            async move {
                increment_gauge!("file_jobs_running", 1.0);
                let s = Instant::now();
                c.archive_image(&job.clone(), need_full_image).await.ok();
                histogram!("file_job_duration", s.elapsed().as_millis() as f64);
                decrement_gauge!("file_jobs_running", 1.0);
                counter!("file_jobs_completed", 1);
                tx.send(true).await.ok();
            }
        )
    }
    pub async fn archive_image(&self, job: &ImageJob, need_full_image: bool) -> Result<(),()> {
        let mut thumbnail_sha256 = job.thumbnail_sha256.clone();
        let mut file_sha256 = job.file_sha256.clone();

        if thumbnail_sha256.is_empty() {
            thumbnail_sha256 = self.http_client.download_file_checksum(&job.thumbnail_url, &".jpg".to_string(), true).await
            .unwrap_or(thumbnail_sha256);
            counter!("thumbnails_fetched", 1);
            info!("Processed thumbnail for /{}/{}", job.board, job.no);
            self.db_client.set_post_files(&job.board, job.no, &file_sha256, &thumbnail_sha256).await
            .map_err(|e| {error!("Failed to update file for post: /{}/{}: {}", job.board, job.no, e);})?;
        }

        if file_sha256.is_empty() && need_full_image {
            file_sha256 = self.http_client.download_file_checksum(&job.url, &job.ext, false).await
            .unwrap_or(file_sha256);
            counter!("files_fetched", 1);
            info!("Processed full image for /{}/{}", job.board, job.no);
            self.db_client.set_post_files(&job.board, job.no, &file_sha256, &thumbnail_sha256).await
            .map_err(|e| {error!("Failed to update file for post: /{}/{}: {}", job.board, job.no, e);})?;
        }
        self.db_client.delete_image_job(job.id).await
        .map_err(|e| {error!("Failed to delete file job {} from backlog: {}", job.id, e);})?;
        Ok(())
    }
    pub fn run_image_cycle(&self) -> tokio::task::JoinHandle<()> {
        let c = self.clone();
        tokio::task::spawn(async move {
            loop {
                c.image_cycle().await.ok();
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        })
    }
}