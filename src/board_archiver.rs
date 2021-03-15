use std::path::Path;
use std::time::Duration;
use std::sync::Arc;
use std::convert::TryInto;
use std::collections::HashSet;
#[allow(unused_imports)]
use log::{info, warn, error, debug};

use tokio::task::block_in_place;
use tokio::fs::create_dir_all;
use base64::decode;
use base32::{Alphabet, encode};

use crate::http::HttpClient;
use crate::models::{Thread, ThreadInfo, ThreadsPage, ImageInfo, Image, Board, BoardsList, ImageJob};

use crate::models::Post;
use crate::db::DBClient;

pub fn get_board_page_api_url(board: &String) -> String {
    format!("https://a.4cdn.org/{}/threads.json", board)
}
pub fn get_thread_api_url(board: &String, tid: &String) -> String {
    format!("https://a.4cdn.org/{}/thread/{}.json", board, tid)
}
pub fn base64_to_32(b64: String) -> anyhow::Result<String> {
    let binary = decode(b64)?;
    let s = encode(Alphabet::RFC4648{padding: false}, binary.as_slice());
    Ok(s)
}

#[derive(Clone)]
pub struct Archiver {
    pub http_client: Arc<HttpClient>,
    db_client: DBClient
}

impl Default for Archiver {
    fn default() -> Self { 
        Self::new(HttpClient::default())
    }
}

impl Archiver {
    pub fn new(client: HttpClient) -> Archiver {
        Archiver {
            http_client: Arc::new(client),
            db_client: DBClient::new()
        }
    }
    pub async fn get_board_pages(&self, board: &String) -> anyhow::Result<Vec<ThreadsPage>> {
        self.http_client.fetch_json::<Vec<ThreadsPage>>(&get_board_page_api_url(board)).await
    }
    pub fn get_post_image_info(&self, board: &String, post: &Post) -> Option<ImageInfo> {
        if post.tim == 0 {
            return None // no image
        }
        let url = format!("https://i.4cdn.org/{}/{}{}", board, post.tim, post.ext);
        let thumbnail_url = format!("https://i.4cdn.org/{}/{}s.jpg", board, post.tim);
        let md5_b32 = match base64_to_32(post.md5.clone()) {
            Ok(b32) => b32,
            Err(e) => {
                error!("Error converting image to base32: {}", e);
                return None
            }
        };
        let filename = format!("{}{}", md5_b32, post.ext);
        let thumbnail_filename = format!("{}.jpg", md5_b32);
        Some(ImageInfo{url, thumbnail_url, filename, thumbnail_filename, md5: post.md5.clone(), md5_base32: md5_b32, board: board.clone()})
    }

