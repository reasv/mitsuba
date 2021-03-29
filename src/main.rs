#![recursion_limit="128"]
use std::num::NonZeroU32;
use std::env;

#[allow(unused_imports)]
use log::{info, warn, error, debug};
use clap::{Clap};

mod board_archiver;
#[allow(dead_code)]
mod db;
mod models;
mod http;
mod api;
mod frontend;
mod util;
mod thread_archiver;
mod image_archiver;
mod archiver;
use api::web_main;
use archiver::{Archiver};
use http::HttpClient;


#[derive(Clap)]
#[clap(version = "1.0", about="High performance board archiver software. Add boards with `add`, then start with `start` command. See `help add` and `help start`")]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand
}

#[derive(Clap, Clone)]
enum SubCommand {
    #[clap(about = "Start the archiver, API, and webserver")]
    Start(StartArc),
    #[clap(about = "Start in read only mode. Archivers will not run, only API and webserver")]
    StartReadOnly(ReadOnlyMode),
    #[clap(about = "Add a board to the archiver, or replace its settings.")]
    Add(Add),
    #[clap(about = "Stop archiver for a board. Does not delete any data, does not reset the board. Archiver will only stop after completing the current cycle.")]
    Remove(Remove),
    #[clap(about = "List all boards in the database and their current settings. Includes stopped ('removed') boards")]
    List(ListBoards)
}

#[derive(Clap, Default, Debug, Clone)]
struct StartArc {
    #[clap(long, long_about = "(Optional) If true, will only run the archiver and not the web ui or the web API. If false, run everything. Default is false.")]
    archiver_only: Option<bool>,
    #[clap(long, long_about = "(Optional) Max requests per minute. Default is 60.")]
    rpm: Option<NonZeroU32>,

    #[clap(long, long_about = "(Optional) Max burst of requests started at once. Default is 10.")]
    burst: Option<NonZeroU32>,

    #[clap(long, long_about = "(Optional) Minimum amount of jitter to add to rate-limiting to ensure requests don't start at the same time, in milliseconds. Default is 200ms", global=true)]
    jitter_min: Option<u32>,

    #[clap(long, long_about = "(Optional) Variance for jitter, in milliseconds. Default is 800ms. Jitter will be a random value between min and min+variance.")]
    jitter_variance: Option<u32>,

    #[clap(long, long_about = "(Optional) Maximum amount of time to spend retrying a failed image download, in seconds. Default is 60s (10m).")]
    max_time: Option<u32>
}

#[derive(Clap, Clone)]
struct ReadOnlyMode;
#[derive(Clap, Clone)]
struct Add {
    #[clap(about = "Board name (eg. 'po')")]
    name: String,
    #[clap(long, long_about = "(Optional) If false, will only download thumbnails for this board. If true, thumbnails and full images. Default is false.")]
    full_images: Option<bool>,
}
#[derive(Clap, Clone)]
struct Remove {
    #[clap(about = "Board name (eg. 'po')")]
    name: String,
}
#[derive(Clap, Clone)]
struct ListBoards;


fn get_env(name: &str, def: u32) -> u32 {
    match env::var(name) {
        Err(_) => {
            debug!("Using default value for {}", name);
            def
        }
        Ok(str_value) => str_value.parse().unwrap()
    }
}

fn main() {
    actix_web::rt::System::with_tokio_rt(||
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
    ).block_on(async {
        real_main().await
    })
}

async fn real_main() {
    dotenv::dotenv().ok();
    env_logger::init();

    let opts: Opts = Opts::parse();
    let arc_opt = match opts.subcmd.clone() {
        SubCommand::Start(a) => a,
        _ => StartArc::default(),
    };
    let rpm = match arc_opt.rpm {
        Some(r) => r,
        None => NonZeroU32::new(get_env("RATE_LIMIT_QUOTA_PER_MINUTE", 120)).unwrap()
    };
    let burst = match arc_opt.burst {
        Some(r) => r,
        None => NonZeroU32::new(get_env("RATE_LIMIT_BURST", 10)).unwrap()
    };
    let jitter_min = match arc_opt.jitter_min {
        Some(r) => r,
        None => get_env("RATE_LIMIT_JITTER_MIN_MS", 200)
    };
    let jitter_variance = match arc_opt.jitter_variance {
        Some(r) => r,
        None => get_env("RATE_LIMIT_JITTER_INTERVAL_MS", 800)
    };
    let retry_max_time = match arc_opt.max_time {
        Some(r) => r,
        None => get_env("RETRY_FAILED_MAX_TIME_SECONDS", 600)
    };
    let client = Archiver::new(
        HttpClient::new(
            rpm,
            burst,
            jitter_min.into(),
            jitter_variance.into(),
            retry_max_time.into()
        )
    ).await;
    sqlx::migrate!().run(&client.db_client.pool).await.expect("Failed to migrate");
    
    match opts.subcmd {
        SubCommand::StartReadOnly(_) => {
            web_main().await.unwrap();
        },
        SubCommand::Start(arcopts) => {
            let handle = client.run_archivers();
            if arcopts.archiver_only.unwrap_or_default() {
                handle.await.ok();
            } else {
                web_main().await.unwrap();
            }
        },
        SubCommand::Remove(remove_opt) => {
            client.stop_board(&remove_opt.name).await.unwrap();
            println!("Disabled /{}/", remove_opt.name);
        },
        SubCommand::Add(add_opt) => {
            use models::Board;
            let board = Board { 
                name: add_opt.name, 
                full_images: add_opt.full_images.unwrap_or(false),
                archive: true
            };
            client.set_board(board.clone()).await.unwrap();
            println!("Added /{}/ Enabled: {}, Full Images: {}",
                board.name, board.archive, board.full_images);
        }
        SubCommand::List(_) => {
            let boards = client.get_all_boards().await.unwrap();
            for board in boards.iter() {
                println!("/{}/ Enabled: {}, Full Images: {}",
                board.name, board.archive, board.full_images);
            }
            println!("{} boards found in database", boards.len());

        }
    }
}
