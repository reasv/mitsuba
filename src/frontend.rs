#[allow(unused_imports)]
use log::{info, warn, error, debug};
use actix_web::{get, web, HttpResponse, Result};
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};

use crate::db::DBClient;
use crate::models::{IndexThread, Post, IndexPost, Board};

#[derive(Debug, Clone, Deserialize, Serialize)]
struct TemplateThread {
    pub boards: Vec<Board>,
    pub op: Post,
    pub posts: Vec<Post>
}
#[derive(Debug, Clone, Deserialize, Serialize)]
struct TemplateThreadIndex {
    pub boards: Vec<Board>,
    pub next: i64,
    pub prev: i64,
    pub current: i64,
    pub op: Post,
    pub threads: Vec<TemplateThreadIndexThread>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
struct TemplateThreadIndexThread {
    pub op: IndexPost,
    pub posts: Vec<IndexPost>
}
#[get("/{board}/thread/{no}")]
pub(crate) async fn thread_page(db: web::Data<DBClient>, hb: web::Data<Handlebars<'_>>, info: web::Path<(String, i64)>) 
-> Result<HttpResponse, HttpResponse> {
    let (board, no) = info.into_inner();
    let boards = db.get_all_boards_async().await
        .map_err(|e| {
            error!("Error getting boards from DB: {}", e);
            HttpResponse::InternalServerError().finish()
        })?;
    
    let thread = db.get_thread_async(&board, no).await
        .map_err(|e| {
            error!("Error getting thread from DB: {}", e);
            HttpResponse::InternalServerError().finish()
        })?
        .ok_or(HttpResponse::NotFound().finish())?;
    
    let body = hb.render("thread", &TemplateThread{
        boards,
        op: thread.posts[0].clone(),
        posts: thread.posts[1..].to_vec()
    }).unwrap();
    Ok(HttpResponse::Ok().body(body))
}

#[get("/{board}/{idx}")]
pub(crate) async fn index_page(db: web::Data<DBClient>, hb: web::Data<Handlebars<'_>>, info: web::Path<(String, i64)>) 
-> Result<HttpResponse, HttpResponse> {
    let (board, index) = info.into_inner();
    let mut nonzero_index = 1;
    if index > 0 {
        nonzero_index = index;
    }

    let boards = db.get_all_boards_async().await
        .map_err(|e| {
            error!("Error getting boards from DB: {}", e);
            HttpResponse::InternalServerError().finish()
        })?;
    let threads = db.get_thread_index_async(&board, nonzero_index-1, 15).await
        .map_err(|e| {
            error!("Error getting post from DB: {}", e);
            HttpResponse::InternalServerError().finish()
        })?;
    if threads.len() == 0 {
        return Err(HttpResponse::NotFound().finish())
    }
    let index_threads: Vec<IndexThread> = threads.clone().into_iter().map(|t| t.into()).collect();
    let prev = match nonzero_index == 1 {
        true => nonzero_index,
        false=> nonzero_index-1
    };
    let body = hb.render("index_page", &TemplateThreadIndex {
        boards,
        next: nonzero_index+1,
        current: nonzero_index,
        prev,
        op: threads[0].posts[0].clone(),
        threads: index_threads.into_iter().map(|t| TemplateThreadIndexThread{op: t.posts[0].clone(), posts: t.posts[1..].to_vec()}).collect()
    }).unwrap();
    Ok(HttpResponse::Ok().body(body))
}