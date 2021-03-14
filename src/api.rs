#[allow(unused_imports)]
use log::{info, warn, error, debug};
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};

use crate::db::DBClient;
use crate::models::{Thread, Post};

#[get("/{board}/thread/{no}.json")]
async fn get_thread(db: web::Data<DBClient>, info: web::Path<(String, i64)>) -> impl Responder {
    let (board, no) = info.into_inner();

    let thread_res: Result<Option<Thread>, actix_web::HttpResponse> = web::block(move || db.get_thread(&board, no)).await
        .map_err(|e| {
            error!("Error getting thread from DB: {}", e);
            HttpResponse::InternalServerError().finish()
        });
    
    // let post_op: std::option::Option<models::Thread> = thread_res?;
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
    
    // let post_op: std::option::Option<models::Thread> = thread_res?;
    let post = match post_res? {
        Some(post) => post,
        None => {
            warn!("404 Not Found for /{}/post/{}", board, no);
            return Err(HttpResponse::NotFound().finish())
        }
    };
    Ok(HttpResponse::Ok().json(post))
}

#[actix_web::main]
pub async fn web_main() -> std::io::Result<()> {
    
    HttpServer::new(move || {
        let dbc: DBClient = DBClient::new();
        App::new().data(dbc).service(get_thread)
        .service(get_post)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}