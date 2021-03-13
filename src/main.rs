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
use nonzero_ext::nonzero;
use crate::http::HttpClient;


#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();
    let client = Archiver::new(HttpClient::new(nonzero!(120u32), nonzero!(10u32), 200, 800, 600));

    let h = client.run_archive_cycle("po".to_string(), Duration::from_secs(5));
    h.await;
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
