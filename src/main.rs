#![recursion_limit="128"]
use std::num::NonZeroU32;
use std::env;

#[allow(unused_imports)]
use log::{info, warn, error, debug};
use clap::Parser;

#[allow(dead_code)]
mod db;
mod models;
mod http;
mod util;
mod object_storage;
mod metric;
mod archiver;
mod web;

use web::web_main;
use archiver::Archiver;
use http::HttpClient;


#[derive(Parser)]
#[clap(version = "1.11", about="High performance board archiver software. Add boards with `add`, then start with `start` command. See `help add` and `help start`")]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand
}

#[derive(Parser, Clone)]
enum SubCommand {
    #[clap(about = "Start the archiver, API, and webserver")]
    Start(StartArc),
    #[clap(about = "Start in read only mode. Archivers will not run, only API and webserver")]
    StartReadOnly(ReadOnlyMode),
    #[clap(about = "Add a board to the archiver, or replace its settings.")]
    Add(Add),
    #[clap(about = "Stop and disable archiver for a particular board. Does not delete any data. Archiver will only stop after completing the current cycle.")]
    Remove(Remove),
    #[clap(about = "List all boards in the database and their current settings. Includes disabled ('removed') boards")]
    List(ListBoards),
    #[clap(about = "Purge all archive data from a specific board from the database. Use with caution.")]
    Purge(Purge),
    #[clap(about = "Hide a specific post/thread from the public webui and API. Nondestructive.")]
    Hide(Hide),
    #[clap(about = "Unhide a previously hidden post, making it visible again")]
    Unhide(Unhide),
    #[clap(about = "Delete the image or file associated with a specific post and blacklist it from being downloaded again. Does not delete the post itself.")]
    PurgeImage(DeleteImage),
    #[clap(about = "If the image associated with the given post is blacklisted, remove it from the blacklist. Does not restore the image.")]
    UnpurgeImage(Unhide),
    #[clap(about = "Add a new user to the database")]
    UserAdd(AddUser),
    #[clap(about = "Remove a user from the database")]
    UserRemove(RemoveUser),
    #[clap(about = "Change a user's password")]
    UserSetPassword(ChangePassword),
    #[clap(about = "Change a user's role")]
    UserSetRole(ChangeRole),
    #[clap(about = "List all users in the database")]
    UsersList,
}
#[derive(Parser, Default, Debug, Clone)]
struct AddUser {
    #[clap(help = "Username")]
    username: String,
    #[clap(help = "Role (admin, mod, janitor)")]
    role: String,
    #[clap(help = "Password")]
    password: String,
}

#[derive(Parser, Default, Debug, Clone)]
struct RemoveUser {
    #[clap(help = "Username")]
    username: String,
}

#[derive(Parser, Default, Debug, Clone)]
struct ChangePassword {
    #[clap(help = "Username")]
    username: String,
    #[clap(help = "New password")]
    password: String,
}

#[derive(Parser, Default, Debug, Clone)]
struct ChangeRole {
    #[clap(help = "Username")]
    username: String,
    #[clap(help = "New role (admin, mod, janitor)")]
    role: String,
}

#[derive(Parser, Default, Debug, Clone)]
struct StartArc {
    #[clap(long, long_help = "(Optional) If true, will only run the archiver and not the web ui or the web API. If false, run everything. Default is false.")]
    archiver_only: Option<bool>,
    #[clap(long, long_help = "(Optional) Max requests per minute. Default is 60.")]
    rpm: Option<NonZeroU32>,

    #[clap(long, long_help = "(Optional) Max burst of requests started at once. Default is 10.")]
    burst: Option<NonZeroU32>,

    #[clap(long, long_help = "(Optional) Minimum amount of jitter to add to rate-limiting to ensure requests don't start at the same time, in milliseconds. Default is 200ms", global=true)]
    jitter_min: Option<u32>,

    #[clap(long, long_help = "(Optional) Variance for jitter, in milliseconds. Default is 800ms. Jitter will be a random value between min and min+variance.")]
    jitter_variance: Option<u32>,

