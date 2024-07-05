use actix_web::middleware;
use base64::prelude::*;
#[allow(unused_imports)]
use log::{info, warn, error, debug};

use actix_web::{web, App, HttpServer, middleware::NormalizePath};
use actix_session::{
    SessionMiddleware,
    storage::CookieSessionStore,
    config::CookieContentSecurity,
    config::TtlExtensionPolicy,
    config::SessionLifecycle,
    config::PersistentSession
};

use actix_web::cookie::Key;
use s3::creds::time::Duration;
use tokio::fs::create_dir_all;

use crate::archiver::Archiver;
use crate::object_storage::ObjectStorage;

mod api;
mod frontend;
mod auth;

fn load_or_generate_key(data_folder_str: &String) -> actix_web::cookie::Key {
    let secret_seed_path = format!("{}/cookie_secret_seed", data_folder_str);
    let secret_seed = match std::fs::read_to_string(&secret_seed_path) {
        Ok(seed) => seed,
        Err(_) => {
            let seed = BASE64_STANDARD.encode(rand::random::<[u8; 32]>());
            std::fs::write(&secret_seed_path, &seed).unwrap();
            seed
        }
    };
    let secret_key = Key::derive_from(&secret_seed.as_bytes());
    secret_key
}

pub async fn web_main(archiver: Archiver) -> std::io::Result<()> {
    let dbc = archiver.db_client.clone();

    let handlebars = frontend::build_handlebars();

    let handlebars_ref = web::Data::new(handlebars);
    let data_folder_str = std::env::var("DATA_ROOT").unwrap_or("data".to_string());
    let secret_key = load_or_generate_key(&data_folder_str);
    let image_folder = format!("{}/images", data_folder_str);
    let port = std::env::var("WEB_PORT").unwrap_or("8080".to_string());
    let ip = std::env::var("WEB_IP").unwrap_or("0.0.0.0".to_string());
    info!("Web adress: {}:{}", ip, port);
    create_dir_all(std::path::Path::new(&image_folder)).await.ok();
    HttpServer::new(move || {
        let obc = ObjectStorage::new();
        let mut app = App::new()
        .wrap(
            SessionMiddleware::builder(
                CookieSessionStore::default(),
                secret_key.clone()
            )
            .cookie_content_security(CookieContentSecurity::Private)
            .session_lifecycle(SessionLifecycle::PersistentSession(
                PersistentSession::default()
                .session_ttl_extension_policy(
                    TtlExtensionPolicy::OnEveryRequest
                )
                .session_ttl(Duration::days(1))
            ))
            .build()
        )
        .app_data(web::Data::new(dbc.clone()))
        .app_data(web::Data::new(archiver.clone()))
        .app_data(handlebars_ref.clone())
        .wrap(NormalizePath::new(middleware::TrailingSlash::Trim))
        .wrap(middleware::Compress::default())
        .service(api::get_index)
        .service(api::get_thread)
        .service(api::get_post)
        .service(api::login_api)
        .service(api::logout_api)
        .service(api::authcheck_api)
        .service(frontend::login_page)
        .service(frontend::thread_page)
        .service(frontend::index_page_handler)
        .service(frontend::board_page)
        .service(api::get_boards_status)
        .service(api::get_users)
        .service(api::post_user)
        .service(api::put_user)
        .service(api::delete_user)
        .service(api::put_current_user)
        .service(api::put_board)
        .service(api::delete_board)
        .service(api::put_post)
        .service(frontend::home_page)
        .service(web::resource("/static/{_:.*}").route(web::get().to(frontend::dist)));

        if obc.enabled {
            app = app.service(web::resource("/img/{_:.*}").route(web::get().to(api::get_file_object_storage_handler)))
                    .app_data(web::Data::new(obc))
                    .service(api::get_full_image_object_storage)
                    .service(api::get_thumbnail_image_object_storage);
        } else {
            app = app.service(actix_files::Files::new("/img", image_folder.clone()))
                .service(api::get_thumbnail_image)
                .service(api::get_full_image);
        }
        app
    })
    .bind(format!("{}:{}", ip, port))?
    .run()
    .await
}
