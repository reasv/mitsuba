use serde::{Deserialize, Serialize};
use std::string::String;
use std::str::FromStr;
use sqlx::Type;
use sqlx::decode::Decode;
use sqlx::encode::{Encode, IsNull};
use sqlx::error::BoxDynError;
use sqlx::postgres::{PgTypeInfo, PgValueRef, Postgres, PgArgumentBuffer};

#[derive(Debug, Clone, Deserialize, Serialize, Default, Eq, PartialEq, Hash)]
pub struct Post {
    #[serde(skip)]
    pub post_id: i64,
    #[serde(default)]
    pub board: String,
    pub no: i64,
    pub resto: i64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub sticky: i64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub closed: i64,
    pub now: String,
    pub time: i64,
    #[serde(default)]
    pub name: String,
    #[serde(default, skip_serializing_if = "is_empty_string")]
    pub trip: String,
    #[serde(default, skip_serializing_if = "is_empty_string")]
    pub id: String,
    #[serde(default, skip_serializing_if = "is_empty_string")]
    pub capcode: String,
    #[serde(default, skip_serializing_if = "is_empty_string")]
    pub country: String,
    #[serde(default, skip_serializing_if = "is_empty_string")]
    pub country_name: String,
    #[serde(default, skip_serializing_if = "is_empty_string")]
    pub sub: String,
    #[serde(default, skip_serializing_if = "is_empty_string")]
    pub com: String,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub tim: i64,
    #[serde(default, skip_serializing_if = "is_empty_string")]
    pub filename: String,
    #[serde(default, skip_serializing_if = "is_empty_string")]
    pub ext: String,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub fsize: i64,
    #[serde(default, skip_serializing_if = "is_empty_string")]
    pub md5: String,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub w: i64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub h: i64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub tn_w: i64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub tn_h: i64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub filedeleted: i64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub spoiler: i64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub custom_spoiler: i64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub replies: i64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub images: i64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub bumplimit: i64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub imagelimit: i64,
    #[serde(default, skip_serializing_if = "is_empty_string")]
    pub tag: String,
    #[serde(default, skip_serializing_if = "is_empty_string")]
    pub semantic_url: String,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub since4pass: i64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub unique_ips: i64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub m_img: i64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub archived: i64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub archived_on: i64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub last_modified: i64,
    #[serde(default, skip_serializing_if = "is_empty_string_or_none")]
    pub file_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "is_empty_string_or_none")]
    pub thumbnail_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub deleted_on: i64,
    #[serde(default, skip_serializing_if = "is_false")]
    pub mitsuba_post_hidden: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub mitsuba_com_hidden: bool,
    #[serde(default, skip_serializing_if = "is_false_or_none")]
    pub mitsuba_file_hidden: Option<bool>,
    #[serde(default, skip_serializing_if = "is_false_or_none")]
    pub mitsuba_file_blacklisted: Option<bool>
}

fn is_empty_string(s: &String) -> bool {
    s.is_empty()
}

fn is_empty_string_or_none(s: &Option<String>) -> bool {
    match s {
        Some(s) => s.is_empty(),
        None => true
    }
}

fn is_zero(i: &i64) -> bool {
    *i == 0
}

fn is_false_or_none(b: &Option<bool>) -> bool {
    match b {
        Some(b) => !*b,
        None => true
    }
}

fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PostUpdate {
    pub closed: i64,
    pub sticky: i64,
    pub filedeleted: i64,
    pub replies: i64,
    pub images: i64,
    pub bumplimit: i64,
    pub imagelimit: i64,
    pub unique_ips: i64,
    pub archived: i64,
    pub archived_on: i64,
    pub last_modified: i64
}
impl From<&Post> for PostUpdate {
    fn from(post: &Post) -> Self {
        // let unique_ips = match post.unique_ips > 0 {
        //     true => Some(post.unique_ips),
        //     false => None // Do not take update if update is 0
        // };
        Self {
            closed: post.closed,
            sticky: post.sticky,
            filedeleted: post.filedeleted,
            replies: post.replies,
            images: post.images,
            bumplimit: post.bumplimit,
            imagelimit: post.imagelimit,
            unique_ips: post.unique_ips,
            archived: post.archived,
            archived_on: post.archived_on,
            last_modified: post.last_modified
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default, Serialize, Eq, PartialEq)]
pub struct Thread {
    pub posts: Vec<Post>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThreadsPage {
    pub page: i64,
    pub threads: Vec<ThreadInfo>
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, Hash)]
pub struct ThreadInfo {
    #[serde(default)]
    pub board: String,
    pub no: i64,
    pub last_modified: i64,
    pub replies: i64,
    #[serde(default)]
    pub page: i32
}
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ThreadJob {
    pub id: i64,
    pub board: String,
    pub no: i64,
    pub last_modified: i64,
    pub replies: i64,
    pub page: i32
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ImageInfo {
    pub board: String,
    pub no: i64,
    pub url: String,
    pub thumbnail_url: String,
    pub ext: String,
    pub page: i32,
    pub file_sha256: Option<String>,
    pub thumbnail_sha256: Option<String>
}
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
pub struct ImageJob {
    pub id: i64,
    pub board: String,
    pub no: i64,
    pub url: String,
    pub thumbnail_url: String,
    pub ext: String,
    pub page: i32,
    pub file_sha256: Option<String>,
    pub thumbnail_sha256: Option<String>
}

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub struct Image {
    pub md5: String,
    pub md5_base32: String,
    pub thumbnail: bool,
    pub full_image: bool
}
#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct Board {
    pub name: String,
    pub full_images: bool,
    pub archive: bool,
    pub enable_search: bool,
}
// From /boards.json endpoint
#[derive(Debug, Clone, Deserialize, Serialize, Default, Eq, PartialEq)]
pub struct BoardInfo {
    pub board: String,
    pub title: String,
    pub ws_board: i64,
    pub per_page: i64,
    pub pages: i64,
    pub max_filesize: i64,
    pub max_webm_filesize: i64,
    pub max_comment_chars: i64,
    pub max_webm_duration: i64,
    pub bump_limit: i64,
    pub image_limit: i64,
    pub meta_description: String,
    #[serde(default)]
    pub spoilers: i64,
    #[serde(default)]
    pub custom_spoilers: i64,
    #[serde(default)]
    pub is_archived: i64,
    #[serde(default)]
    pub troll_flags: i64,
    #[serde(default)]
    pub country_flags: i64,
    #[serde(default)]
    pub user_ids: i64,
    #[serde(default)]
    pub oekaki: i64,
    #[serde(default)]
    pub sjis_tags: i64,
    #[serde(default)]
    pub code_tags: i64,
    #[serde(default)]
    pub math_tags: i64,
    #[serde(default)]
    pub text_only: i64,
    #[serde(default)]
    pub forced_anon: i64,
    #[serde(default)]
    pub webm_audio: i64,
    #[serde(default)]
    pub require_subject: i64,
    #[serde(default)]
    pub min_image_width: i64,
    #[serde(default)]
    pub min_image_height: i64
}
#[derive(Debug, Clone, Deserialize, Serialize, Default, Eq, PartialEq)]
pub struct BoardsList {
    pub boards: Vec<BoardInfo>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, Eq, PartialEq)]
pub struct ThreadNo {
    pub resto: i64
}
#[derive(Debug, Clone, Deserialize, Serialize, Default, Eq, PartialEq)]
pub struct IndexPage {
    pub threads: Vec<IndexThread>
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, Eq, PartialEq)]
pub struct IndexSearchResults {
    pub posts: Vec<Post>,
    pub total_results: i64
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, Eq, PartialEq)]
pub struct IndexThread {
    pub posts: Vec<IndexPost>
}
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
pub struct IndexPost {
    #[serde(flatten)]
    pub inner_post: Post,
    pub omitted_posts: i64,
    pub omitted_images: i64,
}

impl From<Thread> for IndexThread {
    fn from(thread: Thread) -> Self {
        if thread.posts.len() < 1 {
            return Self {
                posts: Vec::new()
            }
        }
        let (op, mut thread_posts): (Post, Vec<Post>) = match thread.posts.clone().split_first_mut() {
            Some((opref, postsref)) => (opref.clone(), postsref.to_vec()),
            None => return Self {posts: Vec::new()}
        };

        let mut kept_posts = Vec::new();
        for _ in 0..5 {
            if let Some(post) = thread_posts.pop() {
                kept_posts.push(post);
            }
        }
        let omitted_posts = thread_posts.len() as i64;
        let omitted_images: i64 = thread_posts.iter().filter(|p| p.tim != 0).count() as i64;
        let mut index_posts = Vec::new();
        index_posts.push(
            IndexPost {
                inner_post: op,
                omitted_posts,
                omitted_images
            }
        );
        for post in kept_posts.into_iter().rev() {
            index_posts.push(
                IndexPost {
                    inner_post: post,
                    omitted_posts,
                    omitted_images
                }
            )
        }
        Self {
            posts: index_posts
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
pub struct BoardsStatus {
    pub boards: Vec<Board>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
pub struct PurgeReport {
    pub full_files_deleted: u64,
    pub thumbnails_deleted: u64,
    pub full_files_failed: u64,
    pub thumbnails_failed: u64,
    pub removed_posts: u64,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
pub struct User {
    pub name: String,
    pub password_hash: String,
    pub role: UserRole
}

/**
 * Represents the level of privilege of a user in the system.
 */
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq, Copy)]
pub enum UserRole {
    Admin = 0,
    Mod = 1,
    Janitor = 2,
    #[default]
    Anonymous = 99
}

/**
 * Implement UserRole greater than and less than for sorting. 
 * Roles with a smaller integer value are considered "greater" than roles with a larger integer value.
 * This is because the UserRole enum is ordered from most to least privileged.
 */
impl std::cmp::PartialOrd for UserRole {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // Reverse the comparison to make the smaller integer value "greater"
        Some((*other as i32).cmp(&(*self as i32)))
    }
}

impl FromStr for UserRole {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "admin" => Ok(UserRole::Admin),
            "mod" => Ok(UserRole::Mod),
            "janitor" => Ok(UserRole::Janitor),
            _ => Ok(UserRole::Anonymous)
        }
    }
}

