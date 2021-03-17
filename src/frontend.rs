#[allow(unused_imports)]
use log::{info, warn, error, debug};
use actix_web::{get, web, HttpResponse, Result};
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};

use crate::db::DBClient;
// use crate::board_archiver::base64_to_32;

#[derive(Debug, Clone, Deserialize, Serialize)]
struct TemplateThread {
    pub op: crate::models::Post,
    pub posts: Vec<crate::models::Post>
}

#[get("/{board}/thread/{no}")]
pub(crate) async fn thread_page(db: web::Data<DBClient>, hb: web::Data<Handlebars<'_>>, info: web::Path<(String, i64)>) 
-> Result<HttpResponse, HttpResponse> {
    let (board, no) = info.into_inner();
    let thread = db.get_thread_async(&board, no).await
        .map_err(|e| {
            error!("Error getting thread from DB: {}", e);
            HttpResponse::InternalServerError().finish()
        })?
        .ok_or(HttpResponse::NotFound().finish())?;
    
    let body = hb.render("thread", &TemplateThread{
        op: thread.posts[0].clone(),
        posts: thread.posts[1..].to_vec()
    }).unwrap();
    Ok(HttpResponse::Ok().body(body))
}