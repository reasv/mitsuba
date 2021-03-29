#[allow(unused_imports)]
use log::{info, warn, error, debug};

use actix_web::{get, web, App, HttpResponse, HttpServer, Result, middleware::NormalizePath};
use actix_files::NamedFile;
use tokio::fs::create_dir_all;

use crate::db::DBClient;
use crate::frontend::{thread_page, index_page, build_handlebars, dist};
use crate::util::{base64_to_32, get_image_folder};
use crate::models::{IndexPage, BoardsStatus};

#[get("/{board}/thread/{no}.json")]
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

#[get("/{board}/post/{no}.json")]
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
#[get("/{board}/{idx}.json")]
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

#[get("/{board}/{tim}.{ext}")]
async fn get_full_image(db: web::Data<DBClient>, info: web::Path<(String, i64, String)>) -> Result<NamedFile, HttpResponse> {
    let (board, tim, ext) = info.into_inner();
    get_image(db, board, tim, ext, false).await
}

#[get("/{board}/{tim}s.jpg")]
async fn get_thumbnail_image(db: web::Data<DBClient>, info: web::Path<(String, i64)>) -> Result<NamedFile, HttpResponse> {
    let (board, tim) = info.into_inner();
    get_image(db, board, tim, "".to_string(), true).await
}
#[get("/boards-status.json")]
async fn get_boards_status(db: web::Data<DBClient>) -> Result<HttpResponse, HttpResponse> {
    let boards = db.get_all_boards().await
        .map_err(|e| {
            error!("Error getting boards from DB: {}", e);
            HttpResponse::InternalServerError().finish()
        })?;
    Ok(HttpResponse::Ok().json(BoardsStatus{boards}))
}

async fn get_image(db: web::Data<DBClient>, board: String, tim: i64, ext: String, is_thumb: bool)-> Result<NamedFile, HttpResponse> {
    let md5_base64 = db.image_tim_to_md5(&board, tim).await
        .map_err(|e| {
            error!("Error getting image from DB: {}", e);
            HttpResponse::InternalServerError().finish()
        })?
        .ok_or(HttpResponse::NotFound().finish())?;

    let md5_base32 = base64_to_32(md5_base64.clone()).unwrap();
    let path = match is_thumb { 
        true => get_image_folder(&md5_base64, true).join(format!("{}.jpg", md5_base32)),
        false => get_image_folder(&md5_base64, false).join(format!("{}.{}", md5_base32, ext))
    };
    NamedFile::open(path).map_err(|e| {
        error!("Error getting image from filesystem: {}", e);
        HttpResponse::NotFound().finish()
    })
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
        App::new()
        .data(dbc.clone())
        .app_data(handlebars_ref.clone())
        .wrap(NormalizePath::default())
        .service(get_index)
        .service(get_thread)
        .service(get_post)
        .service(get_thumbnail_image)
        .service(get_full_image)
        .service(thread_page)
        .service(index_page)
        .service(get_boards_status)
        .service(actix_files::Files::new("/img", image_folder.clone()))
        .service(web::resource("/static/{_:.*}").route(web::get().to(dist)))
    })
    .bind(format!("{}:{}", ip, port))?
    .run()
    .await
}