use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default, Eq, PartialEq)]
pub struct Post {
    #[serde(skip)]
    pub post_id: i64,
    #[serde(default)]
    pub board: String,
    pub no: i64,
    pub resto: i64,
    #[serde(default)]
    pub sticky: i64,
    #[serde(default)]
    pub closed: i64,
    pub now: String,
    pub time: i64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub trip: String,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub capcode: String,
    #[serde(default)]
    pub country: String,
    #[serde(default)]
    pub country_name: String,
    #[serde(default)]
    pub sub: String,
    #[serde(default)]
    pub com: String,
    #[serde(default)]
    pub tim: i64,
    #[serde(default)]
    pub filename: String,
    #[serde(default)]
    pub ext: String,
    #[serde(default)]
    pub fsize: i64,
    #[serde(default)]
    pub md5: String,
    #[serde(default)]
    pub w: i64,
    #[serde(default)]
    pub h: i64,
    #[serde(default)]
    pub tn_w: i64,
    #[serde(default)]
    pub tn_h: i64,
    #[serde(default)]
    pub filedeleted: i64,
    #[serde(default)]
    pub spoiler: i64,
    #[serde(default)]
    pub custom_spoiler: i64,
    #[serde(default)]
    pub replies: i64,
    #[serde(default)]
    pub images: i64,
    #[serde(default)]
    pub bumplimit: i64,
    #[serde(default)]
    pub imagelimit: i64,
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub semantic_url: String,
    #[serde(default)]
    pub since4pass: i64,
    #[serde(default)]
    pub unique_ips: i64,
    #[serde(default)]
    pub m_img: i64,
    #[serde(default)]
    pub archived: i64,
    #[serde(default)]
    pub archived_on: i64,
    #[serde(default)]
    pub last_modified: i64
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

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
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
    pub md5: String,
    pub md5_base32: String,
    pub board: String,
    pub url: String,
    pub thumbnail_url: String,
    pub filename: String,
    pub thumbnail_filename: String,
}
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
pub struct ImageJob {
    pub id: i64,
    pub md5: String,
    pub md5_base32: String,
    pub board: String,
    pub url: String,
    pub thumbnail_url: String,
    pub filename: String,
    pub thumbnail_filename: String,
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
    pub archive: bool
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
pub struct IndexThread {
    pub posts: Vec<IndexPost>
}
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
pub struct IndexPost {
    #[serde(flatten)]
    pub inner_post: Post,
    pub omitted_posts: i64,
    pub omitted_images: i64,
    pub last_modified: i64
}

impl From<Thread> for IndexThread {
    fn from(thread: Thread) -> Self {
        if thread.posts.len() < 1 {
            return Self {
                posts: Vec::new()
            }
        }
        // safe because we checked length to be 1 or gt
        let last_modified = thread.posts.last().unwrap().time;
        
        let (op, mut thread_posts): (Post, Vec<Post>) = match thread.posts.clone().split_first_mut() {
            Some((opref, postsref)) => (opref.clone(), postsref.to_vec()),
            None => return Self {posts: Vec::new()}
        };
        // let op = thread_posts[0].clone();
        let mut kept_posts = Vec::new();
        for _ in 0..4 {
            match thread_posts.pop() {
                Some(post) =>  kept_posts.push(post),
                None => continue
            }
        }
        let omitted_posts = thread_posts.len() as i64;
        let omitted_images: i64 = thread_posts.iter().filter(|p| p.tim != 0).count() as i64;
        let mut index_posts = Vec::new();
        index_posts.push(
            IndexPost {
                inner_post: op,
                omitted_posts,
                omitted_images,
                last_modified
            }
        );
        for post in kept_posts.into_iter().rev() {
            index_posts.push(
                IndexPost {
                    inner_post: post,
                    omitted_posts,
                    omitted_images,
                    last_modified
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