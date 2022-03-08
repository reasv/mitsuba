#[allow(unused_imports)]
use log::{info, warn, error, debug};

use actix_web::{web, App, HttpServer, middleware::NormalizePath};
use tokio::fs::create_dir_all;

use crate::db::DBClient;
use crate::object_storage::ObjectStorage;

mod api;
mod frontend;

pub async fn web_main() -> std::io::Result<()> {
    let handlebars = frontend::build_handlebars();

    let handlebars_ref = web::Data::new(handlebars);
    let data_folder_str = std::env::var("DATA_ROOT").unwrap_or("data".to_string());
    let image_folder = format!("{}/images", data_folder_str);
    let port = std::env::var("WEB_PORT").unwrap_or("8080".to_string());
    let ip = std::env::var("WEB_IP").unwrap_or("0.0.0.0".to_string());
    info!("Web adress: {}:{}", ip, port);
    create_dir_all(std::path::Path::new(&image_folder)).await.ok();
    let dbc = DBClient::new().await;
    HttpServer::new(move || {
        let obc = ObjectStorage::new();
        let mut app = App::new()
        .app_data(web::Data::new(dbc.clone()))
        .app_data(handlebars_ref.clone())
        .wrap(NormalizePath::default())
        .service(api::get_index)
        .service(api::get_thread)
        .service(api::get_post)
        .service(frontend::thread_page)
        .service(frontend::index_page_handler)
        .service(frontend::board_page)
        .service(api::get_boards_status)
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
