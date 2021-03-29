#[allow(unused_imports)]
use log::{info, warn, error, debug};
use actix_web::{get, web, HttpResponse, Result};
use serde::{Deserialize, Serialize};

use handlebars::{Handlebars, RenderContext, Helper, Context, JsonRender, HelperResult, Output, RenderError};
use handlebars::handlebars_helper;
use handlebars_misc_helpers::register;

use crate::util::{shorten_string, string_to_idcolor,base64_to_32, get_image_url};
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
    let boards = db.get_all_boards().await
        .map_err(|e| {
            error!("Error getting boards from DB: {}", e);
            HttpResponse::InternalServerError().finish()
        })?;
    
    let thread = db.get_thread(&board, no).await
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

    let boards = db.get_all_boards().await
        .map_err(|e| {
            error!("Error getting boards from DB: {}", e);
            HttpResponse::InternalServerError().finish()
        })?;
    let threads = db.get_thread_index(&board, nonzero_index-1, 15).await
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

pub(crate) fn build_handlebars() -> Handlebars<'static> {
    let mut handlebars = Handlebars::new();
    handlebars
        .register_templates_directory(".html", "./src/templates")
        .unwrap();
    
    register(&mut handlebars);
    handlebars_helper!(b_to_kb: |b: i64|  b/1024i64);
    handlebars.register_helper("b_to_kb", Box::new(b_to_kb));

    handlebars.register_helper("shorten",
        Box::new(|h: &Helper, _r: &Handlebars, _: &Context, _rc: &mut RenderContext, out: &mut dyn Output| -> HelperResult {
            let length = h.param(0).ok_or(RenderError::new("Length not found"))?;
            let text = h.param(1).ok_or(RenderError::new("String not found"))?.value().render();
            let sz = length.value().as_u64().unwrap_or_default();
            out.write(shorten_string(sz as usize, text).as_ref())?;
            Ok(())
        }));
    handlebars.register_helper("id_color",
        Box::new(|h: &Helper, _r: &Handlebars, _: &Context, _rc: &mut RenderContext, out: &mut dyn Output| -> HelperResult {
            let id_text = h.param(0).ok_or(RenderError::new("ID not found"))?.value().render();
            out.write(string_to_idcolor(id_text).as_ref())?;
            Ok(())
        }));
    handlebars.register_helper("base64_to_32",
        Box::new(|h: &Helper, _r: &Handlebars, _: &Context, _rc: &mut RenderContext, out: &mut dyn Output| -> HelperResult {
            let b64_text = h.param(0).ok_or(RenderError::new("base64 not found"))?.value().render();
            out.write(base64_to_32(b64_text).unwrap_or_default().as_ref())?;
            Ok(())
        }));
    handlebars.register_helper("get_image_url",
    Box::new(|h: &Helper, _r: &Handlebars, _: &Context, _rc: &mut RenderContext, out: &mut dyn Output| -> HelperResult {
        let b64_text = h.param(0).ok_or(RenderError::new("base64 not found"))?.value().render();
        let is_thumb = h.param(1).ok_or(RenderError::new("Boolan not found"))?;
        let is_thumb_bool = is_thumb.value().as_bool().unwrap_or_default();
        out.write(get_image_url(&b64_text, is_thumb_bool).as_ref())?;
        Ok(())
    }));
    handlebars
}