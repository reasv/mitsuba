use std::path::Path;
use std::time::Duration;
use std::sync::Arc;
use std::convert::TryInto;
use std::collections::HashSet;
#[allow(unused_imports)]
use log::{info, warn, error, debug};
use tokio::fs::create_dir_all;

use crate::http::HttpClient;
use crate::models::{Thread, ThreadInfo, ThreadsPage, ImageInfo, Image, Board, BoardsList, ImageJob, Post};
use crate::db::DBClient;
use crate::util::{get_board_page_api_url,get_thread_api_url,base64_to_32, get_image_folder};

#[derive(Clone)]
pub struct Archiver {
    pub http_client: Arc<HttpClient>,
    pub db_client: DBClient
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
        self.http_client.fetch_json::<Thread>(&get_thread_api_url(board, tid)).await
        .map_err(|e| {error!("Could not get thread /{}/{} (Error: {:?})", board, tid, e); e})
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
    
    pub async fn archive_cycle(&self, board: String) -> anyhow::Result<bool> {
        let board_object = self.db_client.get_board_async(&board).await
        .map_err(|e| {error!("Error getting board from database: {}", e); e})?.unwrap();

        if !board_object.archive {
            info!("Stopping archiver for {}", board_object.name);
            return Ok(false);
        }

        let last_change = board_object.last_modified;
        info!("Fetching new thread changes for board /{}/", board);
        let (thread_infos, new_last_change) = self.get_all_thread_info_since(&board, last_change).await
        .map_err(|e| {error!("Failed to fetch thread changes for /{}/: {}", board, e); e})?;

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
        for handle in handles {
            // The task returns the last modified of thread, plus thread if it succeeded, None if failed
            let thread_res = handle.await?;
            if thread_res.is_err() { // Thread not fetched successfully
                let last_modified = thread_res.unwrap_err();
                // next time resume archiving from before earliest failed thread
                if current_last_change > last_modified {
                    current_last_change = last_modified - 1;
                }
            }
        }
        let mut new_board_object = self.db_client.get_board_async(&board).await
        .map_err(|e| {error!("Error getting board from database: {}", e); e})?.unwrap();
       
        new_board_object.last_modified = current_last_change;
        self.db_client.insert_board_async(&new_board_object).await?;
        Ok(true)
    }
    pub async fn archive_thread(&self, board: String, info: ThreadInfo) -> Result<(), i64> {
        let thread = self.get_thread(&board, &info.no.to_string()).await
        .map_err(|e| {error!("Failed to fetch thread /{}/{}: {}", board, info.no, e); info.last_modified})?;

        let posts: Vec<Post> = thread.posts.clone().into_iter().map(|mut post|{post.board = board.clone(); post}).collect();
        let image_infos = posts.iter().filter_map(|post| self.get_post_image_info(&board,post)).collect::<Vec<ImageInfo>>();

        self.db_client.insert_posts_async(&posts).await
        .map_err(|e| {error!("Failed to insert thread /{}/{} into database: {}", board, info.no, e); info.last_modified}).ok();

        for image_info in image_infos {
            self.db_client.insert_image_job_async(&image_info).await
            .map_err(|e| {error!("Failed to insert image job /{}/{} into database: {}", board, image_info.md5.clone(), e); info.last_modified}).ok();
        }
        Ok(())
    }
    pub async fn image_cycle(&self) -> Result<(),()> {
        let image_jobs = self.db_client.get_image_jobs_async().await
        .map_err(|e|{error!("Failed to get new image jobs from database: {}", e);})?;

        info!("Running image cycle for {} new jobs", image_jobs.len());
        let boards = self.db_client.get_all_boards_async().await
        .map_err(|e| {error!("Failed to get boards from database: {}", e);})?;

        let mut full_images = HashSet::new();
        for board in boards {
            if board.full_images{
                full_images.insert(board.name);
            }
        }
        // let data_folder_str = std::env::var("DATA_ROOT").unwrap_or("data".to_string());
        // let image_folder = Path::new(&data_folder_str).join("images");
        // create_dir_all(image_folder.join("thumb")).await.ok();
        // create_dir_all(image_folder.join("full")).await.ok();
        let mut handles = Vec::new();
        for job in image_jobs {
            let c = self.clone();
            let need_full_image = full_images.contains(&job.board);
            // let folder = image_folder.clone();
            handles.push(tokio::task::spawn(
                async move {
                    c.archive_image(&job.clone(), need_full_image).await
                }
            ))
        }
        for handle in handles {
            handle.await.ok();
        }
        Ok(())
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
    pub async fn run_image_cycle(&self) -> tokio::task::JoinHandle<()> {
        let c = self.clone();
        tokio::task::spawn(async move { 
            loop {
                c.image_cycle().await.ok();
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        })
    }
    pub async fn run_archive_cycle(&self, board: &Board) -> tokio::task::JoinHandle<()>{
        let c = self.clone();
        let board_name = board.name.clone();
        let wait_time = board.wait_time.clone();
        tokio::task::spawn(async move {
            loop {
                let continue_res = c.clone().archive_cycle(board_name.clone()).await;
                if !continue_res.unwrap_or(true) { // We always continue on error
                    break;
                }
                tokio::time::sleep(Duration::from_secs(wait_time.try_into().unwrap_or(10u64))).await;
            }
        })
    }
    pub async fn run_archivers(&self) -> anyhow::Result<()> {
        let boards = self.db_client.get_all_boards_async().await?;
        for board in boards {
            if !board.archive {continue};
            self.run_archive_cycle(&board).await;
        }
        self.run_image_cycle().await;
        Ok(())
    }
    
    pub async fn set_board(&self, board: Board) -> anyhow::Result<usize> {
        let db_board = self.db_client.get_board_async(&board.name).await?;
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
        self.db_client.insert_board_async(&insert_board).await
    }
    pub async fn stop_board(&self, board_name: &String) -> anyhow::Result<usize> {
        let db_board = self.db_client.get_board_async(board_name).await?;
        let insert_board = match db_board {
            Some(mut prev_board) => {
                prev_board.archive = false;
                prev_board
            },
            None => return Ok(0)
        };
        self.db_client.insert_board_async(&insert_board).await
    }
    
    pub async fn reset_board_state(&self, board_name: &String) -> anyhow::Result<usize> {
        let db_board = self.db_client.get_board_async(board_name).await?;
        match db_board {
            Some(mut prev_board) => {
                // Reset last_modified
                prev_board.last_modified = 0;
                self.db_client.insert_board_async(&prev_board).await
            },
            None => Ok(0)
        }
    }
    pub async fn get_all_boards_api(&self) -> anyhow::Result<BoardsList> {
        self.http_client.fetch_json::<BoardsList>("https://a.4cdn.org/boards.json").await
    }
    pub async fn get_all_boards(&self) -> anyhow::Result<Vec<Board>> {
        self.db_client.get_all_boards_async().await
    }
    pub async fn get_boards_set(&self) -> anyhow::Result<HashSet<String>> {
        let boardslist = self.get_all_boards_api().await?;
        let mut name_set = HashSet::new();
        for board in boardslist.boards {
            name_set.insert(board.board);
        }
        Ok(name_set)
    }
}