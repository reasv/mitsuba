#![recursion_limit="128"]

#[allow(unused_imports)]
use log::{info, warn, error, debug};
use nonzero_ext::nonzero;

#[macro_use]
extern crate diesel;

mod board_archiver;
#[allow(dead_code)]
mod db;
pub mod models;
pub mod schema;
pub mod http;
mod api;
use api::web_main;
use board_archiver::{Archiver};
use http::HttpClient;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();
    let client = Archiver::new(HttpClient::new(nonzero!(120u32), nonzero!(10u32), 200, 800, 600));

    // use models::Board;
    // client.set_board(Board{ name: "w".to_string(), wait_time: 10, full_images: false, last_modified: 0, archive: true}).await.unwrap();
    
    client.run_archivers().await.unwrap();
    web_main().unwrap();
}