impl From<String> for UserRole {
    fn from(s: String) -> Self {
        match s.as_str() {
            "admin" => UserRole::Admin,
            "mod" => UserRole::Mod,
            "janitor" => UserRole::Janitor,
            _ => UserRole::Anonymous
        }
    }
}

impl From<UserRole> for i32 {
    fn from(role: UserRole) -> Self {
        match role {
            UserRole::Admin => 0,
            UserRole::Mod => 1,
            UserRole::Janitor => 2,
            UserRole::Anonymous => 99
        }
    }
}

impl From<i32> for UserRole {
    fn from(i: i32) -> Self {
        match i {
            0 => UserRole::Admin,
            1 => UserRole::Mod,
            2 => UserRole::Janitor,
            _ => UserRole::Anonymous
        }
    }
}

impl Type<sqlx::Postgres> for UserRole {
    fn type_info() -> PgTypeInfo {
        <i32 as Type<sqlx::Postgres>>::type_info()
    }
}

impl<'r> Decode<'r, sqlx::Postgres> for UserRole {
    fn decode(value: PgValueRef<'r>) -> Result<Self, BoxDynError> {
        let int_value: i32 = Decode::<Postgres>::decode(value)?;
        int_value.try_into().map_err(|_| "failed to parse enum".into())
    }
}

impl Encode<'_, sqlx::Postgres> for UserRole {
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> IsNull {
        let int_value = *self as i32;
        Encode::<Postgres>::encode_by_ref(&int_value, buf)
    }
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserRole::Admin => write!(f, "admin"),
            UserRole::Mod => write!(f, "mod"),
            UserRole::Janitor => write!(f, "janitor"),
            UserRole::Anonymous => write!(f, "anonymous")
        }
    }
}

pub struct StoredFile {
    pub file_id: i64,
    pub sha256: String,
    pub file_ext: String,
    pub is_thumbnail: bool,
    #[allow(dead_code)]
    pub hidden: bool,
}
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
pub struct ModLogInfo {
    pub log_id: i64,
    pub user_name: String,
    pub reason: String,
    pub comment: Option<String>,
}
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
pub struct ModLogAction {
    pub no: i64,
    pub board: String,
    pub file_sha256: Option<String>,
    pub action: String,
    pub is_thumbnail: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
pub struct ModLogEntry {
    pub log_info: ModLogInfo,
    pub actions: Vec<ModLogAction>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
pub struct ModLog {
    pub entries: Vec<ModLogEntry>
}
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
pub struct UserReport {
    pub no: i64,
    pub board: String,
    pub reason: String,
    pub comment: Option<String>
}
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
pub struct UserReports {
    pub reports: Vec<UserReport>
}

pub enum ModActionType {
    BlacklistImage,
    HidePost,
    HidePostContent,
    HidePostFile,
    UndoBlacklistImage,
    UnhidePost,
    UnhidePostContent,
    UnhidePostFile,
}

// Implement FromStr for ModActionType

impl FromStr for ModActionType {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "blacklist_image" => Ok(ModActionType::BlacklistImage),
            "hide_post" => Ok(ModActionType::HidePost),
            "hide_post_content" => Ok(ModActionType::HidePostContent),
            "hide_post_file" => Ok(ModActionType::HidePostFile),
            "undo_blacklist_image" => Ok(ModActionType::UndoBlacklistImage),
            "unhide_post" => Ok(ModActionType::UnhidePost),
            "unhide_post_content" => Ok(ModActionType::UnhidePostContent),
            "unhide_post_file" => Ok(ModActionType::UnhidePostFile),
            _ => Err(())
        }
    }
}

// Implement From for ModActionType

impl From<String> for ModActionType {
    fn from(s: String) -> Self {
        match s.as_str() {
            "blacklist_image" => ModActionType::BlacklistImage,
            "hide_post" => ModActionType::HidePost,
            "hide_post_content" => ModActionType::HidePostContent,
            "hide_post_file" => ModActionType::HidePostFile,
            "undo_blacklist_image" => ModActionType::UndoBlacklistImage,
            "unhide_post" => ModActionType::UnhidePost,
            "unhide_post_content" => ModActionType::UnhidePostContent,
            "unhide_post_file" => ModActionType::UnhidePostFile,
            _ => ModActionType::HidePost
        }
    }
}

// Implement ToString for ModActionType

impl std::string::ToString for ModActionType {
    fn to_string(&self) -> String {
        match self {
            ModActionType::BlacklistImage => "blacklist_image".to_string(),
            ModActionType::HidePost => "hide_post".to_string(),
            ModActionType::HidePostContent => "hide_post_content".to_string(),
            ModActionType::HidePostFile => "hide_post_file".to_string(),
            ModActionType::UndoBlacklistImage => "undo_blacklist_image".to_string(),
            ModActionType::UnhidePost => "unhide_post".to_string(),
            ModActionType::UnhidePostContent => "unhide_post_content".to_string(),
            ModActionType::UnhidePostFile => "unhide_post_file".to_string(),
        }
    }
}