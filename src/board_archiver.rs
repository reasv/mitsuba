use std::time::{Duration, Instant};

#[allow(unused_imports)]
use log::{info, warn, error, debug};
#[allow(unused_imports)]
use metrics::{gauge, counter, histogram};

use crate::models::{ThreadsPage};
use crate::util::{get_board_page_api_url};
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
    pub async fn board_cycle(&self) -> anyhow::Result<u64, bool> {

        let boards = self.db_client.get_all_boards().await
        .map_err(|e| {error!("Error getting board settings from database: {}", e); false})?;
        let mut added_jobs: u64 = 0;
        for board in boards {
            if !board.archive {
                continue;
            }
            added_jobs += self.push_new_threads(&board.name).await?;
        }
        Ok(added_jobs)
    }
    pub fn run_board_cycle(&self) -> tokio::task::JoinHandle<()> {
        let c = self.clone();
        tokio::task::spawn(async move {
            loop {
                let s = Instant::now();
                let add_opt = c.board_cycle().await.ok();
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