use std::time::Duration;

use diesel::{Queryable, Insertable, AsChangeset};
use serde::{Deserialize, Serialize};

use crate::schema::posts;
use crate::schema::images;

#[derive(Debug, Clone, Deserialize, Serialize, Queryable, Insertable, Default, AsChangeset)]
#[table_name="posts"]
#[primary_key((board, no))] 
pub struct Post {
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
    pub archived_on: i64
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Thread {
    pub posts: Vec<Post>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThreadsPage {
    pub page: i64,
    pub threads: Vec<ThreadInfo>
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThreadInfo {
    pub no: i64,
    pub last_modified: i64,
    pub replies: i64
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImageInfo {
    pub url: String,
    pub filename: String,
    pub md5: String
}

#[derive(Debug, Clone, Deserialize, Serialize, Queryable)]
pub struct Board {
    pub name: String,
    pub wait_time: Duration,
    pub last_modified: i64
}

#[derive(Debug, Clone, Deserialize, Serialize, Queryable, Insertable)]
#[table_name="images"]
pub struct Image {
    pub md5: String
}