#![recursion_limit="128"]

use std::time::Duration;

#[allow(unused_imports)]
use log::{info, warn, error, debug};

#[macro_use]
extern crate diesel;

mod board_archiver;
mod db;
pub mod models;
pub mod schema;
pub mod http;

use board_archiver::{Archiver};
use models::Board;
use nonzero_ext::nonzero;
use crate::http::HttpClient;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();
    let client = Archiver::new(HttpClient::new(nonzero!(120u32), nonzero!(10u32), 200, 800, 600));

    client.add_board(Board{ name: "po".to_string(), wait_time: 10, full_images: false, last_modified: 0, archive: true}).await.unwrap();
    
    client.run_archivers().await.unwrap();
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
