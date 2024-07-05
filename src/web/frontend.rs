use std::cmp::min;
use std::{env, vec};
use std::collections::HashSet;
use std::convert::AsRef;

#[allow(unused_imports)]
use log::{info, warn, error, debug};
use actix_web::{get, web, HttpResponse};
use serde::{Deserialize, Serialize};
use rust_embed::RustEmbed;
use new_mime_guess::from_path;

use handlebars::{Handlebars, RenderContext, Helper, Context, JsonRender, HelperResult, Output, RenderErrorReason};
use handlebars::handlebars_helper;
use handlebars_misc_helpers::register;

use crate::util::{shorten_string, string_to_idcolor,base64_to_32, get_file_url};
use crate::db::DBClient;
use crate::models::{IndexThread, Post, IndexPost, Board, Thread};
use crate::web::auth::{AuthUser, should_respect_hidden_files};

#[derive(Debug, Clone, Deserialize, Serialize)]
struct TemplateThread {
    pub boards: Vec<Board>,
    pub op: Post,
    pub board: String,
    pub posts: Vec<Post>
}
#[derive(Debug, Clone, Deserialize, Serialize)]
struct TemplateThreadIndex {
    pub boards: Vec<Board>,
    pub next: i64,
    pub prev: i64,
    pub current: i64,
    pub board: String,
    pub op: Option<Post>,
    pub pages: Vec<i64>,
    pub threads: Vec<TemplateThreadIndexThread>,
    pub query_string: String
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct TemplateLogin {
    pub boards: Vec<Board>,
    pub board: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
struct TemplateThreadIndexThread {
    pub op: IndexPost,
    pub posts: Vec<IndexPost>
}
#[derive(Debug, Clone, Deserialize, Serialize)]
struct TemplateHomePage {
    pub boards: Vec<Board>,
    pub posts: Vec<Post>
}

fn get_home_boards(boards: &Vec<Board>) -> Vec<String> {
    let env_str = env::var("HOME_PAGE_BOARDS").ok().unwrap_or_default();
    let board_set = env_str.split(",").fold(HashSet::new(), |mut set, s| {if !s.is_empty() {set.insert(s.to_string());} set});
    // in case there's no boards in the set, we use all boards.
    boards.iter().map(|b| b.name.clone()).filter(|b| board_set.contains(b) || board_set.is_empty()).collect()
}

#[get("/")]
pub(crate) async fn home_page(db: web::Data<DBClient>, hb: web::Data<Handlebars<'_>>) 
-> actix_web::Result<HttpResponse> {
    let boards = db.get_all_boards().await
        .map_err(|e| {
            error!("Error getting boards from DB: {}", e);
            actix_web::error::ErrorInternalServerError("")
        })?;
    let home_boards = get_home_boards(&boards);
    let posts = db.get_latest_images(500i64, 0i64, home_boards).await
    .map_err(|e| {
        error!("Error getting posts from DB: {}", e);
        actix_web::error::ErrorInternalServerError("")
    })?;

    let body = hb.render("home", &TemplateHomePage{
        boards,
        posts
    }).unwrap();
    Ok(HttpResponse::Ok().body(body))
}

#[get("/{board:[A-z0-9]+}/thread/{no:\\d+}{foo:/?[^/]*}")]
pub(crate) async fn thread_page(
    db: web::Data<DBClient>,
    hb: web::Data<Handlebars<'_>>,
    info: web::Path<(String, i64)>,
    user: AuthUser
) 
-> actix_web::Result<HttpResponse> {
    let remove_hidden_files = should_respect_hidden_files(user);

    let (board, no) = info.into_inner();
    let boards = db.get_all_boards().await
        .map_err(|e| {
            error!("Error getting boards from DB: {}", e);
            actix_web::error::ErrorInternalServerError("")
        })?;
    
    let thread = db.get_thread(&board, no, remove_hidden_files).await
        .map_err(|e| {
            error!("Error getting thread from DB: {}", e);
            actix_web::error::ErrorInternalServerError("")
        })?
        .ok_or(actix_web::error::ErrorNotFound(""))?;
    
    let body = hb.render("thread", &TemplateThread{
        boards,
        op: thread.posts[0].clone(),
        board: board.clone(),
        posts: thread.posts[1..].to_vec()
    }).unwrap();
    Ok(HttpResponse::Ok().body(body))
}

#[derive(Deserialize)]
struct SearchQuery {
    s: Option<String>,
}

#[get("/{board:[A-z0-9]+}/{idx:\\d+}")]
pub(crate) async fn index_page_handler(
    db: web::Data<DBClient>,
    hb: web::Data<Handlebars<'_>>,
    info: web::Path<(String, i64)>,
    query: web::Query<SearchQuery>,
    user: AuthUser
) 
-> actix_web::Result<HttpResponse> {
    let remove_hidden_files = should_respect_hidden_files(user);
    let (board, index) = info.into_inner();
    // Extract the s query parameter
    if let Some(search_query) = query.s.clone() {
        index_search_page(db, hb, board, index, &search_query, remove_hidden_files).await
    } else {
        index_page(db, hb, board, index, remove_hidden_files).await
    }
}

#[get("/{board:[A-z0-9]+}")]
pub(crate) async fn board_page(
    db: web::Data<DBClient>,
    hb: web::Data<Handlebars<'_>>,
    info: web::Path<String>,
    user: AuthUser
)
-> actix_web::Result<HttpResponse> {
    let remove_hidden_files = should_respect_hidden_files(user);
    let board = info.into_inner();
    index_page(db, hb, board, 1, remove_hidden_files).await
}

async fn index_page(db: web::Data<DBClient>, hb: web::Data<Handlebars<'_>>, board: String, index: i64, remove_hidden_files: bool) 
-> actix_web::Result<HttpResponse> {
    let mut nonzero_index = 1;
    if index > 0 {
        nonzero_index = index;
    }

    let prev = match nonzero_index == 1 {
        true  => nonzero_index,
        false => nonzero_index-1
    };

    let boards = db.get_all_boards().await
        .map_err(|e| {
            error!("Error getting boards from DB: {}", e);
            actix_web::error::ErrorInternalServerError("")
        })?;
    let threads = db
        .get_thread_index(
            &board,
            nonzero_index-1,
            15,
            remove_hidden_files
        ).await
        .map_err(|e| {
            error!("Error getting post from DB: {}", e);
            actix_web::error::ErrorInternalServerError("")
        })?;
    
    let index_threads: Vec<IndexThread> = threads.clone().into_iter().map(|t| t.into()).collect();
    
    let op: Option<Post> = match threads.len() > 0 {
        true  => Some(threads[0].posts[0].clone()),
        false => None
    };
    let body = hb.render("index_page", &TemplateThreadIndex {
        boards,
        next: nonzero_index+1,
        current: nonzero_index,
        pages: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
        prev,
        board: board.clone(),
        op,
        threads: index_threads.into_iter()
            .map(|t|
                TemplateThreadIndexThread{
                    op: t.posts[0].clone(), posts: t.posts[1..].to_vec()
                })
            .collect(),
        query_string: "".to_string()
    }).unwrap();
    Ok(HttpResponse::Ok().body(body))
}

async fn index_search_page(
    db: web::Data<DBClient>,
    hb: web::Data<Handlebars<'_>>,
    board: String,
    index: i64,
    search_query: &String,
    remove_hidden_files: bool
) 
-> actix_web::Result<HttpResponse> {
    let mut nonzero_index = 1;
    if index > 0 {
        nonzero_index = index;
    }
    let prev = match nonzero_index == 1 {
        true => nonzero_index,
        false=> nonzero_index-1
    };

    let boards = db.get_all_boards().await
        .map_err(|e| {
            error!("Error getting boards from DB: {}", e);
            actix_web::error::ErrorInternalServerError("")
        })?;
    
    let current_board = boards.iter().find(|b| b.name == board).ok_or(actix_web::error::ErrorNotFound(""))?;

    if !current_board.enable_search {
        return Err(actix_web::error::ErrorNotFound("Search disabled for this board"))
    }

    let page_size = 15;

    let (posts, total_results) = db
        .posts_full_text_search(
            &board,
            search_query,
            nonzero_index-1,
            page_size,
            remove_hidden_files
        ).await
        .map_err(|e| {
            error!("Error getting post from DB: {}", e);
            actix_web::error::ErrorInternalServerError("")
        })?;

    let op: Option<Post> = match posts.len() > 0 {
        true  => Some(posts[0].clone()),
        false => None
    };

    let index_threads: Vec<IndexThread> = posts.into_iter()
        .map(|p| Thread{posts: vec![p]})
        .map(|t| t.into()).collect();

    // Calculate the number of pages, capped at 100
    let num_pages = min((total_results as f64 / page_size as f64).ceil() as i64, 100);
    // Generate `num_pages` page numbers
    let pages: Vec<i64> = (1..=num_pages).collect();

    let body = hb.render("index_page", &TemplateThreadIndex {
        boards,
        next: nonzero_index+1,
        current: nonzero_index,
        prev,
        pages,
        op,
        board: board.clone(),
        threads: index_threads.into_iter()
            .map(|t|
                TemplateThreadIndexThread{
                    op: t.posts[0].clone(), posts: t.posts[1..].to_vec()
            }).collect(),
        query_string: format!("?s={}", search_query.clone())
    }).unwrap();
    Ok(HttpResponse::Ok().body(body))
}

#[get("/_mitsuba/login")]
pub(crate) async fn login_page(db: web::Data<DBClient>, hb: web::Data<Handlebars<'_>>) 
-> actix_web::Result<HttpResponse> {
    let boards = db.get_all_boards().await
        .map_err(|e| {
            error!("Error getting boards from DB: {}", e);
            actix_web::error::ErrorInternalServerError("")
        })?;

    let body = hb.render("login_page", &TemplateLogin{
        boards,
        board: "a".to_string()
    }).unwrap();
    Ok(HttpResponse::Ok().body(body))
}

#[derive(RustEmbed)]
#[folder = "src/templates"]
struct Templates;

pub(crate) fn build_handlebars() -> Handlebars<'static> {
    let mut handlebars = Handlebars::new();
    for template_path in Templates::iter() {
        if let Some(template_file) = Templates::get(&template_path) {
            let path_vec: Vec<&str> = template_path.split(".").collect();
            let name = path_vec[0];
            //let template_str: String = std::str::from_utf8(template_file.as_ref()).unwrap().to_string();
            let template_str: String = std::str::from_utf8(template_file.data.as_ref()).unwrap().to_string();
            handlebars.register_template_string(&name, &template_str).unwrap();
        }
    }
    
    register(&mut handlebars);
    handlebars_helper!(b_to_kb: |b: i64|  b/1024i64);
    handlebars.register_helper("b_to_kb", Box::new(b_to_kb));

    handlebars.register_helper("shorten",
        Box::new(|h: &Helper, _r: &Handlebars, _: &Context, _rc: &mut RenderContext, out: &mut dyn Output| -> HelperResult {
            let length = h.param(0).ok_or(RenderErrorReason::Other("Length not found".to_string()))?;
            let text = h.param(1).ok_or(RenderErrorReason::Other("String not found".to_string()))?.value().render();
            let sz = length.value().as_u64().unwrap_or_default();
            out.write(shorten_string(sz as usize, text).as_ref())?;
            Ok(())
        }));
    handlebars.register_helper("id_color",
        Box::new(|h: &Helper, _r: &Handlebars, _: &Context, _rc: &mut RenderContext, out: &mut dyn Output| -> HelperResult {
            let id_text = h.param(0).ok_or(RenderErrorReason::Other("ID not found".to_string()))?.value().render();
            out.write(string_to_idcolor(id_text).as_ref())?;
            Ok(())
        }));
    handlebars.register_helper("base64_to_32",
        Box::new(|h: &Helper, _r: &Handlebars, _: &Context, _rc: &mut RenderContext, out: &mut dyn Output| -> HelperResult {
            let b64_text = h.param(0).ok_or(RenderErrorReason::Other("base64 not found".to_string()))?.value().render();
            out.write(base64_to_32(b64_text).unwrap_or_default().as_ref())?;
            Ok(())
        }));
    handlebars.register_helper("get_file_url",
    Box::new(|h: &Helper, _r: &Handlebars, _: &Context, _rc: &mut RenderContext, out: &mut dyn Output| -> HelperResult {
        let sha256 = h.param(0).ok_or(RenderErrorReason::Other("sha256 not found".to_string()))?.value().render();
        let ext = h.param(1).ok_or(RenderErrorReason::Other("ext not found".to_string()))?.value().render();
        out.write(get_file_url(&sha256, &ext, false).as_ref())?;
        Ok(())
    }));
    handlebars.register_helper("get_thumbnail_url",
    Box::new(|h: &Helper, _r: &Handlebars, _: &Context, _rc: &mut RenderContext, out: &mut dyn Output| -> HelperResult {
        let sha256 = h.param(0).ok_or(RenderErrorReason::Other("sha256 not found".to_string()))?.value().render();
        out.write(get_file_url(&sha256, &".jpg".to_string(), true).as_ref())?;
        Ok(())
    }));
    handlebars
}

#[derive(RustEmbed)]
#[folder = "static"]
struct Asset;

fn handle_embedded_file(path: &str) -> HttpResponse {
    match Asset::get(path) {
      Some(content) => HttpResponse::Ok()
        .content_type(from_path(path).first_or_octet_stream().as_ref())
        .body(content.data.into_owned()),
      None => HttpResponse::NotFound().body("404 Not Found"),
    }
  }
  
pub(crate) async fn dist(path: web::Path<String>) -> HttpResponse {
    handle_embedded_file(&path.into_inner())
}