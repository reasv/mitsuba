#![recursion_limit="128"]
use std::num::NonZeroU32;
use std::env;
#[allow(unused_imports)]
use log::{info, warn, error, debug};

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

fn get_env(name: &str, def: u32) -> u32 {
    match env::var(name) {
        Err(_) => {
            debug!("Using default value for {}", name);
            def
        }
        Ok(str_value) => str_value.parse().unwrap()
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();
    let rpm = get_env("RATE_LIMIT_QUOTA_PER_MINUTE", 120);
    let burst = get_env("RATE_LIMIT_BURST", 10);
    let jitter_min = get_env("RATE_LIMIT_JITTER_MIN_MS", 200);
    let jitter_interval = get_env("RATE_LIMIT_JITTER_INTERVAL_MS", 800);
    let retry_max_time = get_env("RETRY_FAILED_MAX_TIME_SECONDS", 600);

    let client = Archiver::new(
        HttpClient::new(
            NonZeroU32::new(rpm).unwrap(), 
            NonZeroU32::new(burst).unwrap(), 
            jitter_min.into(), 
            jitter_interval.into(), 
            retry_max_time.into()
        )
    );
    let connection = client.db_client.pool.get().unwrap();
    embedded_migrations::run_with_output(&connection, &mut std::io::stdout()).unwrap();

    use models::Board;
    client.set_board(Board{ name: "i".to_string(), wait_time: 10, full_images: true, last_modified: 0, archive: true}).await.unwrap();
    
    client.run_archivers().await.unwrap();
    web_main().unwrap();
}
