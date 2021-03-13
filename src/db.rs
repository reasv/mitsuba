use diesel::prelude::*;
use diesel::debug_query;
use diesel::pg::Pg;
use diesel::pg::PgConnection;
use dotenv::dotenv;
use std::env;

use crate::models::{Post, Image};

pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url))
}

pub fn insert_posts(entries: Vec<Post>) -> anyhow::Result<usize> {
    use crate::schema::posts::dsl::*;
    use crate::schema::posts::table;
    let connection = establish_connection();
    
    let query = diesel::insert_into(table)
        .values(&entries)
        .on_conflict((board, no))
        .do_update().set(
            (
                closed.eq(closed),
                sticky.eq(sticky),
                filedeleted.eq(filedeleted),
                replies.eq(replies),
                images.eq(images),
                bumplimit.eq(bumplimit),
                imagelimit.eq(imagelimit),
                unique_ips.eq(unique_ips),
                archived.eq(archived),
            )
        );
    println!("{}", debug_query::<Pg, _>(&query));
    Ok(query.execute(&connection)?)
}
pub fn _load_posts() {
    use crate::schema::posts::dsl::*;

    let connection = establish_connection();
    let results = posts.filter(resto.eq(0)).load::<Post>(&connection).expect("Error loading posts");

    for post in results {
        println!("{:?}", post);
        println!("----------\n");
    }
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

#[cfg(test)]
#[test]
fn image_operations() {
    let img_md5 = "test".to_string();
    assert_eq!(false, image_exists(&img_md5).unwrap());
    assert_eq!(1, insert_image(&Image{ md5: img_md5.clone() }).unwrap());
    assert_eq!(true, image_exists(&img_md5).unwrap());
    assert_eq!(1, delete_image(&img_md5).unwrap());
    assert_eq!(false, image_exists(&img_md5).unwrap());
}

#[test]
fn thread_operations() {
    let mut post1 = Post::default();
    post1.board = "biz".to_string();
    post1.no = 10;
    let mut post2 = Post::default();
    post2.board = "biz".to_string();
    post2.no = 12;
    post1.images = 1;
    post2.images = 2;
    insert_posts(vec![post1, post2]).unwrap();
}
#[test]
fn thread_operations2() {
    let mut post1 = Post::default();
    post1.board = "biz".to_string();
    post1.no = 10;
    let mut post2 = Post::default();
    post2.board = "biz".to_string();
    post2.no = 12;
    post1.images = 77;
    post2.images = 88;
    let mut post3 = Post::default();
    post3.board = "biz".to_string();
    post3.no = 15;
    post3.time = 1556;
    insert_posts(vec![post1, post2, post3]).unwrap();
}