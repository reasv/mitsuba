#[allow(unused_imports)]
use log::{info, warn, error, debug};

use actix_web::{get, web, App, HttpResponse, HttpServer, Result, middleware::NormalizePath};
use actix_files::NamedFile;
use tokio::fs::create_dir_all;
use mime_guess::from_path;

use crate::db::DBClient;
use crate::object_storage::ObjectStorage;
use crate::frontend::{thread_page, index_page_handler, board_page, build_handlebars, dist, home_page};
use crate::util::{get_file_folder, get_file_url};
use crate::models::{IndexPage, BoardsStatus};

#[get("/boards-status.json")]
async fn get_boards_status(db: web::Data<DBClient>) -> Result<HttpResponse, HttpResponse> {
    let boards = db.get_all_boards().await
        .map_err(|e| {
            error!("Error getting boards from DB: {}", e);
            HttpResponse::InternalServerError().finish()
        })?;
    Ok(HttpResponse::Ok().json(BoardsStatus{boards}))
}

#[get("/{board:[A-z0-9]+}/thread/{no:\\d+}.json")]
async fn get_thread(db: web::Data<DBClient>, info: web::Path<(String, i64)>) -> Result<HttpResponse, HttpResponse> {
    let (board, no) = info.into_inner();
    let thread = db.get_thread(&board, no).await
        .map_err(|e| {
            error!("Error getting thread from DB: {}", e);
            HttpResponse::InternalServerError().finish()
        })?
        .ok_or(HttpResponse::NotFound().finish())?;
    
    Ok(HttpResponse::Ok().json(thread))
}

#[get("/{board:[A-z0-9]+}/post/{no:\\d+}.json")]
async fn get_post(db: web::Data<DBClient>, info: web::Path<(String, i64)>) -> Result<HttpResponse, HttpResponse> {
    let (board, no) = info.into_inner();
    let post = db.get_post(&board, no).await
        .map_err(|e| {
            error!("Error getting post from DB: {}", e);
            HttpResponse::InternalServerError().finish()
        })?
        .ok_or(HttpResponse::NotFound().finish())?;
    Ok(HttpResponse::Ok().json(post))
}
#[get("/{board:[A-z0-9]+}/{idx:\\d+}.json")]
async fn get_index(db: web::Data<DBClient>, info: web::Path<(String, i64)>) -> Result<HttpResponse, HttpResponse> {
    let (board, mut index) = info.into_inner();
    if index > 0 {
        index -= 1;
    }
    let threads = db.get_thread_index(&board, index, 15).await
        .map_err(|e| {
            error!("Error getting post from DB: {}", e);
            HttpResponse::InternalServerError().finish()
        })?;
    Ok(HttpResponse::Ok().json(IndexPage {threads: threads.into_iter().map(|t| t.into()).collect()}))
}

#[get("/{board:[A-z0-9]+}/{tim:\\d+}.{ext}")]
async fn get_full_image(db: web::Data<DBClient>, info: web::Path<(String, i64, String)>) -> Result<NamedFile, HttpResponse> {
    let (board, tim, ext) = info.into_inner();
    get_image_from_tim(db, board, tim, ext, false).await
}

#[get("/{board:[A-z0-9]+}/{tim:\\d+}s.jpg")]
async fn get_thumbnail_image(db: web::Data<DBClient>, info: web::Path<(String, i64)>) -> Result<NamedFile, HttpResponse> {
    let (board, tim) = info.into_inner();
    get_image_from_tim(db, board, tim, "".to_string(), true).await
}

async fn get_image_from_tim(db: web::Data<DBClient>, board: String, tim: i64, ext: String, is_thumb: bool)-> Result<NamedFile, HttpResponse> {
    let sha256 = db.image_tim_to_sha256(&board, tim, is_thumb).await
        .map_err(|e| {
            error!("Error getting image from DB: {}", e);
            HttpResponse::InternalServerError().finish()
        })?
        .ok_or(HttpResponse::NotFound().finish())?;
    
    let filename = match is_thumb { 
        true => format!("{}.jpg", sha256),
        false => format!("{}.{}", sha256, ext)
    };
    let path = get_file_folder(&sha256, is_thumb).join(filename);
    NamedFile::open(path).map_err(|e| {
        error!("Error getting image from filesystem: {}", e);
        HttpResponse::NotFound().finish()
    })
}

