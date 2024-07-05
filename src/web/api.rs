use actix_session::Session;
#[allow(unused_imports)]
use log::{info, warn, error, debug};

use actix_web::{get, put, post, delete, web, HttpResponse};
use actix_files::NamedFile;
use new_mime_guess::from_path;
use serde::{Deserialize, Serialize};

use crate::archiver::Archiver;
use crate::db::DBClient;
use crate::object_storage::ObjectStorage;
use crate::util::{get_file_folder, get_file_url};
use crate::models::{Board, BoardsStatus, IndexPage, IndexSearchResults, UserRole};
use crate::web::auth::{should_respect_hidden_files, AuthUser, Authenticated, AdminOnly};

use super::auth::RequireJanitor;

#[derive(Deserialize)]
struct LoginBody {
    username: String,
    password: String,
}

#[derive(Serialize)]
struct LoginResult {
    success: bool,
    user: String,
    role: String,
    message: String,
}
#[derive(Serialize)]
struct APIResult {
    success: bool,
    message: String,
}

#[put("/_mitsuba/login.json")]
pub(crate) async fn login_api(archiver: web::Data<Archiver>, query: web::Json<LoginBody>, session: Session) -> actix_web::Result<HttpResponse> {
    // Extract the username and password from the query
    let query = query.into_inner();
    let username = query.username;
    let password = query.password;

    if let Some(user) = archiver.login(&username, &password).await
    .map_err(|e| {
        error!("Error getting user from DB: {}", e);
        actix_web::error::ErrorInternalServerError("Error getting user from DB")
    })? {
        session.insert("username", user.name.clone())?;
        return Ok(HttpResponse::Ok().json(LoginResult{
            success: true,
            user: user.name,
            role: user.role.to_string(),
            message: "Logged in".to_string()
        }));
    }
    Ok(HttpResponse::Unauthorized().json(APIResult{
        success: false,
        message: "Wrong username or password".to_string()
    }))
}

#[put("/_mitsuba/logout.json")]
pub(crate) async fn logout_api(session: Session) -> actix_web::Result<HttpResponse> {
        session.remove("username");
        session.purge();
        Ok(HttpResponse::Ok().json(APIResult{
            success: true,
            message: "Logged out".to_string()
        })
    )
}

#[get("/_mitsuba/authcheck.json")]
pub(crate) async fn authcheck_api(user: AuthUser<Authenticated>) -> actix_web::Result<HttpResponse> {
        Ok(HttpResponse::Ok().json(LoginResult{
            success: true,
            user: user.name.clone(),
            role: user.role.clone().to_string(),
            message: format!("Logged in as {} (Role: {})", user.name, user.role)
        })
    )
}

#[get("/_mitsuba/admin/users.json")]
pub(crate) async fn get_users(db: web::Data<DBClient>, _: AuthUser<AdminOnly>) -> actix_web::Result<HttpResponse> {
    let users = db.get_users().await
        .map_err(|e| {
            error!("Error getting users from DB: {}", e);
            actix_web::error::ErrorInternalServerError("Error getting users from DB")
        })?;
    // Remove the password hash from the response
    let users_: Vec<AuthUser> = users.into_iter().map(|u| u.into()).collect();
    Ok(HttpResponse::Ok().json(users_))
}

#[derive(Serialize, Deserialize)]
struct NewUser {
    username: String,
    password_hash: String,
    role: UserRole,
}
#[post("/_mitsuba/admin/users.json")]
pub(crate) async fn post_user(
    archiver: web::Data<Archiver>,
    new_user: web::Json<NewUser>,
    _: AuthUser<AdminOnly>
)
-> actix_web::Result<HttpResponse> {
    let new_user = new_user.into_inner();
    let user = archiver.add_user(
        &new_user.username,
        &new_user.password_hash,
        new_user.role
    ).await
        .map_err(|e| {
            error!("Error creating user in DB: {}", e);
            actix_web::error::ErrorInternalServerError("Error creating user in DB")
    })?;
    Ok(HttpResponse::Ok().json(user))
}

#[derive(Serialize)]
struct JsonResult<T> {
    success: bool,
    data: Option<T>,
    message: String,
}

