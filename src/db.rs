use diesel::prelude::*;
use diesel::pg::PgConnection;
use dotenv::dotenv;
use std::env;

use crate::models::{Post, Image, PostUpdate, Board};

pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url))
}

pub fn insert_posts(entries: Vec<Post>) -> anyhow::Result<usize> {
    use crate::schema::posts::table;
    let connection = establish_connection();
    
    let res = diesel::insert_into(table)
        .values(&entries)
        .on_conflict_do_nothing()
        .execute(&connection)?;
    
    for entry in &entries {
        update_post(entry)?;
    }
    Ok(res)
}
pub fn update_post(entry: &Post) -> anyhow::Result<usize> {
    use crate::schema::posts::dsl::*;
    let connection = establish_connection();
    let target = posts.filter(board.eq(&entry.board)).filter(no.eq(&entry.no));
    let res = diesel::update(target).set(&PostUpdate::from(entry)).execute(&connection)?;
    Ok(res)
}
pub fn get_post(board_name: &String, post_no: i64) -> anyhow::Result<Option<Post>> {
    use crate::schema::posts::dsl::*;
    let connection = establish_connection();
    let post = posts.filter(board.eq(board_name)).filter(no.eq(&post_no)).first::<Post>(&connection).optional()?;
    Ok(post)
}
pub fn delete_post(board_name: &String, post_no: i64) -> anyhow::Result<usize>{
    use crate::schema::posts::dsl::*;
    let connection = establish_connection();
    let res = diesel::delete(posts.filter(board.eq(board_name)).filter(no.eq(&post_no)))
        .execute(&connection)?;
    Ok(res)
}

pub fn image_exists(img_md5: &String) -> anyhow::Result<bool>{
    use crate::schema::images::dsl::*;
    let connection = establish_connection();
    match images.find(img_md5).get_result::<Image>(&connection).optional()? {
        Some(_) => Ok(true),
        None => Ok(false)
    }
}
pub fn insert_image(img: &Image) -> anyhow::Result<usize> {
    use crate::schema::images::table;
    let connection = establish_connection();
    let res = diesel::insert_into(table)
        .values(img)
        .on_conflict_do_nothing()
        .execute(&connection)?;
    Ok(res)
}
pub fn delete_image(img_md5: &String) -> anyhow::Result<usize> {
    use crate::schema::images::dsl::*;
    let connection = establish_connection();
    let res = diesel::delete(images.filter(md5.eq(img_md5)))
        .execute(&connection)?;
    Ok(res)
}
pub fn insert_board(board: &Board) -> anyhow::Result<usize> {
    use crate::schema::boards::table;
    use crate::schema::boards::dsl::*;
    let connection = establish_connection();
    let res = diesel::insert_into(table)
        .values(board)
        .on_conflict(name)
        .do_update().set(
            (
                wait_time.eq(board.wait_time),
                full_images.eq(board.full_images),
                last_modified.eq(board.last_modified)
            )
        )
        .execute(&connection)?;
    Ok(res)
}
pub fn delete_board(board_name: &String) -> anyhow::Result<usize> {
    use crate::schema::boards::table;
    use crate::schema::boards::dsl::*;
    let connection = establish_connection();
    let res = diesel::delete(
        boards.filter(name.eq(board_name))
        ).execute(&connection)?;
    Ok(res)
}
pub fn get_board(board_name: &String) -> anyhow::Result<Option<Board>> {
    use crate::schema::boards::dsl::*;
    let connection = establish_connection();
    let post = boards.filter(name.eq(board_name)).first::<Board>(&connection).optional()?;
    Ok(post)
}
pub fn get_all_boards() -> anyhow::Result<Vec<Board>> {
    use crate::schema::boards::dsl::*;
    let connection = establish_connection();
    let post = boards.load::<Board>(&connection)?;
    Ok(post)
}


#[cfg(test)]
#[test]
fn image_operations() {
    let img_md5 = "test".to_string();
    assert_eq!(false, image_exists(&img_md5).unwrap());
    assert_eq!(1, insert_image(&Image{ md5: img_md5.clone(), thumbnail: true, full_image: true, md5_base32:"test".to_string()}).unwrap());
    assert_eq!(true, image_exists(&img_md5).unwrap());
    assert_eq!(1, delete_image(&img_md5).unwrap());
    assert_eq!(false, image_exists(&img_md5).unwrap());
}

#[test]
fn insert_upsert_test() {
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
    assert_eq!(3, insert_posts(vec![post1.clone(), post2.clone(), post3.clone()]).unwrap());
    post3.time = 5555;
    post2.images = 2222;
    post1.images = 1111;
    assert_eq!(0, insert_posts(vec![post1.clone(), post2.clone(), post3.clone()]).unwrap());

    assert_eq!(1111, get_post(&post1.board, post1.no).unwrap().unwrap().images);
    assert_eq!(2222, get_post(&post2.board, post2.no).unwrap().unwrap().images);
    assert_eq!(4444, get_post(&post3.board, post3.no).unwrap().unwrap().time);
    
    assert_eq!(1, delete_post(&post1.board, post1.no).unwrap());
    assert_eq!(1, delete_post(&post2.board, post2.no).unwrap());
    assert_eq!(1, delete_post(&post3.board, post3.no).unwrap());
}

#[test]
fn update_test(){
    let mut post1 = Post::default();
    post1.board = "test".to_string();
    post1.no = 10;
    insert_posts(vec![post1.clone()]).unwrap();
    post1.images = 55;
    assert_eq!(1, update_post(&post1).unwrap());
    assert_eq!(55, get_post(&post1.board, post1.no).unwrap().unwrap().images);
    post1.time = 500;
    assert_eq!(1, update_post(&post1).unwrap());
    assert_eq!(0, get_post(&post1.board, post1.no).unwrap().unwrap().time);
    post1.unique_ips = 30;
    assert_eq!(1, update_post(&post1).unwrap());
    assert_eq!(30, get_post(&post1.board, post1.no).unwrap().unwrap().unique_ips);
    post1.unique_ips = 0;
    assert_eq!(1, update_post(&post1).unwrap());
    assert_eq!(30, get_post(&post1.board, post1.no).unwrap().unwrap().unique_ips);
    assert_eq!(1, delete_post(&post1.board, post1.no).unwrap());
    assert_eq!(None, get_post(&post1.board, post1.no).unwrap());
}