#[get("/{board:[A-z0-9]+}/{tim:\\d+}.{ext}")]
async fn get_full_image_object_storage(db: web::Data<DBClient>, obc: web::Data<ObjectStorage>, info: web::Path<(String, i64, String)>) -> Result<HttpResponse, HttpResponse> {
    let (board, tim, ext) = info.into_inner();
    get_image_from_tim_object_storage(db, obc, board, tim, ext, false).await
}
#[get("/{board:[A-z0-9]+}/{tim:\\d+}s.jpg")]
async fn get_thumbnail_image_object_storage(db: web::Data<DBClient>, obc: web::Data<ObjectStorage>, info: web::Path<(String, i64)>) -> Result<HttpResponse, HttpResponse> {
    let (board, tim) = info.into_inner();
    get_image_from_tim_object_storage(db, obc, board, tim, "jpg".to_string(), true).await
}

async fn get_image_from_tim_object_storage(db: web::Data<DBClient>, obc: web::Data<ObjectStorage>, board: String, tim: i64, ext: String, is_thumb: bool) -> Result<HttpResponse, HttpResponse> {
    let sha256 = db.image_tim_to_sha256(&board, tim, is_thumb).await
        .map_err(|e| {
            error!("Error getting image from DB: {}", e);
            HttpResponse::InternalServerError().finish()
        })?
        .ok_or(HttpResponse::NotFound().finish())?;
    let path = get_file_url(&sha256, &(".".to_string()+&ext), is_thumb);

    get_file_object_storage(obc, &path).await
}

async fn get_file_object_storage_handler(obc: web::Data<ObjectStorage>, path: web::Path<String>) -> Result<HttpResponse, HttpResponse> {
    let path = &path.into_inner();
    get_file_object_storage(obc, &("/img/".to_string()+&path)).await
}

async fn get_file_object_storage(obc: web::Data<ObjectStorage>, path: &String) -> Result<HttpResponse, HttpResponse> {
    let (data, code) = obc.bucket.get_object(path).await.map_err(|e| {
        error!("Error getting file ({}) from bucket: {}", path, e);
        HttpResponse::InternalServerError().finish()
    })?;
    if code == 404 {
        Err(HttpResponse::NotFound().body("404 Not Found (Object storage)"))
    } else if code == 200 {
        let region = obc.bucket.url();
        debug!("{}{}", region, path);
        // Ok(HttpResponse::Ok().content_type(from_path(path).first_or_octet_stream().as_ref()).streaming(data))
        Ok(HttpResponse::Ok().content_type(from_path(path).first_or_octet_stream().as_ref()).body(data))
    } else {
        error!("Error getting file ({}) from bucket: {}", path, code);
        Err(HttpResponse::InternalServerError().finish())
    }
}
pub async fn web_main() -> std::io::Result<()> {
    let handlebars = build_handlebars();

    let handlebars_ref = web::Data::new(handlebars);
    let data_folder_str = std::env::var("DATA_ROOT").unwrap_or("data".to_string());
    let image_folder = format!("{}/images", data_folder_str);
    let port = std::env::var("WEB_PORT").unwrap_or("8080".to_string());
    let ip = std::env::var("WEB_IP").unwrap_or("0.0.0.0".to_string());
    create_dir_all(std::path::Path::new(&image_folder)).await.ok();
    let dbc = DBClient::new().await;
    HttpServer::new(move || {
        let obc = ObjectStorage::new();
        let mut app = App::new()
        .data(dbc.clone())
        .app_data(handlebars_ref.clone())
        .wrap(NormalizePath::default())
        .service(get_index)
        .service(get_thread)
        .service(get_post)
        .service(thread_page)
        .service(index_page_handler)
        .service(board_page)
        .service(get_boards_status)
        .service(home_page)
        .service(web::resource("/static/{_:.*}").route(web::get().to(dist)));

        if obc.enabled {
            app = app.service(web::resource("/img/{_:.*}").route(web::get().to(get_file_object_storage_handler)))
                    .data(obc)
                    .service(get_full_image_object_storage)
                    .service(get_thumbnail_image_object_storage);
        } else {
            app = app.service(actix_files::Files::new("/img", image_folder.clone()))
                .service(get_thumbnail_image)
                .service(get_full_image);
        }
        app
    })
    .bind(format!("{}:{}", ip, port))?
    .run()
    .await
}