use std::time::Duration;
use std::collections::HashSet;
#[allow(unused_imports)]
use log::{info, warn, error, debug};
use tokio::fs::create_dir_all;

use crate::models::{Image, ImageJob};
use crate::util::{get_image_folder};
use crate::archiver::Archiver;

impl Archiver {
    pub async fn get_boards_with_full_images(&self) -> Result<HashSet<String>, ()> {
        let boards = self.db_client.get_all_boards_async().await
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
        let full_image_boards = self.get_boards_with_full_images().await?;

        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        let mut handles = Vec::new();
        let mut jobs_dispatched = 0;
        loop {
            let mut image_jobs = self.db_client.get_image_jobs_async().await
                .map_err(|e|{error!("Failed to get new image jobs from database: {}", e);})?;
            
            if image_jobs.len() == 0 {break}; // No more jobs available
            
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
        }
        return Ok(())
    }
    pub fn dispatch_archive_image(&self, tx: tokio::sync::mpsc::Sender<bool>, need_full_image: bool, job: ImageJob) -> tokio::task::JoinHandle<()> {
        let c = self.clone();
        tokio::task::spawn(
            async move {
                c.archive_image(&job.clone(), need_full_image).await.ok();
                tx.send(true).await.ok();
            }
        )
    }
    pub async fn archive_image(&self, job: &ImageJob, need_full_image: bool) -> Result<(),()> {
        let thumbnail_folder = get_image_folder(&job.md5, true);
        let full_folder = get_image_folder(&job.md5, false);
        create_dir_all(&thumbnail_folder).await.ok();
        create_dir_all(&full_folder).await.ok();

        let (thumb_exists, full_exists) = self.db_client.image_exists_full_async(&job.md5).await
        .map_err(|e| {error!("Failed to get image status from database: {}", e); e} )
        .unwrap_or((false, false));

        let mut image = Image{
            md5: job.md5.clone(), 
            thumbnail: thumb_exists,
            full_image: full_exists,
            md5_base32: job.md5_base32.clone()
        };
        
        let thumb_success = match !thumb_exists {
            true => self.http_client.download_file(&job.thumbnail_url, 
                &thumbnail_folder.join(&job.thumbnail_filename)).await,
            false => thumb_exists
        };

        image.thumbnail = thumb_success;

        self.db_client.insert_image_async(&image).await
        .map_err(|e| {error!("Failed to insert image {} into database: {}", job.md5, e);})?;

        info!("Processed thumbnail {} ({})", job.md5, job.thumbnail_filename);

        let full_success = match need_full_image && !full_exists {
            true => self.http_client.download_file(&job.url, 
                &full_folder.join(&job.filename)).await,
            false => full_exists
        };

        image.full_image = full_success;

        self.db_client.insert_image_async(&image).await
        .map_err(|e| {error!("Failed to insert image {} into database: {}", job.md5, e);})?;
        self.db_client.delete_image_job_async(&job.board, &job.md5).await
        .map_err(|e| {error!("Failed to delete image {} from backlog: {}", job.md5, e);})?;

        info!("Processed image {} ({}) successfully", job.md5, job.filename);
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