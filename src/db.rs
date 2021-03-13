use diesel::prelude::*;
use diesel::pg::PgConnection;
use dotenv::dotenv;
use std::env;

use crate::models::Post;

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
    let res = diesel::insert_into(table)
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
        ).execute(&connection)?;
    Ok(res)
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