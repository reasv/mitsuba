use std::time::{Duration, Instant};
use futures::future::FutureExt;
use std::panic::AssertUnwindSafe;
#[allow(unused_imports)]
use log::{info, warn, error, debug};
#[allow(unused_imports)]
use metrics::{gauge, counter, histogram};

use crate::models::{ThreadsPage, ThreadInfo};
use crate::util::{get_board_page_api_url, get_board_archive_api_url};
use crate::archiver::Archiver;

impl Archiver {
    pub async fn get_board_pages(&self, board: &String) -> Result<Vec<ThreadsPage>, bool> {
        self.http_client.fetch_json::<Vec<ThreadsPage>>(&get_board_page_api_url(board)).await
    }
    pub async fn push_new_threads(&self, board: &String) -> anyhow::Result<u64, bool> {
        let mut pages = self.get_board_pages(board).await?;
        let mut added_jobs: u64 = 0;
        while let Some(mut page) = pages.pop() { // pop lets us iterate in reverse, we want threads about to die to get fetched first
            // eventually we should implement a formal priority queue system especially for images
            while let Some(mut thread_info) = page.threads.pop() {
                thread_info.board = board.clone();
                thread_info.page = page.page as i32;
                let job_opt = self.db_client.insert_thread_job(&thread_info).await
                .map_err(|e| {error!("Error inserting thread job into database: {}", e); false})?;
                if job_opt.is_some() {
                    added_jobs +=1;
                }
            }
        }
        Ok(added_jobs)
    }
    pub async fn get_board_archive(&self, board: &String) -> Result<Vec<i64>, bool> {
        self.http_client.fetch_json::<Vec<i64>>(&get_board_archive_api_url(board)).await
    }
    pub async fn push_archived_threads(&self, board: &String) -> anyhow::Result<(), bool> {
        let tids = self.get_board_archive(board).await?;
        for tid in tids {
            let mut last_modified = 0;
            let mut replies = 0;
            if let Some(op_post) = self.db_client.get_post(board, tid).await // We have this thread somewhere
            .map_err(|e| {error!("Error getting post from database: {}", e); false})? {
                if op_post.archived == 1 { // We already have the archive version of this, skip
                    continue;
                }
                last_modified = op_post.last_modified;
                replies = op_post.replies;
            }
            info!("Scheduling archived thread /{}/{}", board, tid);
            let thread_info = ThreadInfo {
                board: board.clone(),
                no: tid,
                last_modified,
                replies,
                page: 0,
            };
            self.db_client.insert_thread_job(&thread_info).await
            .map_err(|e| {error!("Error inserting thread job into database: {}", e); false})?;
            counter!("thread_archived_jobs_scheduled", 1);
        }
        Ok(())
    }
    pub async fn board_cycle(&self) -> anyhow::Result<u64, bool> {

        let boards = self.db_client.get_all_boards().await
        .map_err(|e| {error!("Error getting board settings from database: {}", e); false})?;
        let mut added_jobs: u64 = 0;
        for board in boards {
            if !board.archive {
                continue;
            }
            added_jobs += self.push_new_threads(&board.name).await?;
            self.push_archived_threads(&board.name).await?;

        }
        Ok(added_jobs)
    }
    pub fn run_board_cycle(&self) -> tokio::task::JoinHandle<()> {
        let c = self.clone();
        tokio::task::spawn(async move {
            loop {
                let s = Instant::now();
                let add_opt = AssertUnwindSafe(c.board_cycle())
                .catch_unwind().await
                .ok().and_then(|res| res.ok());

                histogram!("boards_scan_duration", s.elapsed().as_millis() as f64);
                if let Some(added_jobs) = add_opt {
                    if added_jobs == 0 { // No new changes
                        tokio::time::sleep(Duration::from_secs(10)).await;
                    } else {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        })
    }
}