use std::env;

#[allow(unused_imports)]
use crate::models::{Post, Image, PostUpdate, Board, Thread, ImageInfo, ImageJob, ThreadInfo, ThreadJob, ThreadNo};

pub async fn sqlx_connection() -> sqlx::Pool<sqlx::Postgres> {
    use sqlx::postgres::PgPoolOptions;
    dotenv::dotenv().ok();
    let pool = PgPoolOptions::new()
        .max_connections(50)
        .connect(&env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set")).await
        .expect("Failed to connect to database");
    pool
}

#[derive(Clone)]
pub struct DBClient {
    pub pool: sqlx::Pool<sqlx::Postgres>,
}

impl DBClient {
    pub async fn new() -> Self {
        Self {
            pool: sqlx_connection().await
        }
    }
    pub async fn get_all_boards(&self) -> anyhow::Result<Vec<Board>> {
        let boards = sqlx::query_as!(Board,
            "SELECT * FROM boards ORDER BY name ASC"
        ).fetch_all(&self.pool)
        .await?;
        Ok(boards)
    }
    pub async fn get_board(&self, board_name: &String) -> anyhow::Result<Option<Board>> {
        let board = sqlx::query_as!(Board,
            "SELECT * FROM boards WHERE name = $1",
            board_name,
        ).fetch_optional(&self.pool)
        .await?;
        Ok(board)
    }
    pub async fn delete_board(&self, board_name: &String) -> anyhow::Result<u64> {
        let res: u64 = sqlx::query!(
            "DELETE FROM boards WHERE name = $1",
            board_name,
        ).execute(&self.pool)
        .await?
        .rows_affected();
        Ok(res)
    }
    pub async fn insert_board(&self, board: &Board) -> anyhow::Result<Board> {
        let job = sqlx::query_as!(Board,
            "
            INSERT INTO boards (name, full_images, archive)
            VALUES
            ($1, $2, $3)
            ON CONFLICT(name) DO
            UPDATE SET
            full_images = $2,
            archive = $3
            RETURNING *;
            ",
            board.name,
            board.full_images,
            board.archive
        ).fetch_one(&self.pool)
        .await?;
        Ok(job)
    }
    pub async fn get_image_jobs(&self, limit: i64) -> anyhow::Result<Vec<ImageJob>> {
        let jobs = sqlx::query_as!(ImageJob,
            "
            SELECT * FROM image_backlog
            ORDER BY (page, id) ASC
            LIMIT $1
            ",
            limit
        ).fetch_all(&self.pool)
        .await?;
        Ok(jobs)
    }
    pub async fn delete_image_job(&self, job_id: i64) -> anyhow::Result<u64> {
        let res: u64 = sqlx::query!(
            "DELETE FROM image_backlog WHERE id = $1",
            job_id,
        ).execute(&self.pool)
        .await?
        .rows_affected();
        Ok(res)
    }
    pub async fn get_image_job(&self, job_id: i64) -> anyhow::Result<Option<ImageJob>> {
        let job = sqlx::query_as!(ImageJob,
            "
            SELECT * FROM  image_backlog WHERE id = $1
            ",
            job_id,
        ).fetch_optional(&self.pool)
        .await?;
        Ok(job)
    }
    pub async fn insert_image_job(&self, img: &ImageInfo) -> anyhow::Result<ImageJob> {
        let job = sqlx::query_as!(ImageJob,
            "
            INSERT INTO image_backlog (
                board, -- 1
                no, -- 2
                url, -- 3
                thumbnail_url, -- 4
                ext, -- 5
                page, -- 6
                file_sha256, -- 7
                thumbnail_sha256 -- 8
            )
            VALUES
            ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT(board, no) DO UPDATE
            SET 
            page = $6
            WHERE image_backlog.board = $1 AND image_backlog.no = $2
            RETURNING *;
            ",
            img.board, //1
            img.no, //2
            img.url, //3
            img.thumbnail_url, //4
            img.ext, //5
            img.page, //6
            img.file_sha256, //7
            img.thumbnail_sha256 //8
        ).fetch_one(&self.pool)
        .await?;
        Ok(job)
    }
    pub async fn delete_thread_job(&self, job_id: i64) -> anyhow::Result<u64> {
        let res: u64 = sqlx::query!(
            "DELETE FROM thread_backlog WHERE id = $1",
            job_id,
        ).execute(&self.pool)
        .await?
        .rows_affected();
        Ok(res)
    }
    pub async fn get_thread_jobs(&self, limit: i64) -> anyhow::Result<Vec<ThreadJob>> {
        let jobs = sqlx::query_as!(ThreadJob,
            "
            SELECT * FROM thread_backlog
            ORDER BY (page, id) ASC
            LIMIT $1
            ",
            limit
        ).fetch_all(&self.pool)
        .await?;
        Ok(jobs)
    }
    pub async fn insert_thread_job(&self, tinfo: &ThreadInfo) -> anyhow::Result<Option<ThreadJob>> {
        if let Some(post) = self.get_post(&tinfo.board, tinfo.no).await? {
            // if post is more recent or equal to thread_info date
            if post.last_modified >= tinfo.last_modified {
                return Ok(None)
            }
        }
        let job = sqlx::query_as!(ThreadJob,
            "
            INSERT INTO thread_backlog (board, no, last_modified, replies, page)
            VALUES
            ($1, $2, $3, $4, $5)
            ON CONFLICT(board, no, last_modified) DO
            UPDATE SET
            replies = $4,
            page = $5
            RETURNING *;
            ",
            tinfo.board,
            tinfo.no,
            tinfo.last_modified,
            tinfo.replies,
            tinfo.page
        ).fetch_one(&self.pool)
        .await?;
        Ok(Some(job))
    }
    pub async fn image_tim_to_sha256(&self, board: &String, image_tim: i64, thumb: bool) -> anyhow::Result<Option<String>> {
        let post_opt = sqlx::query_as!(Post,
            "
            SELECT *
            FROM posts
            WHERE board = $1
            AND tim = $2
            ",
            board,
            image_tim
        ).fetch_optional(&self.pool)
        .await?;
        if let Some(post) = post_opt {
            if thumb && !post.thumbnail_sha256.is_empty() {
                return Ok(Some(post.thumbnail_sha256))
            }
            if !thumb && !post.file_sha256.is_empty() {
                return Ok(Some(post.file_sha256))
            }
        }
        Ok(None)
    }
    pub async fn get_post(&self, board: &String, post_no: i64) -> anyhow::Result<Option<Post>> {
        let post = sqlx::query_as!(Post,
            "
            SELECT *
            FROM posts
            WHERE board = $1 AND no = $2
            ",
            board,
            post_no
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(post)
    }
    pub async fn delete_post(&self, board: &String, post_no: i64) -> anyhow::Result<u64> {
        let res: u64 = sqlx::query!(
            "
            DELETE FROM posts WHERE board = $1 AND no = $2
            ",
            board,
            post_no
        ).execute(&self.pool)
        .await?
        .rows_affected();
        Ok(res)
    }
    pub async fn get_thread_index(&self, board: &String, index: i64, limit: i64) -> anyhow::Result<Vec<Thread>> {
        let thread_ids = sqlx::query_as!(ThreadNo, 
            "
            SELECT t1.resto FROM posts t1
            LEFT JOIN posts t2
            ON t1.resto = t2.resto AND t1.no < t2.no
            WHERE t2.no IS NULL and t1.board = $1
            ORDER BY t1.no DESC OFFSET $2 LIMIT $3
            ",
            board,
            index*limit,
            limit
        ).fetch_all(&self.pool)
        .await?;
        let mut threads = Vec::new();
        for thread_id in thread_ids {
            if let Some(thread) = self.get_thread(board, thread_id.resto).await? {
                threads.push(thread);
            }
        }
        Ok(threads)
    }
    pub async fn get_thread(&self, board: &String, no: i64) -> anyhow::Result<Option<Thread>> {
        let posts = sqlx::query_as!(Post,
            "
            SELECT *
            FROM posts
            WHERE board = $1
            AND (no = $2 OR resto = $2)
            ORDER BY no ASC
            ",
            board,
            no
        )
        .fetch_all(&self.pool)
        .await?;
        if posts.is_empty() {
            return Ok(None);
        }
        if posts.len() == 1 && posts[0].resto != 0 {
            return Ok(None);
        }
        Ok(Some(Thread{posts}))
    }
    pub async fn set_post_files(&self, board: &String, no: i64, file_sha256: &String, thumbnail_sha256: &String) -> anyhow::Result<Option<Post>> {
        let post = sqlx::query_as!(Post,
            "
            UPDATE posts
            SET 
            file_sha256 = $1,
            thumbnail_sha256 = $2
            WHERE board = $3 AND no = $4
            RETURNING *
            ",
            file_sha256,
            thumbnail_sha256,
            board,
            no
        ).fetch_optional(&self.pool)
        .await?;
        Ok(post)
    }
    pub async fn insert_posts(&self, entries: &Vec<Post>) -> anyhow::Result<Vec<Post>> {
        let mut posts = Vec::new();
        for entry in entries {
            let post = sqlx::query_as!(Post,
                "
                INSERT INTO posts(
                    board, -- 1
                    no, -- 2
                    resto, -- 3
                    sticky, -- 4
                    closed, -- 5
                    now, -- 6
                    time, -- 7
                    name, -- 8
                    trip, -- 9
                    id, -- 10
                    capcode, -- 11
                    country, -- 12
                    country_name, -- 13
                    sub, -- 14
                    com, -- 15
                    tim, -- 16
                    filename, -- 17
                    ext, -- 18
                    fsize, -- 19
                    md5, -- 20
                    w, -- 21
                    h, -- 22
                    tn_w, -- 23
                    tn_h, -- 24
                    filedeleted, -- 25
                    spoiler, -- 26
                    custom_spoiler, -- 27
                    replies, -- 28
                    images, -- 29
                    bumplimit, -- 30
                    imagelimit, -- 31
                    tag, -- 32
                    semantic_url, -- 33
                    since4pass, -- 34
                    unique_ips, -- 35
                    m_img, -- 36
                    archived, -- 37
                    archived_on, -- 38
                    last_modified, -- 39
                    file_sha256, -- 40
                    thumbnail_sha256 -- 41
                )
                VALUES
                ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, 
                $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32, $33, $34, $35, $36, $37, $38, $39, $40, $41)
                ON CONFLICT (board, no) DO 
                UPDATE 
                SET
                closed = $5,
                sticky = $4,
                filedeleted = $25,
                replies = $28,
                images = $29,
                bumplimit = $30,
                imagelimit = $31,
                unique_ips = CASE WHEN posts.unique_ips < $35 THEN $35 ELSE posts.unique_ips END,
                archived = $37,
                archived_on = $38,
                last_modified = $39

                WHERE posts.board = $1 AND posts.no = $2
                RETURNING *;
                ",
                entry.board, //1
                entry.no, //2
                entry.resto, //3
                entry.sticky, //4
                entry.closed, //5
                entry.now, //6
                entry.time, //7
                entry.name, //8
                entry.trip, //9
                entry.id, //10
                entry.capcode, //11
                entry.country, //12
                entry.country_name, //13
                entry.sub, //14
                entry.com, //15
                entry.tim, //16
                entry.filename, //17
                entry.ext, //18
                entry.fsize, //19
                entry.md5, //20
                entry.w, //21
                entry.h, //22
                entry.tn_w, //23
                entry.tn_h, //24
                entry.filedeleted, //25
                entry.spoiler, //26
                entry.custom_spoiler, //27
                entry.replies, //28
                entry.images, //29
                entry.bumplimit, //30
                entry.imagelimit, //31
                entry.tag, //32
                entry.semantic_url, //33
                entry.since4pass, //34
                entry.unique_ips, //35
                entry.m_img, //36
                entry.archived, //37
                entry.archived_on, //38
                entry.last_modified, //39
                entry.file_sha256, //40,
                entry.thumbnail_sha256 //41
            )
            .fetch_one(&self.pool)
            .await?;
            posts.push(post);
        }
        
        Ok(posts)
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;

    fn run_async<F: std::future::Future>(f: F) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(f);
    }
    #[test]
    fn test_migrations(){
        run_async(migrations());
    }
    async fn migrations() {
        let dbc = DBClient::new().await;
        sqlx::migrate!()
        .run(&dbc.pool)
        .await.expect("Failed to migrate");
    }

    #[test]
    fn test_post_insert_upsert(){
        run_async(post_insert_upsert());
    }
    async fn post_insert_upsert() {
        let dbc = DBClient::new().await;
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
        assert_eq!(3usize, dbc.insert_posts(&vec![post1.clone(), post2.clone(), post3.clone()]).await.unwrap().len());
        post3.time = 5555;
        post2.images = 2222;
        post1.images = 1111;
        assert_eq!(3usize, dbc.insert_posts(&vec![post1.clone(), post2.clone(), post3.clone()]).await.unwrap().len());
    
        assert_eq!(1111, dbc.get_post(&post1.board, post1.no).await.unwrap().unwrap().images);
        assert_eq!(2222, dbc.get_post(&post2.board, post2.no).await.unwrap().unwrap().images);
        assert_eq!(4444, dbc.get_post(&post3.board, post3.no).await.unwrap().unwrap().time);
        
        assert_eq!(1, dbc.delete_post(&post1.board, post1.no).await.unwrap());
        assert_eq!(1, dbc.delete_post(&post2.board, post2.no).await.unwrap());
        assert_eq!(1, dbc.delete_post(&post3.board, post3.no).await.unwrap());
    }
    #[test]
    fn test_post_update(){
        run_async(post_update());
    }
    async fn post_update(){
        let dbc = DBClient::new().await;
        let mut post1 = Post::default();
        post1.board = "test".to_string();
        post1.no = 10;
        dbc.insert_posts(&vec![post1.clone()]).await.unwrap();
        post1.images = 55;
        assert_eq!(1usize, dbc.insert_posts(&vec![post1.clone()]).await.unwrap().len());
        assert_eq!(55, dbc.get_post(&post1.board, post1.no).await.unwrap().unwrap().images);
        post1.time = 500;
        assert_eq!(1usize, dbc.insert_posts(&vec![post1.clone()]).await.unwrap().len());
        assert_eq!(0, dbc.get_post(&post1.board, post1.no).await.unwrap().unwrap().time);
        post1.unique_ips = 30;
        assert_eq!(1usize, dbc.insert_posts(&vec![post1.clone()]).await.unwrap().len());
        assert_eq!(30, dbc.get_post(&post1.board, post1.no).await.unwrap().unwrap().unique_ips);
        post1.unique_ips = 0;
        assert_eq!(1usize, dbc.insert_posts(&vec![post1.clone()]).await.unwrap().len());
        assert_eq!(30, dbc.get_post(&post1.board, post1.no).await.unwrap().unwrap().unique_ips);
        assert_eq!(1u64, dbc.delete_post(&post1.board, post1.no).await.unwrap());
        assert_eq!(None, dbc.get_post(&post1.board, post1.no).await.unwrap());
    }
    // #[test]
    // fn test_job_update(){
    //     run_async(job_insert_update());
    // }
    // async fn job_insert_update() {
    //     let dbc = DBClient::new().await;
    //     let mut img = ImageInfo::default();
    //     img.board = "test_a".to_string();
    //     img.md5 = "test".to_string();
    //     img.md5_base32 = "test".to_string();
    //     img.url = "url1".to_string();
    //     let img_job = dbc.insert_image_job(&img).await.unwrap();
    //     img.url = "url2".to_string();
    //     let mut img_b = ImageInfo::default();
    //     img_b.board = "test_b".to_string();
    //     img_b.md5 = "test".to_string();
    //     img_b.md5_base32 = "test".to_string();
    //     img_b.url = "urlB".to_string();
    //     let img_b_job = dbc.insert_image_job(&img_b).await.unwrap();
        
    //     dbc.insert_image_job(&img).await.unwrap();

    //     assert_eq!("url2".to_string(), dbc.get_image_job(&img.board, &img.md5).await.unwrap().unwrap().url);
    //     assert_eq!("urlB".to_string(), dbc.get_image_job(&img_b.board, &img_b.md5).await.unwrap().unwrap().url);
    //     assert_eq!(1u64, dbc.delete_image_job(img_b_job.id).await.unwrap());
    //     assert_eq!(1u64, dbc.delete_image_job(img_job.id).await.unwrap());
    //     assert_eq!(None, dbc.get_image_job(&img.board, &img.md5).await.unwrap());
    //     assert_eq!(None, dbc.get_image_job(&img_b.board, &img_b.md5).await.unwrap());
    // }
    #[test]
    fn test_many_post_update(){
        run_async(many_post_update());
    }
    async fn many_post_update(){
        let dbc = DBClient::new().await;
        let mut post = dbc.get_post(&"vip".to_string(), 103205).await.unwrap().unwrap();
        post.board = "test".to_string();
        for i in 0..100000 {
            post.last_modified = i;
            dbc.insert_posts(&vec![post.clone()]).await.unwrap();
        }
    }
}