    #[clap(long, long_help = "(Optional) Maximum amount of time to spend retrying a failed image download, in seconds. Default is 600s (10m).")]
    max_time: Option<u32>
}

#[derive(Parser, Clone)]
struct ReadOnlyMode;
#[derive(Parser, Clone)]
struct Add {
    #[clap(help = "Board name (eg. 'po')")]
    name: String,
    #[clap(long, long_help = "(Optional) If false, will only download thumbnails for this board. If true, thumbnails and full images/files. Default is false.")]
    full_images: Option<bool>,
    #[clap(long, long_help = "(Optional) If true, will create a full text search index in postgres for this board. Default is false. Can be changed later.")]
    full_text_search: Option<bool>
}
#[derive(Parser, Clone)]
struct Remove {
    #[clap(help = "Board name (eg. 'po')")]
    name: String,
}
#[derive(Parser, Clone)]
struct ListBoards;

#[derive(Parser, Clone)]
struct Purge {
    #[clap(help = "Board name (eg. 'po')")]
    name: String,
    #[clap(long, long_help = "(Optional) If true, will only delete full images and files saved for this board, preserving posts and thumbnails. If false, everything, including posts and thumbnails. Default is false. If false, board must be disabled using the `remove` command first.")]
    only_purge_full_images: Option<bool>,
}

#[derive(Parser, Clone)]
struct Hide {
    #[clap(help = "Board name (eg. 'po')")]
    board_name: String,
    #[clap(help = "Post number (eg. 123456)")]
    post: i64,
    #[clap(long, long_help = "(Optional) If true, will hide the comment text (post body) rather than the whole post. Default is false.")]
    hide_comment: Option<bool>,
    #[clap(long, long_help = "(Optional) If true, will hide the image and thumbnail rather than the whole post. Default is false.")]
    hide_image: Option<bool>
}

#[derive(Parser, Clone)]
struct Unhide {
    #[clap(help = "Board name (eg. 'po')")]
    board_name: String,
    #[clap(help = "Post number (eg. 123456)")]
    post: i64
}

