use std::env;
use std::sync::Arc;

use diesel::prelude::*;
use diesel::pg::{PgConnection, Pg};
use dotenv::dotenv;
use diesel::r2d2::{ Pool, ConnectionManager, PoolError};
use diesel::{sql_query, debug_query, sql_types::{BigInt, Varchar}, query_builder::{DebugQuery, SqlQuery} };
use diesel::expression::UncheckedBind;
use crate::models::{Post, Image, PostUpdate, Board, Thread, ImageInfo, ImageJob, ThreadNo};

pub type PgPool = Pool<ConnectionManager<PgConnection>>;

#[allow(unused_macros)]
macro_rules! do_async {
    ($self:ident.$func_name:ident($($arg:ident),+)) => ({
        // `stringify!` will convert the expression *as it is* into a string.
        $( let $arg = $arg.clone(); )+
        let self_ref = $self.clone();
        // $self.$func_name($(&$arg),+);
        tokio::task::spawn_blocking(move || {self_ref.$func_name($(&$arg),+)})
    });
}

macro_rules! gen_async {
    ($func_name:ident, pub fn $func:ident(&$self:ident, $(&(ref $ref_arg:ident) : $ref_typ: ty),+, $($arg:ident: $typ: ty),+) -> $ret:ty $b:block) => {
        pub fn $func(&$self, $($ref_arg : $ref_typ),+, $($arg : $typ),+) -> $ret $b

        pub async fn $func_name(&$self, $($ref_arg: $ref_typ),+, $($arg: $typ),+,) -> $ret {
            $( let $ref_arg = $ref_arg.clone(); )+
            $( let $arg = $arg.clone(); )+
            let self_ref = $self.clone();
            tokio::task::spawn_blocking(move || {self_ref.$func($(&$ref_arg),+,$($arg),+)}).await?
        }
        
    };
    ($func_name:ident, pub fn $func:ident(&$self:ident, $($ref_arg:ident : $ref_typ: ty),+) -> $ret:ty $b:block) => {
        pub fn $func(&$self, $($ref_arg : $ref_typ),+) -> $ret $b

        pub async fn $func_name(&$self, $($ref_arg: $ref_typ),+) -> $ret {
            $( let $ref_arg = $ref_arg.clone(); )+
            let self_ref = $self.clone();
            tokio::task::spawn_blocking(move || {self_ref.$func($(&$ref_arg),+)}).await?
        }
        
    };
    ($func_name:ident, pub fn $func:ident(&$self:ident) -> $ret:ty $b:block) => {
        pub fn $func(&$self) -> $ret $b

        pub async fn $func_name(&$self) -> $ret {
            let self_ref = $self.clone();
            tokio::task::spawn_blocking(move || {self_ref.$func()}).await?
        }
        
    };
}

fn init_pool(database_url: &str) -> Result<PgPool, PoolError> {
       let manager = ConnectionManager::<PgConnection>::new(database_url);
       Pool::builder().build(manager)
}
pub fn establish_connection() -> PgPool {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    init_pool(&database_url).expect("Failed to create pool")
}
#[derive(Clone)]
pub struct DBClient {
    pub pool: Arc<PgPool>,
}
impl DBClient {
    pub fn new() -> Self {
        Self {
            pool: Arc::new(establish_connection())
        }
    }
    gen_async!(insert_posts_async,
    pub fn insert_posts(&self, entries: &Vec<Post>) -> anyhow::Result<usize> {
        use crate::schema::posts::table;
        let connection = self.pool.get()?;
        
        let res = diesel::insert_into(table)
            .values(entries)
            .on_conflict_do_nothing()
            .execute(&connection)?;
        
        for entry in entries {
            self.update_post(entry)?;
        }
        Ok(res)
    });

    gen_async!(update_post_async,
    pub fn update_post(&self, entry: &Post) -> anyhow::Result<usize> {
        use crate::schema::posts::dsl::*;
        let connection = self.pool.get()?;

        let target = posts.filter(board.eq(&entry.board)).filter(no.eq(&entry.no));
        let res = diesel::update(target).set(&PostUpdate::from(entry)).execute(&connection)?;
        Ok(res)
    });