#[derive(Serialize, Deserialize)]
struct UserEdits {
    role: Option<UserRole>,
    password_hash: Option<String>,
}
#[put("/_mitsuba/admin/users/{username}.json")]
pub(crate) async fn put_user(
    archiver: web::Data<Archiver>,
    user_edits: web::Json<UserEdits>,
    username: web::Path<String>,
    _: AuthUser<AdminOnly>
) -> actix_web::Result<HttpResponse> {
    let user_edits = user_edits.into_inner();
    let username = username.into_inner();
    
    if let Some(role) = user_edits.role {
        archiver.change_role(&username, role).await
        .map_err(|e| {
            error!("Error setting user role in DB: {}", e);
            actix_web::error::ErrorInternalServerError("Error setting user role in DB")
        })?;
    }

    if let Some(password_hash) = user_edits.password_hash {
        archiver.change_password(&username, &password_hash).await
        .map_err(|e| {
            error!("Error setting user password in DB: {}", e);
            actix_web::error::ErrorInternalServerError("Error setting user password in DB")
        })?;
    }

    Ok(HttpResponse::Ok().json(JsonResult::<()>{
        success: true,
        data: None,
        message: "User edited".to_string()
    }))
}

#[delete("/_mitsuba/admin/users/{username}.json")]
pub(crate) async fn delete_user(
    archiver: web::Data<Archiver>,
    username: web::Path<String>,
    _: AuthUser<AdminOnly>
) -> actix_web::Result<HttpResponse> {
    let username = username.into_inner();
    archiver.delete_user(&username).await
    .map_err(|e| {
        error!("Error deleting user from DB: {}", e);
        actix_web::error::ErrorInternalServerError("Error deleting user from DB")
    })?;
    Ok(HttpResponse::Ok().json(JsonResult::<()>{
        success: true,
        data: None,
        message: "User deleted".to_string()
    }))
}

#[derive(Serialize, Deserialize)]
struct UserSelfEdits {
    current_passsword: String,
    password_hash: Option<String>,
}
#[put("/_mitsuba/admin/user.json")]
pub(crate) async fn put_current_user(
    archiver: web::Data<Archiver>,
    user_edits: web::Json<UserSelfEdits>,
    user: AuthUser<Authenticated>
) -> actix_web::Result<HttpResponse> {
    let user_edits = user_edits.into_inner();

    // Attempt to login with the current password
    if archiver.login(
        &user.name,
        &user_edits.current_passsword
    ).await
    .map_err(|e| {
            error!("Error getting user from DB: {}", e);
            actix_web::error::ErrorInternalServerError("Error getting user from DB")
        }
    )?.is_none() {
        return Ok(HttpResponse::Ok().json(JsonResult::<()>{
            success: false,
            data: None,
            message: "Wrong password".to_string()
        }));
    }

    if let Some(password_hash) = user_edits.password_hash {
        archiver.change_password(&user.name, &password_hash).await
        .map_err(|e| {
            error!("Error setting user password in DB: {}", e);
            actix_web::error::ErrorInternalServerError("Error setting user password in DB")
        })?;
    }

    Ok(HttpResponse::Ok().json(JsonResult::<()>{
        success: true,
        data: None,
        message: "User edited".to_string()
    }))
}


#[get("/_mitsuba/admin/boards-status.json")]
pub(crate) async fn get_boards_status(db: web::Data<DBClient>, _: AuthUser<Authenticated>) -> actix_web::Result<HttpResponse> {
    let boards = db.get_all_boards().await
        .map_err(|e| {
            error!("Error getting boards from DB: {}", e);
            actix_web::error::ErrorInternalServerError("")
        })?;
    Ok(HttpResponse::Ok().json(BoardsStatus{boards}))
}

#[derive(Serialize, Deserialize)]
struct BoardSettings {
    pub full_images: Option<bool>,
    pub archive: Option<bool>,
    pub enable_search: Option<bool>,
}
#[put("/{board:[A-z0-9]+}/board.json")]
pub(crate) async fn put_board(
    archiver: web::Data<Archiver>,
    info: web::Path<String>,
    settings: web::Json<BoardSettings>,
    _: AuthUser<AdminOnly>
) -> actix_web::Result<HttpResponse> {
    let board_name = info.into_inner();
    let settings = settings.into_inner();

    let mut board: Board = archiver.db_client.get_board(&board_name).await
    .map_err(|e| {
        error!("Error getting board from DB: {}", e);
        actix_web::error::ErrorInternalServerError("Error getting board from DB")
    })?
    .unwrap_or_default();

    board.full_images = settings.full_images.unwrap_or(board.full_images);
    board.archive = settings.archive.unwrap_or(board.archive);
    board.enable_search = settings.enable_search.unwrap_or(board.enable_search);

    archiver.set_board(board.clone()).await
        .map_err(|e| {
            error!("Error setting board settings in DB: {}", e);
            actix_web::error::ErrorInternalServerError("Error setting board settings in DB")
        })?;
    Ok(HttpResponse::Ok().json(JsonResult::<Board>{
        success: true,
        data: Some(board),
        message: "Board settings edited".to_string()
    }))
}

