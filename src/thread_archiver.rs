use std::time::Duration;
#[allow(unused_imports)]
use log::{info, warn, error, debug};

use crate::models::{ThreadJob, ImageInfo, Post, Thread};
use crate::util::get_thread_api_url;
use crate::archiver::Archiver;

impl Archiver {
    pub async fn get_thread(&self, board: &String, tid: &String) -> Result<Option<Thread>, bool> {
        match self.http_client.fetch_json::<Thread>(&get_thread_api_url(board, tid)).await {
            Ok(t) => Ok(Some(t)),
            Err(is_404) => {
                if is_404 {
                    Ok(None)
                } else {
                    Err(is_404)
                }
            }
        }
    }
    pub fn get_post_image_info(&self, board: &String, page: i32, post: &Post) -> Option<ImageInfo> {
        if post.tim == 0 || post.filedeleted == 1 {
            return None // no image
        }
        let url = format!("https://i.4cdn.org/{}/{}{}", board, post.tim, post.ext);
        let thumbnail_url = format!("https://i.4cdn.org/{}/{}s.jpg", board, post.tim);
        Some(ImageInfo{url, thumbnail_url, ext: post.ext.clone(), file_sha256: post.file_sha256.clone(), thumbnail_sha256: post.thumbnail_sha256.clone(), page, no: post.no, board: board.clone()})
    }
    pub async fn thread_cycle(&self) -> anyhow::Result<()> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        let mut handles = Vec::new();
        let mut jobs_dispatched = 0u64;
        loop {
            let mut jobs = self.db_client.get_thread_jobs(250).await
                .map_err(|e|{error!("Failed to get new thread jobs from database: {}", e); e})?;
            
            if jobs.len() == 0 {break}; // No more jobs available
            
            while let Some(job) = jobs.pop() {
                handles.push(self.dispatch_archive_thread(tx.clone(), job));
                jobs_dispatched+=1;
                if jobs_dispatched < 20 {continue}; // dispatch up to 20 first images in the log first

                if let Some(res) = rx.recv().await { // Wait for one job to complete before continuing
                    if let Err(info) = res {
                        jobs.push(info); // re-add thread that failed for non-404 reasons for immediate re-trying
                    }
                }
                debug!("One thread job has completed")
            }
            while let Some(handle) = handles.pop() {
                handle.await.ok();
            }
        }
        Ok(())
    }
    pub fn dispatch_archive_thread(&self, tx: tokio::sync::mpsc::Sender<Result<(), ThreadJob>>, job: ThreadJob) -> tokio::task::JoinHandle<Result<(), ThreadJob>>{
        let c = self.clone();
        tokio::task::spawn(async move {
            let res = c.archive_thread(job).await;
            tx.send(res.clone()).await.ok();
            res
        })
    }
    pub async fn archive_thread(&self, job: ThreadJob) -> Result<(), ThreadJob> {
        let thread_opt = self.get_thread(&job.board, &job.no.to_string()).await
        .map_err(|_| {error!("Failed to fetch thread /{}/{}", job.board, job.no); job.clone()})?;

        if thread_opt.is_none() { // Thread was 404
            self.db_client.delete_thread_job(job.id).await
            .map_err(|e| {error!("Failed to delete thread /{}/{} from backlog: {}", job.board, job.no, e); job})?;
            return Ok(())
        }
        let thread = thread_opt.unwrap_or_default();

        let posts: Vec<Post> = thread.posts.clone().into_iter().map(|mut post|{post.board = job.board.clone(); post.last_modified = job.last_modified; post}).collect();
        
        let inserted_posts = self.db_client.insert_posts(&posts).await
        .map_err(|e| {error!("Failed to insert thread /{}/{} into database: {}", job.board, job.no, e); job.clone()})?;

        let image_jobs = inserted_posts.iter().filter_map(|post| self.get_post_image_info(&job.board, job.page, post))
        .collect::<Vec<ImageInfo>>();

        for image_info in image_jobs {
            self.db_client.insert_image_job(&image_info).await
            .map_err(|e| {error!("Failed to insert image job /{}/{} into database: {}", job.board, image_info.no, e); job.clone()})?;
        }
        self.db_client.delete_thread_job(job.id).await
        .map_err(|e| {error!("Failed to delete thread /{}/{} from backlog: {}", job.board, job.no, e); job})?;
        Ok(())
    }
    pub fn run_thread_cycle(&self) -> tokio::task::JoinHandle<()> {
        let c = self.clone();
        tokio::task::spawn(async move {
            loop {
                c.thread_cycle().await.ok();
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        })
    }
}
