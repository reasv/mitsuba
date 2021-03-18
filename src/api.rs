use std::path::Path;
#[allow(unused_imports)]
use log::{info, warn, error, debug};
use actix_web::{get, web, App, HttpResponse, HttpServer, Result};
use actix_files::NamedFile;
use handlebars::{Handlebars, RenderContext, Helper, Context, JsonRender, HelperResult, Output, RenderError};
use handlebars::handlebars_helper;

use crate::db::DBClient;
use crate::frontend::thread_page;
use crate::util::{shorten_string, string_to_idcolor,base64_to_32};

#[get("/{board}/thread/{no}.json")]
async fn get_thread(db: web::Data<DBClient>, info: web::Path<(String, i64)>) -> Result<HttpResponse, HttpResponse> {
    let (board, no) = info.into_inner();
    let thread = db.get_thread_async(&board, no).await
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
    let post = db.get_post_async(&board, no).await
        .map_err(|e| {
            error!("Error getting post from DB: {}", e);
            HttpResponse::InternalServerError().finish()
        })?
        .ok_or(HttpResponse::NotFound().finish())?;
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
    get_image(db, board, tim, "".to_string(), true).await
}

async fn get_image(db: web::Data<DBClient>, board: String, tim: i64, ext: String, is_thumb: bool)-> Result<NamedFile> {
    let md5_base64 = db.image_tim_to_md5_async(&board, tim).await
        .map_err(|e| {
            error!("Error getting image from DB: {}", e);
            HttpResponse::InternalServerError().finish()
        })?
        .ok_or(HttpResponse::NotFound().finish())?;

    let md5_base32 = base64_to_32(md5_base64).unwrap();
    let path = match is_thumb { 
        true => Path::new("data").join("images").join("thumb").join(format!("{}.jpg", md5_base32)),
        false => Path::new("data").join("images").join("full").join(format!("{}.{}", md5_base32, ext))
    };
    Ok(NamedFile::open(path)?)
}

// #[actix_web::main]
pub async fn web_main() -> std::io::Result<()> {
    let mut handlebars = Handlebars::new();
    handlebars
        .register_templates_directory(".html", "./src/templates")
        .unwrap();
    

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

    let handlebars_ref = web::Data::new(handlebars);
    
    HttpServer::new(move || {
        let dbc: DBClient = DBClient::new();
        App::new()
        .data(dbc)
        .app_data(handlebars_ref.clone())
        .service(get_thread)
        .service(get_post)
        .service(get_thumbnail_image)
        .service(get_full_image)
        .service(thread_page)
        .service(actix_files::Files::new("/img", "data/images"))
        .service(actix_files::Files::new("/static", "static"))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}