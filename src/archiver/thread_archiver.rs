use futures::future::FutureExt;
use std::collections::HashMap;
use std::panic::AssertUnwindSafe;
use std::time::{Duration, Instant, SystemTime};

#[allow(unused_imports)]
use log::{debug, error, info, warn};
#[allow(unused_imports)]
use metrics::{counter, decrement_gauge, gauge, histogram, increment_gauge};

use crate::archiver::Archiver;
use crate::models::{ImageInfo, Post, Thread, ThreadJob};
use crate::util::get_thread_api_url;

impl Archiver {
    pub async fn get_thread(&self, board: &String, tid: &String) -> Result<Option<Thread>, bool> {
        match self
            .http_client
            .fetch_json::<Thread>(&get_thread_api_url(board, tid))
            .await
        {
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
            return None; // no image
        }
        let url = format!("https://i.4cdn.org/{}/{}{}", board, post.tim, post.ext);
        let thumbnail_url = format!("https://i.4cdn.org/{}/{}s.jpg", board, post.tim);
        Some(ImageInfo {
            url,
            thumbnail_url,
            ext: post.ext.clone(),
            file_sha256: post.file_sha256.clone(),
            thumbnail_sha256: post.thumbnail_sha256.clone(),
            page,
            no: post.no,
            board: board.clone(),
        })
    }
    pub async fn thread_cycle(&self) -> anyhow::Result<()> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        let mut running_jobs = HashMap::new();
        loop {
            let mut jobs = self.db_client.get_thread_jobs(250).await.map_err(|e| {
                error!("Failed to get new thread jobs from database: {}", e);
                e
            })?;

            if jobs.len() == 0 {
                // No more jobs available
                tokio::time::sleep(Duration::from_secs(10)).await;
                continue;
            };

            while let Some(job) = jobs.pop() {
                if running_jobs.contains_key(&job.id) {
                    // Don't schedule the same job twice
                    continue;
                }
                running_jobs.insert(job.id, self.dispatch_archive_thread(tx.clone(), job));
                if running_jobs.len() < 20 {
                    continue;
                }; // Dispatch up to 20 jobs at once

                if let Some(job_id) = rx.recv().await {
                    // wait for a job to complete before dispatching the next
                    running_jobs.remove(&job_id);
                } else {
                    // All jobs are terminated or panicked
                    running_jobs.clear();
                }
                debug!("One thread job has completed")
            }
        }
    }
    pub fn dispatch_archive_thread(
        &self,
        tx: tokio::sync::mpsc::Sender<i64>,
        job: ThreadJob,
    ) -> tokio::task::JoinHandle<()> {
        let c = self.clone();
        tokio::task::spawn(async move {
            increment_gauge!("thread_jobs_running", 1.0);
            let s = Instant::now();
            let job_id = job.id.clone();
            AssertUnwindSafe(c.archive_thread(job))
                .catch_unwind()
                .await
                .ok();
            histogram!("thread_job_duration", s.elapsed().as_millis() as f64);
            decrement_gauge!("thread_jobs_running", 1.0);
            tx.send(job_id).await.ok();
        })
    }
    pub async fn archive_thread(&self, job: ThreadJob) -> Result<(), ()> {
        let timestamp: i64 = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|_| {
                error!("SystemTime before UNIX EPOCH!");
            })?
            .as_secs() as i64;

        let thread_opt = self
            .get_thread(&job.board, &job.no.to_string())
            .await
            .map_err(|_| {
                error!("Failed to fetch thread /{}/{}", job.board, job.no);
            })?;
        counter!("threads_fetched", 1);

        if thread_opt.is_none() {
            // Thread was 404
            warn!(
                "Thread /{}/{} [{}] 404, deleting from backlog ({}).",
                job.board, job.no, job.last_modified, job.id
            );
            self.db_client
                .set_post_deleted(&job.board, job.no, timestamp)
                .await
                .map_err(|e| {
                    error!(
                        "Failed to set thread /{}/{} as deleted: {}",
                        job.board, job.no, e
                    );
                })?;

            self.db_client
                .delete_thread_job(job.id)
                .await
                .map_err(|e| {
                    error!(
                        "Failed to delete thread /{}/{} from backlog: {}",
                        job.board, job.no, e
                    );
                })?;
            counter!("thread_404", 1);
            return Ok(());
        }
        let thread = thread_opt.unwrap_or_default();

        let posts: Vec<Post> = thread
            .posts
            .clone()
            .into_iter()
            .map(|mut post| {
                post.board = job.board.clone();
                post.last_modified = job.last_modified;
                post
            })
            .collect();

        // Handle detecting posts that have been deleted
        let post_ids: Vec<i64> = thread.posts.iter().map(|p| p.no).collect();
        let deleted_posts = self
            .db_client
            .set_missing_posts_deleted(&job.board, job.no, post_ids, timestamp)
            .await
            .map_err(|e| {
                error!(
                    "Failed to set deleted posts for /{}/{} in database: {}",
                    job.board, job.no, e
                );
            })?;
        counter!("post_deleted", deleted_posts.len() as u64);

        let inserted_posts = self.db_client.insert_posts(&posts).await.map_err(|e| {
            error!(
                "Failed to insert thread /{}/{} into database: {}",
                job.board, job.no, e
            );
        })?;

        for post in inserted_posts {
            if let Some(image_info) = self.get_post_image_info(&job.board, job.page, &post) {
                self.db_client
                    .insert_image_job(&image_info)
                    .await
                    .map_err(|e| {
                        error!(
                            "Failed to insert image job /{}/{} into database: {}",
                            job.board, image_info.no, e
                        );
                    })?;
            }
        }
        self.db_client
            .delete_thread_job(job.id)
            .await
            .map_err(|e| {
                error!(
                    "Failed to delete thread /{}/{} from backlog: {}",
                    job.board, job.no, e
                );
            })?;
        Ok(())
    }
    pub fn run_thread_cycle(&self) -> tokio::task::JoinHandle<()> {
        let c = self.clone();
        tokio::task::spawn(async move {
            loop {
                AssertUnwindSafe(c.thread_cycle()).catch_unwind().await.ok();
            }
        })
    }
}
