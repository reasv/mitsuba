use std::collections::HashSet;
use std::sync::Arc;
#[allow(unused_imports)]
use log::{info, warn, error, debug};

use dashmap::DashSet;

mod board_archiver;
mod image_archiver;
mod thread_archiver;
mod archiver_metrics;

use crate::http::HttpClient;
use crate::models::{Board, BoardsList};
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
    
}

impl std::panic::UnwindSafe for Archiver {}
impl std::panic::RefUnwindSafe for Archiver {}