#[derive(Parser, Clone)]
struct DeleteImage {
    #[clap(help = "Board name (eg. 'po')")]
    board_name: String,
    #[clap(help = "Post number (eg. 123456)")]
    post: i64,
    #[clap(long, long_help = "(Optional) Reason for deletion. Will be logged in the database.")]
    reason: Option<String>
}


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
    if let Err(err) = log4rs::init_file("log4rs.yml", Default::default()) {
        println!("Did not initialize log4rs ({:?}), fallback to env_logger.\nIf you want to use log4rs, make sure to create a valid log4rs.yml file.", err);
        env_logger::init();
    }

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

    let new_admin_password = env::var("ADMIN_PASSWORD").ok();

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
            if let Some(password) = new_admin_password {
                client.ensure_admin_exists(&password).await.unwrap();
            }
            web_main(client).await.unwrap();
        },
        SubCommand::Start(arcopts) => {
            // Metrics are only for the archiver, for now.
            // Starting them earlier makes using the cli tools impossible
            // while the archiver is running. Metrics would try to bind to the same port.
            metric::init_metrics();
            let handle = client.run_archivers();
            if arcopts.archiver_only.unwrap_or_default() {
                handle.await.ok();
            } else {
                if let Some(password) = new_admin_password {
                    client.ensure_admin_exists(&password).await.unwrap();
                }
                web_main(client).await.unwrap();
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
                archive: true,
                enable_search: add_opt.full_text_search.unwrap_or(false)
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
        SubCommand::Purge(purge_opt) => {
            let only_full_images = purge_opt.only_purge_full_images.unwrap_or(false);
            let board = purge_opt.name;
            if only_full_images {
                println!("Purging full images and files for /{}/ from disk", board);
            } else {
                if client.is_board_enabled(&board).await.unwrap() {
                    println!("You must disable archiving of {} using the `remove` command before proceeding with a purge.", board);
                    return;
                }
                println!("Purging all data for /{}/", board);
            }
            let report = client.purge_board(&board, only_full_images).await.unwrap();
            println!("Purged {} posts, {} thumbnails, {} full images / files", report.removed_posts, report.thumbnails_deleted, report.full_files_deleted);
            println!("Failed to delete {} thumbnails and {} full images/files", report.thumbnails_failed, report.full_files_failed);
        },
        SubCommand::Hide(hide_opt) => {
            let board = hide_opt.board_name;
            let post = hide_opt.post;
            let hide_comment = hide_opt.hide_comment.unwrap_or(false);
            let hide_image = hide_opt.hide_image.unwrap_or(false);
            let hide_post = !hide_comment && !hide_image;

            let log_id = client.db_client
                .create_moderation_log_entry(None,None, None)
                .await.unwrap();
            
            client.hide_post(
                &board,
                post,
                Some(hide_post),
                hide_opt.hide_comment,
                hide_opt.hide_image,
                log_id)
            .await.unwrap();
            let hide_post = !hide_comment && !hide_image;
            println!("Hid post /{}/{} (Entire post hidden: {}, Only comment field hidden: {}, Only image hidden: {})", board, post, hide_post, hide_comment, hide_image);
        },
        SubCommand::Unhide(unhide_opt) => {
            let board = unhide_opt.board_name;
            let post = unhide_opt.post;

            let log_id = client.db_client
                .create_moderation_log_entry(None,None, None)
                .await.unwrap();

            client.unhide_post(&board, post, log_id).await.unwrap();
            println!("Unhid post /{}/{}", board, post);
        },
        SubCommand::PurgeImage(ban_image_opt) => {
            let board = ban_image_opt.board_name;
            let post = ban_image_opt.post;
            
            let log_id = client.db_client
                .create_moderation_log_entry(
                    None,
                    ban_image_opt.reason,
                    None
                )
                .await.unwrap();
            let image_hashes = client
                .ban_image(
                    &board,
                    post,
                    log_id
                ).await.unwrap();
            for sha256 in image_hashes {
                println!("Purged image {} for post /{}/{}", sha256, board, post);
            }
        },
        SubCommand::UnpurgeImage(unban_image_opt) => {
            let board = unban_image_opt.board_name;
            let post = unban_image_opt.post;
            let log_id = client.db_client
                .create_moderation_log_entry(
                    None,
                    None,
                    None
                )
                .await.unwrap();
            let image_hashes = client
                .unban_image(
                    &board,
                    post,
                    log_id
                 ).await.unwrap();
            for sha256 in image_hashes {
                println!("Unpurged image {} for post /{}/{}", sha256, board, post);
            }
        },
        SubCommand::UserAdd(user_add) => {
            let role = match user_add.role.as_str() {
                "admin" => models::UserRole::Admin,
                "mod" => models::UserRole::Mod,
                "janitor" => models::UserRole::Janitor,
                _ => {
                    println!("Invalid role. Valid roles are: admin, mod, janitor");
                    return;
                }
            };
            client.add_user(&user_add.username, &user_add.password, role).await.unwrap();
            println!("Added user {}", user_add.username);
        },
        SubCommand::UserRemove(user_remove) => {
            client.delete_user(&user_remove.username).await.unwrap();
            println!("Removed user {}", user_remove.username);
        },
        SubCommand::UserSetPassword(user_set_password) => {
            client.change_password(&user_set_password.username, &user_set_password.password).await.unwrap();
            println!("Changed password for user {}", user_set_password.username);
        },
        SubCommand::UserSetRole(user_set_role) => {
            let role = match user_set_role.role.as_str() {
                "admin" => models::UserRole::Admin,
                "mod" => models::UserRole::Mod,
                "janitor" => models::UserRole::Janitor,
                _ => {
                    println!("Invalid role. Valid roles are: admin, mod, janitor");
                    return;
                }
            };
            client.change_role(&user_set_role.username, role).await.unwrap();
            println!("Changed role for user {}", user_set_role.username);
        },
        SubCommand::UsersList => {
            let users = client.db_client.get_users().await.unwrap();
            for user in users.iter() {
                println!("User: {} Role: {}", user.name, user.role);
            }
            println!("{} users found in database", users.len());
        }
    }
}
