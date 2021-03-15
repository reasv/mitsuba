#[allow(unused_imports)]
use log::{info, warn, error, debug};
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder, Result};
use actix_files::NamedFile;
use std::path::Path;

use crate::db::DBClient;
use crate::models::{Thread, Post};
use crate::board_archiver::base64_to_32;

#[get("/{board}/thread/{no}.json")]
async fn get_thread(db: web::Data<DBClient>, info: web::Path<(String, i64)>) -> impl Responder {
    let (board, no) = info.into_inner();

    let thread_res: Result<Option<Thread>, actix_web::HttpResponse> = web::block(move || db.get_thread(&board, no)).await
        .map_err(|e| {
            error!("Error getting thread from DB: {}", e);
            HttpResponse::InternalServerError().finish()
        });
    let thread = match thread_res? {
        Some(post) => post,
        None => return Err(HttpResponse::NotFound().finish())
    };
    Ok(HttpResponse::Ok().json(thread))
}
#[get("/{board}/post/{no}.json")]
async fn get_post(db: web::Data<DBClient>, info: web::Path<(String, i64)>) -> impl Responder {
    let (board, no) = info.into_inner();
    let b = board.clone();
    let post_res: Result<Option<Post>, actix_web::HttpResponse> = web::block(move || db.get_post(&b, no)).await
        .map_err(|e| {
            error!("Error getting post from DB: {}", e);
            HttpResponse::InternalServerError().finish()
        });
    let post = match post_res? {
        Some(post) => post,
        None => {
            warn!("404 Not Found for /{}/post/{}", board, no);
            return Err(HttpResponse::NotFound().finish())
        }
    };
    Ok(HttpResponse::Ok().json(post))
}
#[get("/{board}/{tim}.{ext}")]
async fn get_full_image(db: web::Data<DBClient>, info: web::Path<(String, i64, String)>) -> Result<NamedFile> {
    let (board, tim, ext) = info.into_inner();
    get_image(db, board, tim, ext, false).await
}

#[get("/{board}/{tim}s.jpg")]
async fn get_thumbnail_image(db: web::Data<DBClient>, info: web::Path<(String, i64)>) -> Result<NamedFile> {
    let (board, tim) = info.into_inner();
    get_image(db, board, tim, "".to_string(), false).await
}

async fn get_image(db: web::Data<DBClient>, board: String, tim: i64, ext: String, is_thumb: bool)-> Result<NamedFile> {
    let b = board.clone();
    let image_md5_res: Result<Option<String>, actix_web::HttpResponse> = web::block(move || db.image_tim_to_md5(&b, tim)).await
        .map_err(|e| {
            error!("Error getting post from DB: {}", e);
            HttpResponse::InternalServerError().finish()
        });
    
    
    let md5_base64 = match image_md5_res? {
        Some(md5) => md5,
        None => {
            warn!("404 Not Found for /{}/{}s.jpg", board, tim);
            return Err(actix_web::error::ErrorNotFound("Image Not Found"))
        }
    };
    let md5_base32 = base64_to_32(md5_base64).unwrap();
    let path = match is_thumb { 
        true => Path::new("data").join("images").join("thumb").join(format!("{}.jpg", md5_base32)),
        false => Path::new("data").join("images").join("full").join(format!("{}.{}", md5_base32, ext))
    };
    Ok(NamedFile::open(path)?)
}
#[actix_web::main]
pub async fn web_main() -> std::io::Result<()> {
    
    HttpServer::new(move || {
        let dbc: DBClient = DBClient::new();
        App::new().data(dbc)
        .service(get_thread)
        .service(get_post)
        .service(get_thumbnail_image)
        .service(get_full_image)
        .service(actix_files::Files::new("/img", "data/images"))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}