#[derive(Serialize, Deserialize)]
struct BoardDeleteOptions {
    pub only_delete_files: Option<bool>,
}
#[delete("/{board:[A-z0-9]+}/board.json")]
pub(crate) async fn delete_board(
    archiver: web::Data<Archiver>,
    info: web::Path<String>,
    options: web::Json<BoardDeleteOptions>,
    _: AuthUser<AdminOnly>
) -> actix_web::Result<HttpResponse> {
    let board_name = info.into_inner();
    let options = options.into_inner();

    archiver.purge_board(&board_name, options.only_delete_files.unwrap_or(false)).await
        .map_err(|e| {
            error!("Error deleting board from DB: {}", e);
            actix_web::error::ErrorInternalServerError("Error deleting board from DB")
        })?;
    Ok(HttpResponse::Ok().json(JsonResult::<()>{
        success: true,
        data: None,
        message: "Board deleted".to_string()
    }))
}

#[get("/{board:[A-z0-9]+}/thread/{no:\\d+}.json")]
pub(crate) async fn get_thread(
    db: web::Data<DBClient>,
    info: web::Path<(String, i64)>,
    user: AuthUser,
) -> actix_web::Result<HttpResponse> {
    let (board, no) = info.into_inner();

    let respect_hidden_files = should_respect_hidden_files(user);
    let thread = db
        .get_thread(
            &board,
            no,
            respect_hidden_files
        ).await
        .map_err(|e| {
            error!("Error getting thread from DB: {}", e);
            actix_web::error::ErrorInternalServerError("")
        })?
        .ok_or(actix_web::error::ErrorNotFound(""))?;
    
    Ok(HttpResponse::Ok().json(thread))
}

#[get("/{board:[A-z0-9]+}/post/{no:\\d+}.json")]
pub(crate) async fn get_post(
    db: web::Data<DBClient>,
    info: web::Path<(String, i64)>,
    user: AuthUser,
) -> actix_web::Result<HttpResponse> {
    let (board, no) = info.into_inner();
    let respect_hidden_files = should_respect_hidden_files(user);
    let post = db.get_post(&board, no, respect_hidden_files).await
        .map_err(|e| {
            error!("Error getting post from DB: {}", e);
            actix_web::error::ErrorInternalServerError("")
        })?
        .ok_or(actix_web::error::ErrorNotFound(""))?;
    Ok(HttpResponse::Ok().json(post))
}

#[derive(Serialize, Deserialize)]
struct PostEdits{
    mitsuba_post_hidden: Option<bool>,
    mitsuba_file_hidden: Option<bool>,
    mitsuba_com_hidden: Option<bool>,
    // Purges the image from the filesystem if this is set
    ban_image_reason: Option<String>,
    unban_image: Option<bool>,
}
#[put("/{board:[A-z0-9]+}/post/{no:\\d+}.json")]
pub(crate) async fn put_post(
    db: web::Data<DBClient>,
    post_edits: web::Json<PostEdits>,
    archiver: web::Data<Archiver>,
    info: web::Path<(String, i64)>,
    user: AuthUser<RequireJanitor>,
) -> actix_web::Result<HttpResponse> {
    let (board, no) = info.into_inner();
    
    let post_edits = post_edits.into_inner();

    archiver.hide_post(
        &board,
        no,
        post_edits.mitsuba_file_hidden,
        post_edits.mitsuba_com_hidden,
        post_edits.mitsuba_file_hidden
    ).await.map_err(|e| {
        error!("Error hiding post: {}", e);
        actix_web::error::ErrorInternalServerError("Error hiding post")
    })?;

    if let Some(reason) = post_edits.ban_image_reason {
        // Only mods and above can purge images
        if user.role > UserRole::Janitor {
            archiver
            .ban_image(
                &board,
                no,
                &reason
            ).await.map_err(|e| {
                error!("Error purging image: {}", e);
                actix_web::error::ErrorInternalServerError("Error banning image")
            })?;
        }
    }

    if let Some(true) = post_edits.unban_image {
        // Only mods and above can unban images
        if user.role > UserRole::Janitor {
            archiver
            .unban_image(
                &board,
                no
            ).await.map_err(|e| {
                error!("Error unpurging image: {}", e);
                actix_web::error::ErrorInternalServerError("Error unbanning image")
            })?;
        }
    }

    let post = db.get_post(
        &board,
        no,
        false
    ).await
        .map_err(|e| {
            error!("Error getting post from DB: {}", e);
            actix_web::error::ErrorInternalServerError("")
        })?
        .ok_or(actix_web::error::ErrorNotFound(""))?;
    Ok(HttpResponse::Ok().json(post))
}

