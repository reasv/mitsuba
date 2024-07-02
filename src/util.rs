use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::env;

use base64::engine::general_purpose::STANDARD;
use base32::{Alphabet, encode};
use base64::Engine;
use unicode_truncate::UnicodeTruncateStr;
use sha2::{Sha256, Digest};
use weighted_rs::{SmoothWeight, Weight};

use crate::models::{ImageInfo, Post, Thread};

pub fn hash_file(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    encode(Alphabet::Rfc4648{padding: false}, hasher.finalize().as_slice())
}

fn bad_hash(s: String) -> i64 {
    let mut msg = 0i64;
    let j = s.len();
    for i in 0..j {
        msg = ((msg << 5) - msg) + (s.bytes().nth(i).unwrap_or(0) as i64);
    }
    msg
}
pub fn string_to_idcolor(s: String) -> String {
    let hash = bad_hash(s);
    let r = (hash >> 24) & 0xFF;
    let g = (hash >> 16) & 0xFF;
    let b = (hash >> 8) & 0xFF;
    let c = ((r as f64 * 0.299) + (g as f64 * 0.587) + (b as f64* 0.114)) > 125f64;
    let text = match c {
        true => "black",
        false => "white"
    };
    format!("background-color: rgb({}, {}, {}); color: {};",r,g,b,text)
}
pub fn shorten_string(maxlen: usize, s: String) -> String {
    if s.len() > maxlen {
        let (ss, _) = s.unicode_truncate(maxlen);
        ss.to_string() + &"(...)".to_string()
    } else {
        s
    }
}
pub fn get_board_page_api_url(board: &String) -> String {
    format!("https://a.4cdn.org/{}/threads.json", board)
}
pub fn get_board_archive_api_url(board: &String) -> String {
    format!("https://a.4cdn.org/{}/archive.json", board)
}
pub fn get_thread_api_url(board: &String, tid: &String) -> String {
    format!("https://a.4cdn.org/{}/thread/{}.json", board, tid)
}

pub fn get_post_image_info(board: &String, page: i32, post: &Post) -> Option<ImageInfo> {
    if post.tim == 0 || post.filedeleted == 1 {
        return None // no image
    }
    let url = format!("https://i.4cdn.org/{}/{}{}", board, post.tim, post.ext);
    let thumbnail_url = format!("https://i.4cdn.org/{}/{}s.jpg", board, post.tim);
    Some(ImageInfo{url, thumbnail_url, ext: post.ext.clone(), file_sha256: post.file_sha256.clone(), thumbnail_sha256: post.thumbnail_sha256.clone(), page, no: post.no, board: board.clone()})
}

pub fn base64_to_32(b64: String) -> anyhow::Result<String> {
    let binary = STANDARD.decode(b64)?;
    let s = encode(Alphabet::Rfc4648{padding: false}, binary.as_slice());
    Ok(s)
}

pub fn get_file_folder(sha256: &String, is_thumb: bool) -> PathBuf {
    let data_folder_str = std::env::var("DATA_ROOT").unwrap_or("data".to_string());
    let image_folder = Path::new(&data_folder_str).join("images");
    let folder = match is_thumb {
        true => image_folder.join("thumb"),
        false => image_folder.join("full")
    };
    folder.join(&sha256[0..2]).join(&sha256[2..3])
}

pub fn get_file_url(sha256: &String, ext: &String, is_thumb: bool) -> String {
    let folder = match is_thumb {
        true => "thumb",
        false => "full"
    };
    if sha256.len() < 3 {
        return "/static/image/404-Angelguy.png".to_string();
    }

    format!("/img/{}/{}/{}/{}{}", folder, &sha256[0..2], &sha256[2..3], sha256, ext)
}

pub fn bool_from_env(env_var: &String) -> bool {
    bool::from_str(
        &env::var(env_var)
        .unwrap_or("false".to_string())
    )
    .unwrap_or(false)
}

pub fn int_from_env(env_var: &str, default: isize) -> isize {
    isize::from_str(
        &env::var(env_var)
        .unwrap_or(default.to_string())
    )
    .unwrap_or(default)
}

pub fn get_proxy_config() -> SmoothWeight<Option<reqwest::Url>> {
    let mut sw = SmoothWeight::new();
    if !bool_from_env(&"PROXY_ONLY".to_string()) {
        sw.add(None, int_from_env("PROXY_WEIGHT_SELF", 1));
    }
    let mut i = 0;
    while let Some(url) = env::var(format!("PROXY_URL_{}", i)).ok() {
        if let Some(proxy) = reqwest::Url::parse(&url).ok() {
            sw.add(Some(proxy), int_from_env(&format!("PROXY_WEIGHT_{}", i), 1));
        }
        i+=1;
    }
    sw
}
pub fn get_host_string(url_opt: &Option<reqwest::Url>) -> Option<String> {
    if let Some(url) = url_opt {
        if let Some(host_str) = url.host_str() {
            return Some(host_str.to_string().clone());
        }
    }
    None
}

pub fn strip_nullchars(s: &String) -> String {
    s.replace("\u{00}", "\u{2400}")
}

pub fn process_hidden_post(post: &Post) -> Option<Post> {
    if post.mitsuba_post_hidden {
        return None;
    }
    let mut post = post.clone();
    if post.mitsuba_file_hidden {
        post.thumbnail_sha256 = "".to_string();
        post.file_sha256 = "".to_string();
    }
    if post.mitsuba_com_hidden {
        post.com = "<b><i>[Hidden]</i></b>".to_string();
    }
    Some(post)
}

pub fn process_hidden_thread(thread: &Thread) -> Option<Thread> {
    if thread.posts[0].mitsuba_post_hidden {
        return None;
    }
    let mut thread = thread.clone();
    thread.posts = thread.posts.into_iter().filter_map(|p| process_hidden_post(&p)).collect();
    Some(thread)
}