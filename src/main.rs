#![recursion_limit="128"]

#[allow(unused_imports)]
use log::{info, warn, error, debug};
use nonzero_ext::nonzero;

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

mod board_archiver;
#[allow(dead_code)]
mod db;
mod models;
mod schema;
mod http;
mod api;
use api::web_main;
use board_archiver::{Archiver};
use http::HttpClient;

embed_migrations!();

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();
    let client = Archiver::new(HttpClient::new(nonzero!(120u32), nonzero!(10u32), 200, 800, 600));
    let connection = client.db_client.pool.get().unwrap();
    embedded_migrations::run_with_output(&connection, &mut std::io::stdout()).unwrap();

    use models::Board;
    client.set_board(Board{ name: "i".to_string(), wait_time: 10, full_images: true, last_modified: 0, archive: true}).await.unwrap();
    
    client.run_archivers().await.unwrap();
    web_main().unwrap();
}
