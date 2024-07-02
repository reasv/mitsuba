use std::collections::HashSet;
use std::sync::Arc;
use anyhow::Ok;
#[allow(unused_imports)]
use log::{info, warn, error, debug};

use dashmap::DashSet;

mod board_archiver;
mod image_archiver;
mod thread_archiver;
mod archiver_metrics;

use crate::http::HttpClient;
use crate::models::{Board, BoardsList, PurgeReport};
use crate::db::DBClient;

#[derive(Clone)]
pub struct Archiver {
    pub http_client: HttpClient,
    pub db_client: DBClient,
    pub archived_ids: Arc<DashSet<u64>>
}


impl Archiver {
    pub async fn new(client: HttpClient) -> Self {
        Self {
            http_client: client,
            db_client: DBClient::new().await,
            archived_ids: Arc::new(DashSet::new())
        }
    }
    pub fn run_archivers(&self) -> tokio::task::JoinHandle<()> {
        self.run_metrics_cycle();
        self.run_board_cycle();
        self.run_thread_cycle();
        self.run_image_cycle()
    }
    pub async fn get_all_boards_api(&self) -> Result<BoardsList, bool> {
        self.http_client.fetch_json::<BoardsList>("https://a.4cdn.org/boards.json").await
    }
    pub async fn get_boards_set(&self) -> anyhow::Result<HashSet<String>> {
        let boardslist = self.get_all_boards_api().await.map_err(|_| anyhow::anyhow!(""))?;
        let mut name_set = HashSet::new();
        for board in boardslist.boards {
            name_set.insert(board.board);
        }
        Ok(name_set)
    }
    pub async fn set_board(&self, board: Board) -> anyhow::Result<Option<Board>> {
        if !self.get_boards_set().await?.contains(&board.name) {
            error!("Board /{}/ does not exist, skipping", board.name);
            return Ok(None)
        }
        // If full_images is being set to true, check if the board is already in the database
        if board.full_images {
            if let Some(b) = self.db_client.get_board(&board.name).await? {
                if !b.full_images {
                    // The board is being enabled for full images, but it's already in the database with full_images = false
                    let result = self.db_client.insert_board(&board).await?;
                    // We need to make sure existing posts have their full images downloaded
                    let jobs_scheduled = self.db_client.schedule_missing_full_files(&board.name).await?;
                    if jobs_scheduled > 0 {
                        info!("Scheduled {} missing full images for board /{}/", jobs_scheduled, board.name);
                    }
                    return Ok(Some(result))
                }
            }
        }
        Ok(Some(self.db_client.insert_board(&board).await?))
    }
    pub async fn stop_board(&self, board_name: &String) -> anyhow::Result<Option<Board>> {
        if let Some(mut board) = self.db_client.get_board(board_name).await? {
            board.archive = false;
            return Ok(Some(self.db_client.insert_board(&board).await?));
        }
        Ok(None)
    }
    pub async fn get_all_boards(&self) -> anyhow::Result<Vec<Board>> {
        self.db_client.get_all_boards().await
    }

    pub async fn purge_board(&self, board_name: &String, only_full_images: bool) -> anyhow::Result<PurgeReport> {
        let mut report = PurgeReport::default();
        if only_full_images {
            // If we only want to delete full images, we only disable full image downloads for the board
            if let Some(mut board) = self.db_client.get_board(board_name).await? {
                board.full_images = false;
                self.db_client.insert_board(&board).await?;
            }
            // If the board is not in the database, no need to stop full image downloads
        } else {
            // If we want to delete everything, we stop the board archiver entirely
            self.stop_board(board_name).await?;
            // Try to avoid more images being downloaded while we're purging the board
            self.db_client.purge_board_backlogs(board_name).await?;
        }
        
        let full_files = self.db_client.get_files_exclusive_to_board(board_name).await?;
        for file in &full_files {
            // Double check in case the file was added by another board while we were iterating
            if self.db_client.is_file_on_other_boards(&file.file_sha256, &file.ext, &board_name).await? {
                continue;
            }
            if self.http_client.delete_downloaded_file(&file.file_sha256, &file.ext, false).await.is_ok() {
                report.full_files_deleted += 1;
                self.db_client.set_file_purged(&file.file_sha256, &file.ext).await?;
            } else {
                report.full_files_failed += 1;
            }
        }

        if !only_full_images {
            let thumbnail_hashes = self.db_client.get_thumbnails_exclusive_to_board(board_name).await?;
            for hash in &thumbnail_hashes {
                // Double check in case the thumbnail was added by another board while we were iterating
                if self.db_client.is_thumbnail_on_other_boards(hash, &".jpg".to_string(), &board_name).await? {
                    continue;
                }
                if self.http_client.delete_downloaded_file(hash, &".jpg".to_string(), true).await.is_ok() {
                    report.thumbnails_deleted += 1;
                    self.db_client.set_thumbnail_purged(hash, &".jpg".to_string()).await?;
                } else {
                    report.thumbnails_failed += 1;
                }
            }
            let removed_posts = self.db_client.purge_board_data(board_name).await?;
            report.removed_posts = removed_posts;
        }
        Ok(report)
    }
    
}

impl std::panic::UnwindSafe for Archiver {}
impl std::panic::RefUnwindSafe for Archiver {}