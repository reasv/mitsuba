
use std::time::Duration;
use std::sync::Arc;

use crate::http::HttpClient;
use crate::models::{Thread, ThreadInfo, ThreadsPage, ImageInfo};

#[allow(unused_imports)]
use log::{info, warn, error, debug};

use crate::models::Post;
use crate::db;

pub fn get_board_page_api_url(board: &String) -> String {
    format!("https://a.4cdn.org/{}/threads.json", board)
}
pub fn get_thread_api_url(board: &String, tid: &String) -> String {
    format!("https://a.4cdn.org/{}/thread/{}.json", board, tid)
}

#[derive(Clone)]
pub struct Archiver {
    pub http_self: Arc<HttpClient>
}

impl Default for Archiver {
    fn default() -> Self { 
        Self::new(HttpClient::default())
    }
}

impl Archiver {
    pub fn new(client: HttpClient) -> Archiver {
        Archiver {
            http_self: Arc::new(client)
        }
    }
    pub async fn get_board_pages(&self, board: &String) -> anyhow::Result<Vec<ThreadsPage>> {
        self.http_self.fetch_json::<Vec<ThreadsPage>>(&get_board_page_api_url(board)).await
    }
    pub fn get_post_image_info(&self, board: &String, post: &Post) -> Option<ImageInfo> {
        if post.tim == 0 {
            return None // no image
        }
        let url = format!("https://i.4cdn.org/{}/{}{}", board, post.tim, post.ext);
        let filename = format!("{}{}", post.tim, post.ext);
        Some(ImageInfo{url, filename, md5: post.md5.clone()})
    }

    pub async fn get_thread(&self, board: &String, tid: &String) -> anyhow::Result<Thread> {
        match self.http_self.fetch_json::<Thread>(&get_thread_api_url(board, tid)).await {
            Ok(t) => Ok(t),
            Err(msg) => {
                error!("Could not get thread /{}/{} (Error: {:?})", board, tid, msg);
                return Err(msg);
            }
        }
    }

    /// Get all ThreadInfo for threads on this board modified since `last_modified_since`
    pub async fn get_all_thread_info_since(&self, board: &String, last_modified_since: i64) -> anyhow::Result<(Vec<ThreadInfo>, i64)> {
        let pages = self.get_board_pages(board).await?;
        let mut infos = Vec::new();
        for page in pages {
            for thread_info in page.threads {
                if thread_info.last_modified > last_modified_since {
                    infos.push(thread_info);
                }
            }
        }
        // Timestamp for last modified threadinfo
        let last_modified = infos.iter().map(|i| i.last_modified).max().unwrap_or(last_modified_since);
        Ok((infos, last_modified))
    }
    pub async fn archive_cycle(self: Archiver, board: String, last_change: i64) -> i64 {
        info!("Fetching new thread changes for board /{}/", board);
        let (thread_infos, new_last_change) = match self.get_all_thread_info_since(&board, last_change).await {
            Ok(t) => t,
            Err(e) => {
                error!("Failed to fetch thread changes for /{}/: {}", board, e);
                return last_change;
            }
        };
        
        info!("Thread info fetched for /{}/. {} threads have had changes or were created since {}", board, thread_infos.len(), last_change);
        let mut handles = Vec::new();
        for info in thread_infos {
            let c = self.clone();
            let b = board.clone();
            let i = info.clone();
            handles.push(
                tokio::task::spawn(async move {
                    let thread = match c.get_thread(&b, &i.no.to_string()).await {
                        Ok(t) => t,
                        Err(e) => {
                            error!("Failed to fetch thread /{}/{}: {}", b, i.no, e);
                            return (i.last_modified, None)
                        }
                    };
                    match tokio::task::block_in_place(|| db::insert_posts(thread.posts.clone())) {
                        Ok(_) => (),
                        Err(e) => {
                            error!("Failed to insert thread /{}/{} into database: {}", b, i.no, e);
                        }
                    };
                    (i.last_modified, Some(thread))
                })
            );
        }
        let mut current_last_change = new_last_change;
        let mut images = Vec::new();
        for handle in handles {
            // The task returns the last modified of thread, plus thread if it succeeded, None if failed
            let (last_modified, thread) = handle.await.unwrap();
            
            match thread {
                Some(t) => { // Thread fetched successfully
                    images.append(&mut t.posts.iter().filter_map(|p| self.get_post_image_info(&board, p)).collect::<Vec<ImageInfo>>());
                },
                None => { // Thread not fetched successfully
                    if current_last_change > last_modified {
                        // next time resume archiving from before earliest failed thread
                        current_last_change = last_modified - 1;
                    }
                }
            };
        }
        self.archive_images(images).await;

        current_last_change
    }
    pub async fn archive_images(&self, images: Vec<ImageInfo>) {
        // Todo: fetch images
    }
    pub async fn run_archive_cycle(&self, board: String, wait_time: Duration) -> tokio::task::JoinHandle<()>{
        let c = self.clone();
        let mut last_change = 0;
        tokio::task::spawn(async move {
            loop {
                last_change = c.clone().archive_cycle(board.clone(), last_change).await;
                tokio::time::sleep(wait_time).await;
            }
        })
    }    
}