    gen_async!(get_thread_async,
    pub fn get_thread(&self, &(ref board_name): &String, post_no: i64) -> anyhow::Result<Option<Thread>> {
        use crate::schema::posts::dsl::*;
        let connection = self.pool.get()?;
        let all_posts = posts
            .filter(board.eq(board_name))
            .filter(no.eq(&post_no).or(resto.eq(&post_no)))
            .order(no.asc())
            .load::<Post>(&connection)?;
        if all_posts.is_empty() {
            Ok(None)
        } else {
            Ok(Some(Thread { posts: all_posts }))
        }
    });
    gen_async!(get_thread_index_ids_async,
    pub fn get_thread_index_ids(&self, &(ref board_name): &String, index: i64, limit: i64) -> anyhow::Result<Vec<i64>> {
        // Warning: Index starts from 0!!
        let connection = self.pool.get()?;
        let thread_ids: Vec<ThreadNo> = sql_query(
            "SELECT t1.resto FROM posts t1
            LEFT JOIN posts t2
            ON t1.resto = t2.resto AND t1.no < t2.no
            WHERE t2.no IS NULL and t1.board = $1
            ORDER BY t1.no DESC OFFSET $2 LIMIT $3")
            .bind::<Varchar, _>(board_name)
            .bind::<BigInt, _>(index*limit)
            .bind::<BigInt, _>(limit)
            .get_results(&connection)?;
        Ok(thread_ids.into_iter().map(|t|t.resto).collect())
    });
    gen_async!(get_thread_index_async,
        pub fn get_thread_index(&self, &(ref board_name): &String, index: i64, limit: i64) -> anyhow::Result<Vec<Thread>> {
            let post_nos = self.get_thread_index_ids(board_name, index, limit)?;
            Ok(self.get_threads(board_name, post_nos)?)
    });
    gen_async!(get_threads_async,
    pub fn get_threads(&self, &(ref board_name): &String, post_no: Vec<i64>) -> anyhow::Result<Vec<Thread>> {
        use crate::schema::posts::dsl::*;
        let connection = self.pool.get()?;
        let all_posts = posts
            .filter(board.eq(board_name))
            .filter(no.eq_any(&post_no).or(resto.eq_any(&post_no)))
            .order(no.asc())
            .load::<Post>(&connection)?;
        let mut threads_map = std::collections::HashMap::new();
        for post in all_posts.clone() {
            if post.resto == 0 {
                threads_map.insert(post.no, Thread { posts: vec![post]});
            }
        }
        for post in all_posts {
            if post.resto != 0 {
               match threads_map.get_mut(&post.resto){
                   Some(thread) => thread.posts.push(post),
                   None => continue
               }
            }
        }
        let mut threads = Vec::new();
        for postn in post_no {
            match threads_map.get(&postn) {
                Some(thread) => threads.push(thread.clone()),
                None => continue
            }
        }
        Ok(threads)
    });

    gen_async!(get_post_async,
    pub fn get_post(&self, &(ref board_name): &String, post_no: i64) -> anyhow::Result<Option<Post>> {
        use crate::schema::posts::dsl::*;
        let connection = self.pool.get()?;
        let post = posts.filter(board.eq(board_name)).filter(no.eq(&post_no)).first::<Post>(&connection).optional()?;
        Ok(post)
    });

    gen_async!(delete_post_async,
    pub fn delete_post(&self, &(ref board_name): &String, post_no: i64) -> anyhow::Result<usize> {
        use crate::schema::posts::dsl::*;
        let connection = self.pool.get()?;
        let res = diesel::delete(posts.filter(board.eq(board_name)).filter(no.eq(&post_no)))
            .execute(&connection)?;
        Ok(res)
    });

    gen_async!(image_exists_async,
    pub fn image_exists(&self, img_md5: &String) -> anyhow::Result<bool> {
        use crate::schema::images::dsl::*;
        let connection = self.pool.get()?;
        match images.find(img_md5).get_result::<Image>(&connection).optional()? {
            Some(_) => Ok(true),
            None => Ok(false)
        }
    });

    gen_async!(image_exists_full_async,
    pub fn image_exists_full(&self, img_md5: &String) -> anyhow::Result<(bool, bool)> {
        use crate::schema::images::dsl::*;
        let connection = self.pool.get()?;
        match images.find(img_md5).get_result::<Image>(&connection).optional()? {
            Some(i) => Ok((i.thumbnail, i.full_image)),
            None => Ok((false, false))
        }
    });

    gen_async!(insert_image_async,
    pub fn insert_image(&self, img: &Image) -> anyhow::Result<usize> {
        use crate::schema::images::table;
        let connection = self.pool.get()?;
        let res = diesel::insert_into(table)
            .values(img)
            .on_conflict_do_nothing()
            .execute(&connection)?;
        Ok(res)
    });

    gen_async!(delete_image_async,
    pub fn delete_image(&self, img_md5: &String) -> anyhow::Result<usize> {
        use crate::schema::images::dsl::*;
        let connection = self.pool.get()?;
        let res = diesel::delete(images.filter(md5.eq(img_md5)))
            .execute(&connection)?;
        Ok(res)
    });

    gen_async!(image_tim_to_md5_async,
    pub fn image_tim_to_md5(&self, &(ref board_name): &String, image_tim: i64) -> anyhow::Result<Option<String>> {
        use crate::schema::posts::dsl::*;
        let connection = self.pool.get()?;
        match posts.filter(tim.eq(image_tim)).filter(board.eq(board_name)).first::<Post>(&connection).optional()? {
            Some(p) => Ok(Some(p.md5)),
            None => Ok(None)
        }
    });

    gen_async!(insert_image_job_async,
    pub fn insert_image_job(&self, img: &ImageInfo) -> anyhow::Result<usize> {
        use crate::schema::image_backlog::table;
        use crate::schema::image_backlog::dsl::*;
        let connection = self.pool.get()?;
        let res = diesel::insert_into(table)
            .values(img)
            .on_conflict((board, md5))
            .do_update().set(
                (
                    url.eq(&img.url),
                    thumbnail_url.eq(&img.thumbnail_url)
                )
            )
            .execute(&connection)?;
        Ok(res)
    });

    gen_async!(get_image_job_async, 
    pub fn get_image_job(&self, board_name: &String, img_md5: &String) -> anyhow::Result<Option<ImageJob>> {
        use crate::schema::image_backlog::dsl::*;
        let connection = self.pool.get()?;
        match image_backlog.filter(md5.eq(img_md5).and(board.eq(board_name))).first::<ImageJob>(&connection).optional()? {
            Some(i) => Ok(Some(i)),
            None => Ok(None)
        }
    });

    gen_async!(get_image_jobs_async, 
    pub fn get_image_jobs(&self) -> anyhow::Result<Vec<ImageJob>> {
        use crate::schema::image_backlog::dsl::*;
        let connection = self.pool.get()?;
        let jobs = image_backlog.order(id.asc()).load::<ImageJob>(&connection)?;
        Ok(jobs)
    });

    gen_async!(delete_image_job_async,
    pub fn delete_image_job(&self, board_name: &String, img_md5: &String) -> anyhow::Result<usize> {
        use crate::schema::image_backlog::dsl::*;
        let connection = self.pool.get()?;
        let res = diesel::delete(image_backlog.filter(md5.eq(img_md5).and(board.eq(board_name))))
            .execute(&connection)?;
        Ok(res)
    });

    gen_async!(insert_board_async,
    pub fn insert_board(&self, board: &Board) -> anyhow::Result<usize> {
        use crate::schema::boards::table;
        use crate::schema::boards::dsl::*;
        let connection = self.pool.get()?;
        let res = diesel::insert_into(table)
            .values(board)
            .on_conflict(name)
            .do_update().set(
                (
                    wait_time.eq(board.wait_time),
                    full_images.eq(board.full_images),
                    last_modified.eq(board.last_modified),
                    archive.eq(board.archive)
                )
            )
            .execute(&connection)?;
        Ok(res)
    });

    gen_async!(delete_board_async,
    pub fn delete_board(&self, board_name: &String) -> anyhow::Result<usize> {
        use crate::schema::boards::dsl::*;
        let connection = self.pool.get()?;
        let res = diesel::delete(
            boards.filter(name.eq(board_name))
            ).execute(&connection)?;
        Ok(res)
    });

    gen_async!(get_board_async, 
    pub fn get_board(&self, board_name: &String) -> anyhow::Result<Option<Board>> {
        use crate::schema::boards::dsl::*;
        let connection = self.pool.get()?;
        let post = boards.filter(name.eq(board_name)).first::<Board>(&connection).optional()?;
        Ok(post)
    });

    gen_async!(get_all_boards_async,
    pub fn get_all_boards(&self) -> anyhow::Result<Vec<Board>> {
        use crate::schema::boards::dsl::*;
        let connection = self.pool.get()?;
        let post = boards.load::<Board>(&connection)?;
        Ok(post)
    });
}

#[cfg(test)]
#[test]
fn image_operations() {
    let dbc = DBClient::new();
    let img_md5 = "test".to_string();
    assert_eq!(false, dbc.image_exists(&img_md5).unwrap());
    assert_eq!(1, dbc.insert_image(&Image{ md5: img_md5.clone(), thumbnail: true, full_image: true, md5_base32:"test".to_string()}).unwrap());
    assert_eq!(true, dbc.image_exists(&img_md5).unwrap());
    assert_eq!(1, dbc.delete_image(&img_md5).unwrap());
    assert_eq!(false, dbc.image_exists(&img_md5).unwrap());
}

#[test]
fn insert_upsert_test() {
    let dbc = DBClient::new();
    let mut post1 = Post::default();
    post1.board = "test".to_string();
    post1.no = 10;
    post1.time = 1337;
    post1.images = 77;
    let mut post2 = Post::default();
    post2.board = "test".to_string();
    post2.no = 11;
    post2.time = 1559;
    post2.images = 55;
    let mut post3 = Post::default();
    post3.board = "test".to_string();
    post3.no = 12;
    post3.time = 4444;
    assert_eq!(3, dbc.insert_posts(&vec![post1.clone(), post2.clone(), post3.clone()]).unwrap());
    post3.time = 5555;
    post2.images = 2222;
    post1.images = 1111;
    assert_eq!(0, dbc.insert_posts(&vec![post1.clone(), post2.clone(), post3.clone()]).unwrap());

    assert_eq!(1111, dbc.get_post(&post1.board, post1.no).unwrap().unwrap().images);
    assert_eq!(2222, dbc.get_post(&post2.board, post2.no).unwrap().unwrap().images);
    assert_eq!(4444, dbc.get_post(&post3.board, post3.no).unwrap().unwrap().time);
    
    assert_eq!(1, dbc.delete_post(&post1.board, post1.no).unwrap());
    assert_eq!(1, dbc.delete_post(&post2.board, post2.no).unwrap());
    assert_eq!(1, dbc.delete_post(&post3.board, post3.no).unwrap());
}

#[test]
fn update_test(){
    let dbc = DBClient::new();
    let mut post1 = Post::default();
    post1.board = "test".to_string();
    post1.no = 10;
    dbc.insert_posts(&vec![post1.clone()]).unwrap();
    post1.images = 55;
    assert_eq!(1, dbc.update_post(&post1).unwrap());
    assert_eq!(55, dbc.get_post(&post1.board, post1.no).unwrap().unwrap().images);
    post1.time = 500;
    assert_eq!(1, dbc.update_post(&post1).unwrap());
    assert_eq!(0, dbc.get_post(&post1.board, post1.no).unwrap().unwrap().time);
    post1.unique_ips = 30;
    assert_eq!(1, dbc.update_post(&post1).unwrap());
    assert_eq!(30, dbc.get_post(&post1.board, post1.no).unwrap().unwrap().unique_ips);
    post1.unique_ips = 0;
    assert_eq!(1, dbc.update_post(&post1).unwrap());
    assert_eq!(30, dbc.get_post(&post1.board, post1.no).unwrap().unwrap().unique_ips);
    assert_eq!(1, dbc.delete_post(&post1.board, post1.no).unwrap());
    assert_eq!(None, dbc.get_post(&post1.board, post1.no).unwrap());
}

#[test]
fn job_test() {
    let dbc = DBClient::new();
    let mut img = ImageInfo::default();
    img.board = "test_a".to_string();
    img.md5 = "test".to_string();
    img.md5_base32 = "test".to_string();
    img.url = "url1".to_string();
    assert_eq!(1, dbc.insert_image_job(&img).unwrap());
    img.url = "url2".to_string();
    let mut img_b = ImageInfo::default();
    img_b.board = "test_b".to_string();
    img_b.md5 = "test".to_string();
    img_b.md5_base32 = "test".to_string();
    img_b.url = "urlB".to_string();
    assert_eq!(1, dbc.insert_image_job(&img_b).unwrap());
    assert_eq!(1, dbc.insert_image_job(&img).unwrap());
    assert_eq!("url2".to_string(), dbc.get_image_job(&img.board, &img.md5).unwrap().unwrap().url);
    assert_eq!("urlB".to_string(), dbc.get_image_job(&img_b.board, &img_b.md5).unwrap().unwrap().url);
    assert_eq!(1, dbc.delete_image_job(&img.board, &img.md5).unwrap());
    assert_eq!(1, dbc.delete_image_job(&img_b.board, &img_b.md5).unwrap());
}
#[test]
fn index_test() {
    let dbc = DBClient::new();
    println!("{:?}", dbc.get_thread_index(&"i".to_string(), 0, 5).unwrap());
    println!("{:?}", dbc.get_thread_index(&"i".to_string(), 1, 5).unwrap());
}