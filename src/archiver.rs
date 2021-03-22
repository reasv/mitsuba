use std::sync::Arc;
use std::collections::HashSet;
#[allow(unused_imports)]
use log::{info, warn, error, debug};

use crate::http::HttpClient;
use crate::models::{Board, BoardsList};
use crate::db::DBClient;

#[derive(Clone)]
pub struct Archiver {
    pub http_client: Arc<HttpClient>,
    pub db_client: DBClient
}


impl Archiver {
    pub async fn new(client: HttpClient) -> Self {
        Self {
            http_client: Arc::new(client),
            db_client: DBClient::new().await
        }
    }
    pub fn run_archivers(&self) -> tokio::task::JoinHandle<()> {
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
    pub async fn get_all_boards(&self) -> anyhow::Result<Vec<Board>> {
        self.db_client.get_all_boards_async().await
    }
    
}