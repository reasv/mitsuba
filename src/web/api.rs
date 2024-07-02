#[allow(unused_imports)]
use log::{info, warn, error, debug};

use actix_web::{get, web, HttpResponse};
use actix_files::NamedFile;
use new_mime_guess::from_path;

use crate::db::DBClient;
use crate::object_storage::ObjectStorage;
use crate::util::{get_file_folder, get_file_url};
use crate::models::{IndexPage, BoardsStatus};

#[get("/boards-status.json")]
pub(crate) async fn get_boards_status(db: web::Data<DBClient>) -> actix_web::Result<HttpResponse> {
    let boards = db.get_all_boards().await
        .map_err(|e| {
            error!("Error getting boards from DB: {}", e);
            actix_web::error::ErrorInternalServerError("")
        })?;
    Ok(HttpResponse::Ok().json(BoardsStatus{boards}))
}

#[get("/{board:[A-z0-9]+}/thread/{no:\\d+}.json")]
pub(crate) async fn get_thread(db: web::Data<DBClient>, info: web::Path<(String, i64)>) -> actix_web::Result<HttpResponse> {
    let (board, no) = info.into_inner();
    let thread = db.get_thread(&board, no, true).await
        .map_err(|e| {
            error!("Error getting thread from DB: {}", e);
            actix_web::error::ErrorInternalServerError("")
        })?
        .ok_or(actix_web::error::ErrorNotFound(""))?;
    
    Ok(HttpResponse::Ok().json(thread))
}

#[get("/{board:[A-z0-9]+}/post/{no:\\d+}.json")]
pub(crate) async fn get_post(db: web::Data<DBClient>, info: web::Path<(String, i64)>) -> actix_web::Result<HttpResponse> {
    let (board, no) = info.into_inner();
    let post = db.get_post(&board, no, true).await
        .map_err(|e| {
            error!("Error getting post from DB: {}", e);
            actix_web::error::ErrorInternalServerError("")
        })?
        .ok_or(actix_web::error::ErrorNotFound(""))?;
    Ok(HttpResponse::Ok().json(post))
}
#[get("/{board:[A-z0-9]+}/{idx:\\d+}.json")]
pub(crate) async fn get_index(db: web::Data<DBClient>, info: web::Path<(String, i64)>) -> actix_web::Result<HttpResponse> {
    let (board, mut index) = info.into_inner();
    if index > 0 {
        index -= 1;
    }
    let threads = db.get_thread_index(&board, index, 15, true).await
        .map_err(|e| {
            error!("Error getting post from DB: {}", e);
            actix_web::error::ErrorInternalServerError("")
        })?;
    Ok(HttpResponse::Ok().json(IndexPage {threads: threads.into_iter().map(|t| t.into()).collect()}))
}

#[get("/{board:[A-z0-9]+}/{tim:\\d+}.{ext}")]
pub(crate) async fn get_full_image(db: web::Data<DBClient>, info: web::Path<(String, i64, String)>) -> actix_web::Result<NamedFile> {
    let (board, tim, ext) = info.into_inner();
    get_image_from_tim(db, board, tim, ext, false).await
}

#[get("/{board:[A-z0-9]+}/{tim:\\d+}s.jpg")]
pub(crate) async fn get_thumbnail_image(db: web::Data<DBClient>, info: web::Path<(String, i64)>) -> actix_web::Result<NamedFile> {
    let (board, tim) = info.into_inner();
    get_image_from_tim(db, board, tim, "".to_string(), true).await
}

pub(crate) async fn get_image_from_tim(db: web::Data<DBClient>, board: String, tim: i64, ext: String, is_thumb: bool)-> actix_web::Result<NamedFile> {
    let sha256 = db.image_tim_to_sha256(&board, tim, is_thumb, true).await
        .map_err(|e| {
            error!("Error getting image from DB: {}", e);
            actix_web::error::ErrorInternalServerError("")
        })?
        .ok_or(actix_web::error::ErrorNotFound(""))?;
    
    let filename = match is_thumb {
        true => format!("{}.jpg", sha256),
        false => format!("{}.{}", sha256, ext)
    };
    let path = get_file_folder(&sha256, is_thumb).join(filename);
    NamedFile::open(path).map_err(|e| {
        error!("Error getting image from filesystem: {}", e);
        actix_web::error::ErrorNotFound("")
    })
}

#[get("/{board:[A-z0-9]+}/{tim:\\d+}.{ext}")]
pub(crate) async fn get_full_image_object_storage(db: web::Data<DBClient>, obc: web::Data<ObjectStorage>, info: web::Path<(String, i64, String)>) -> actix_web::Result<HttpResponse> {
    let (board, tim, ext) = info.into_inner();
    get_image_from_tim_object_storage(db, obc, board, tim, ext, false).await
}
#[get("/{board:[A-z0-9]+}/{tim:\\d+}s.jpg")]
pub(crate) async fn get_thumbnail_image_object_storage(db: web::Data<DBClient>, obc: web::Data<ObjectStorage>, info: web::Path<(String, i64)>) -> actix_web::Result<HttpResponse> {
    let (board, tim) = info.into_inner();
    get_image_from_tim_object_storage(db, obc, board, tim, "jpg".to_string(), true).await
}

pub(crate) async fn get_image_from_tim_object_storage(db: web::Data<DBClient>, obc: web::Data<ObjectStorage>, board: String, tim: i64, ext: String, is_thumb: bool) -> actix_web::Result<HttpResponse> {
    let sha256 = db.image_tim_to_sha256(&board, tim, is_thumb, true).await
        .map_err(|e| {
            error!("Error getting image from DB: {}", e);
            actix_web::error::ErrorInternalServerError("")
        })?
        .ok_or(actix_web::error::ErrorNotFound(""))?;
    let path = get_file_url(&sha256, &(".".to_string()+&ext), is_thumb);

    get_file_object_storage(obc, &path).await
}

pub(crate) async fn get_file_object_storage_handler(obc: web::Data<ObjectStorage>, path: web::Path<String>) -> actix_web::Result<HttpResponse> {
    let path = &path.into_inner();
    get_file_object_storage(obc, &("/img/".to_string()+&path)).await
}

pub(crate) async fn get_file_object_storage(obc: web::Data<ObjectStorage>, path: &String) -> actix_web::Result<HttpResponse> {
    let response_data = obc.bucket.get_object(path).await.map_err(|e| {
        error!("Error getting file ({}) from bucket: {}", path, e);
        actix_web::error::ErrorInternalServerError("")
    })?;
    let code = response_data.status_code();
    let data = response_data.bytes().to_owned();
    if code == 404 {
        Err(actix_web::error::ErrorNotFound("404 Not Found (Object storage)"))
    } else if code == 200 {
        let region = obc.bucket.url();
        debug!("{}{}", region, path);
        Ok(HttpResponse::Ok().content_type(from_path(path).first_or_octet_stream().as_ref()).body(data))
    } else {
        error!("Error getting file ({}) from bucket: {}", path, code);
        Err(actix_web::error::ErrorInternalServerError(""))
    }
}