#[derive(Deserialize)]
struct SearchQuery {
    s: Option<String>,
}

#[get("/{board:[A-z0-9]+}/{idx:\\d+}.json")]
pub(crate) async fn get_index(
    db: web::Data<DBClient>,
    info: web::Path<(String, i64)>,
    query: web::Query<SearchQuery>,
    user: AuthUser,
) -> actix_web::Result<HttpResponse> {
    let (board, mut index) = info.into_inner();
    if index > 0 {
        index -= 1;
    }
    let respect_hidden_files = should_respect_hidden_files(user);
    if let Some(search_query) = &query.s {
        let (posts, total_results) = db
            .posts_full_text_search(
                &board,
                &search_query,
                index,
                15,
                respect_hidden_files
            ).await
            .map_err(|e| {
                error!("Error getting post from DB: {}", e);
                actix_web::error::ErrorInternalServerError("")
            })?;
        return Ok(HttpResponse::Ok().json(IndexSearchResults {posts, total_results}));
    }

    let threads = db.get_thread_index(
        &board,
        index,
        15,
        respect_hidden_files
    ).await
        .map_err(|e| {
            error!("Error getting post from DB: {}", e);
            actix_web::error::ErrorInternalServerError("")
        })?;
    Ok(HttpResponse::Ok().json(IndexPage {threads: threads.into_iter().map(|t| t.into()).collect()}))
}

#[get("/{board:[A-z0-9]+}/{tim:\\d+}.{ext}")]
pub(crate) async fn get_full_image(
    db: web::Data<DBClient>,
    info: web::Path<(String, i64, String)>,
    user: AuthUser,
) -> actix_web::Result<NamedFile> {
    let (board, tim, ext) = info.into_inner();
    let respect_hidden_files = should_respect_hidden_files(user);
    get_image_from_tim(db, board, tim, ext, false, respect_hidden_files).await
}

#[get("/{board:[A-z0-9]+}/{tim:\\d+}s.jpg")]
pub(crate) async fn get_thumbnail_image(
    db: web::Data<DBClient>,
    info: web::Path<(String, i64)>,
    user: AuthUser,
) -> actix_web::Result<NamedFile> {
    let (board, tim) = info.into_inner();
    let respect_hidden_files = should_respect_hidden_files(user);
    get_image_from_tim(db, board, tim, "".to_string(), true, respect_hidden_files).await
}

pub(crate) async fn get_image_from_tim(db: web::Data<DBClient>, board: String, tim: i64, ext: String, is_thumb: bool, remove_hidden: bool)-> actix_web::Result<NamedFile> {
    let sha256 = db.image_tim_to_sha256(&board, tim, is_thumb, remove_hidden).await
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
pub(crate) async fn get_full_image_object_storage(
    db: web::Data<DBClient>,
    obc: web::Data<ObjectStorage>,
    info: web::Path<(String, i64, String)>,
    user: AuthUser,
) -> actix_web::Result<HttpResponse> {
    let (board, tim, ext) = info.into_inner();
    let respect_hidden_files = should_respect_hidden_files(user);
    get_image_from_tim_object_storage(db, obc, board, tim, ext, false, respect_hidden_files).await
}
#[get("/{board:[A-z0-9]+}/{tim:\\d+}s.jpg")]
pub(crate) async fn get_thumbnail_image_object_storage(
    db: web::Data<DBClient>,
    obc: web::Data<ObjectStorage>,
    info: web::Path<(String, i64)>,
    user: AuthUser,
) -> actix_web::Result<HttpResponse> {
    let (board, tim) = info.into_inner();
    let respect_hidden_files = should_respect_hidden_files(user);
    get_image_from_tim_object_storage(db, obc, board, tim, "jpg".to_string(), true, respect_hidden_files).await
}

pub(crate) async fn get_image_from_tim_object_storage(
    db: web::Data<DBClient>,
    obc: web::Data<ObjectStorage>,
    board: String,
    tim: i64,
    ext: String,
    is_thumb: bool,
    remove_hidden: bool
) -> actix_web::Result<HttpResponse> {
    let sha256 = db.image_tim_to_sha256(&board, tim, is_thumb, remove_hidden).await
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