    pub async fn get_thread(&self, board: &String, tid: &String) -> anyhow::Result<Thread> {
        match self.http_client.fetch_json::<Thread>(&get_thread_api_url(board, tid)).await {
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
                    c.archive_thread(b, i).await
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
                    // Collect image data
                    images.append(&mut t.posts.iter().filter_map(|p| self.get_post_image_info(&board, p)).collect::<Vec<ImageInfo>>());
                },
                None => { // Thread not fetched successfully

                    // next time resume archiving from before earliest failed thread
                    if current_last_change > last_modified {
                        current_last_change = last_modified - 1;
                    }
                }
            };
        }

        current_last_change
    }
    pub async fn archive_thread(&self, board: String, info: ThreadInfo) -> (i64, Option<Thread>) {
        let thread = match self.get_thread(&board, &info.no.to_string()).await {
            Ok(t) => t,
            Err(e) => {
                error!("Failed to fetch thread /{}/{}: {}", board, info.no, e);
                return (info.last_modified, None)
            }
        };
        let posts: Vec<Post> = thread.posts.clone().into_iter().map(|mut post|{post.board = board.clone(); post}).collect();
        let image_infos = posts.iter().filter_map(|post| self.get_post_image_info(&board,post)).collect::<Vec<ImageInfo>>();
        match block_in_place(|| self.db_client.insert_posts(posts)) {
            Ok(_) => (),
            Err(e) => {
                error!("Failed to insert thread /{}/{} into database: {}", board, info.no, e);
            }
        };
        for info in image_infos {
            match block_in_place(|| self.db_client.insert_image_job(&info)) {
                Ok(_) => (),
                Err(e) => {
                    error!("Failed to insert image job /{}/{} into database: {}", board, info.md5.clone(), e);
                }
            };
        }
        (info.last_modified, Some(thread))
    }
    pub async fn image_cycle(&self) {
        let image_jobs = match block_in_place(|| self.db_client.get_image_jobs()) {
            Ok(j) => j,
            Err(e) => {
                error!("Failed to get new image jobs from database: {}", e);
                return
            }
        };
        info!("Running image cycle for {} new jobs", image_jobs.len());
        let boards = match block_in_place(|| self.db_client.get_all_boards()){
            Ok(j) => j,
            Err(e) => {
                error!("Failed to get boards from database: {}", e);
                return
            }
        };
        let mut full_images = HashSet::new();
        for board in boards {
            if board.full_images{
                full_images.insert(board.name);
            }
        }
        
        let image_folder = Path::new("data/images");
        create_dir_all(image_folder.join("thumb")).await.unwrap_or_default();
        create_dir_all(image_folder.join("full")).await.unwrap_or_default();
        let mut handles = Vec::new();
        for job in image_jobs {
            let c = self.clone();
            let need_full_image = full_images.contains(&job.board);
            handles.push(tokio::task::spawn(
                async move {
                    c.archive_image(&job.clone(), image_folder.clone(), need_full_image).await
                }
            ))
        }
        for handle in handles {
            handle.await.unwrap_or_default();
        }
    }
    pub async fn archive_image(&self, job: &ImageJob, folder: &Path, need_full_image: bool) {
        let(thumb_exists, full_exists) = match block_in_place(|| self.db_client.image_exists_full(&job.md5)) {
            Ok(j) => j,
            Err(e) => {
                error!("Failed to get image status from database: {}", e);
                (false, false)
            }
        };
        let thumb_success = match !thumb_exists {
            true => self.http_client.download_file(&job.thumbnail_url, 
                &folder.join("thumb").join(&job.thumbnail_filename)).await,
            false => false
        };
        let full_success = match need_full_image && !full_exists { 
            true => self.http_client.download_file(&job.url, 
                &folder.join("full").join(&job.filename)).await,
            false => false
        };
        
        match block_in_place(|| self.db_client.insert_image(&Image{md5: job.md5.clone(), 
            thumbnail: thumb_success, full_image: full_success, md5_base32: job.md5_base32.clone()})) {
            Ok(_) => (),
            Err(e) => {
                error!("Failed to insert image {} into database: {}", job.md5, e);
                return
            }
        };
        match block_in_place(|| self.db_client.delete_image_job(&job.md5)) {
            Ok(_) => (),
            Err(e) => {
                error!("Failed to delete image {} from backlog: {}", job.md5, e);
            }
        };
        info!("Processed image {} successfully", job.md5);
    }
    pub async fn run_image_cycle(&self) -> tokio::task::JoinHandle<()> {
        let c = self.clone();
        tokio::task::spawn(async move { 
            loop {
                c.image_cycle().await;
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        })
    }
    pub async fn run_archive_cycle(&self, board: &Board) -> tokio::task::JoinHandle<()>{
        let c = self.clone();
        let board_name = board.name.clone();
        tokio::task::spawn(async move {
            loop {
                let mut b = match block_in_place(|| c.db_client.get_board(&board_name)){
                    Ok(opt_board) => opt_board.unwrap(),
                    Err(e) => {
                        info!("Error getting board from database: {}", e);
                        continue;
                    }
                };

                let last_modified = c.clone().archive_cycle(b.name.clone(), b.last_modified).await;
                b = match block_in_place(|| c.db_client.get_board(&board_name)){
                    Ok(opt_board) => opt_board.unwrap(),
                    Err(e) => {
                        info!("Error getting board from database: {}", e);
                        continue;
                    }
                };
                b.last_modified = last_modified;
                block_in_place(|| c.db_client.insert_board(&b)).unwrap_or_default();
                if !b.archive {
                    info!("Stopping archiver for {}", b.name);
                    return;
                }
                tokio::time::sleep(Duration::from_secs(b.wait_time.try_into().unwrap())).await;
            }
        })
    }
    pub async fn run_archivers(&self) -> anyhow::Result<()> {
        let boards = block_in_place(|| self.db_client.get_all_boards())?;
        for board in boards {
            if !board.archive {continue};
            self.run_archive_cycle(&board).await;
        }
        self.run_image_cycle().await;
        Ok(())
    }
    #[allow(dead_code)]
    pub async fn set_board(&self, board: Board) -> anyhow::Result<usize> {
        let db_board = block_in_place(|| self.db_client.get_board(&board.name))?;
        let insert_board = match db_board {
            Some(prev_board) => {
                // Don't overwrite last_modified
                Board {name: board.name, wait_time: board.wait_time, 
                    full_images: board.full_images, archive: board.archive,
                    last_modified: prev_board.last_modified}
            },
            None => board
        };
        if !self.get_boards_set().await?.contains(&insert_board.name) {
            error!("Board /{}/ does not exist, skipping", insert_board.name);
            return Ok(0)
        }
        block_in_place(|| self.db_client.insert_board(&insert_board))
    }
    #[allow(dead_code)]
    pub async fn reset_board_state(&self, board_name: &String) -> anyhow::Result<usize> {
        let db_board = block_in_place(|| self.db_client.get_board(board_name))?;
        match db_board {
            Some(mut prev_board) => {
                // Reset last_modified
                prev_board.last_modified = 0;
                block_in_place(|| self.db_client.insert_board(&prev_board))
            },
            None => Ok(0)
        }
    }
    pub async fn get_all_boards(&self) -> anyhow::Result<BoardsList> {
        self.http_client.fetch_json::<BoardsList>("https://a.4cdn.org/boards.json").await
    }
    pub async fn get_boards_set(&self) -> anyhow::Result<HashSet<String>> {
        let boardslist = self.get_all_boards().await?;
        let mut name_set = HashSet::new();
        for board in boardslist.boards {
            name_set.insert(board.board);
        }
        Ok(name_set)
    }
}