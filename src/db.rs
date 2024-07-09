use std::env;
use std::sync::Arc;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

use dashmap::DashSet;
use log::{debug, warn};
#[allow(unused_imports)]
use metrics::{gauge, increment_gauge, decrement_gauge, counter, histogram};

use crate::models::{ModActionType, ModLog, ModLogInfo, StoredFile, User, UserReport};

#[allow(unused_imports)]
use crate::models::{Post, Image, PostUpdate, Board, Thread, ImageInfo, ImageJob,
     ThreadInfo, ThreadJob, ThreadNo, UserRole, ModLogEntry, ModLogAction};

use crate::util::get_post_image_info;
#[allow(unused_imports)]
use crate::util::strip_nullchars;
use crate::util::{process_hidden_post, process_hidden_thread};

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
    post_hashes: Arc<DashSet<u64>>,
    tinfo_hashes: Arc<DashSet<u64>>
}

impl DBClient {
    pub async fn new() -> Self {
        Self {
            pool: sqlx_connection().await,
            post_hashes: Arc::new(DashSet::new()),
            tinfo_hashes: Arc::new(DashSet::new())
        }
    }
    pub async fn get_image_backlog_size(&self, min_page: i32) -> anyhow::Result<i64> {
        struct Count {
            count: Option<i64>
        }
        let count = sqlx::query_as!(Count,
            "
            SELECT
            count(*)
            FROM image_backlog
            WHERE page >= $1
            ",
            min_page
        ).fetch_one(&self.pool).await?;
        Ok(count.count.unwrap_or(0))
    }
    pub async fn get_thread_backlog_size(&self, min_page: i32) -> anyhow::Result<i64> {
        struct Count {
            count: Option<i64>
        }
        let count = sqlx::query_as!(Count,
            "
            SELECT
            count(*)
            FROM thread_backlog
            WHERE page >= $1
            ",
            min_page
        ).fetch_one(&self.pool).await?;
        Ok(count.count.unwrap_or(0))
    }
    pub async fn get_stored_files(&self) -> anyhow::Result<i64> {
        struct Count {
            count: Option<i64>
        }
        let count = sqlx::query_as!(Count,
            "
            SELECT
            count(*)
            FROM files
            WHERE is_thumbnail != false
            "
        ).fetch_one(&self.pool).await?;
        Ok(count.count.unwrap_or(0))
    }
    pub async fn get_stored_thumbnails(&self) -> anyhow::Result<i64> {
        struct Count {
            count: Option<i64>
        }
        let count = sqlx::query_as!(Count,
            "
            SELECT
            count(*)
            FROM files
            WHERE is_thumbnail = true
            "
        ).fetch_one(&self.pool).await?;
        Ok(count.count.unwrap_or(0))
    }
    pub async fn get_missing_thumbnails(&self) -> anyhow::Result<i64> {
        struct Count {
            count: Option<i64>
        }
        let count = sqlx::query_as!(Count,
            "
            SELECT
            COUNT(*)
            FROM posts
            LEFT JOIN posts_files 
            ON posts_files.post_id = posts.post_id
            WHERE posts_files.thumbnail_id IS NULL
            AND tim != 0 AND filedeleted = 0 AND deleted_on = 0
            "
        ).fetch_one(&self.pool).await?;
        Ok(count.count.unwrap_or(0))
    }

    pub async fn schedule_missing_full_files(&self, board: &String) -> anyhow::Result<usize> {
        let posts_missing_full_images: Vec<Post> = sqlx::query_as!(Post,
            "
            SELECT
            posts.*,
            files.sha256 as file_sha256,
            thumbnails.hidden as mitsuba_file_hidden,
            thumbnails.sha256 as thumbnail_sha256,
            CASE 
                WHEN 
                blacklist_thumbnail.sha256 IS NOT NULL 
                OR 
                blacklist_file.sha256 IS NOT NULL
                THEN true
                ELSE false
            END AS mitsuba_file_blacklisted
            FROM posts
            
            LEFT JOIN posts_files
            ON posts_files.post_id = posts.post_id
            AND posts_files.idx = 0
            
            LEFT JOIN files
            ON files.file_id = posts_files.file_id
            
            LEFT JOIN files as thumbnails
            ON thumbnails.file_id = posts_files.thumbnail_id

            LEFT JOIN file_blacklist as blacklist_thumbnail
            ON thumbnails.sha256 = blacklist_thumbnail.sha256
            
            LEFT JOIN file_blacklist as blacklist_file
            ON files.sha256 = blacklist_file.sha256

            WHERE posts_files.file_id IS NULL
            AND board = $1
            AND tim != 0 AND filedeleted = 0 AND deleted_on = 0
            ",
            board
        ).fetch_all(&self.pool).await?;

        let image_infos: Vec<ImageInfo> = posts_missing_full_images.into_iter()
        .map(|post| {
            get_post_image_info(board, 5, &post) // page 5 gives it a middle priority
        })
        .filter_map(|img| img).collect();

        let mut job_counter = 0;
        for img in &image_infos {
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
                ON CONFLICT(board, no) DO NOTHING
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
            ).fetch_optional(&self.pool)
            .await?;
            if job.is_some() {
                job_counter += 1;
            }
        }
        Ok(job_counter)
    }


    pub async fn get_latest_images(
        &self,
        limit: i64,
        offset: i64,
        boards: Vec<String>
    ) -> anyhow::Result<Vec<Post>> {
        let posts = sqlx::query_as!(Post,
            "
            SELECT
            posts.*,
            files.sha256 as file_sha256,
            thumbnails.hidden as mitsuba_file_hidden,
            thumbnails.sha256 as thumbnail_sha256,
            CASE 
                WHEN 
                blacklist_thumbnail.sha256 IS NOT NULL 
                OR 
                blacklist_file.sha256 IS NOT NULL
                THEN true
                ELSE false
            END AS mitsuba_file_blacklisted
            FROM posts
            
            JOIN posts_files
            ON posts_files.post_id = posts.post_id
            AND posts_files.idx = 0
            
            LEFT JOIN files
            ON files.file_id = posts_files.file_id
            
            LEFT JOIN files as thumbnails
            ON thumbnails.file_id = posts_files.thumbnail_id

            LEFT JOIN file_blacklist as blacklist_thumbnail
            ON thumbnails.sha256 = blacklist_thumbnail.sha256
            
            LEFT JOIN file_blacklist as blacklist_file
            ON files.sha256 = blacklist_file.sha256

            WHERE board = ANY($1)
            AND thumbnails.hidden = false
            AND blacklist_thumbnail.sha256 IS NULL
            AND blacklist_file.sha256 IS NULL
            AND thumbnails.file_id IS NOT NULL
            ORDER BY last_modified DESC
            LIMIT $2 OFFSET $3
            ",
            &boards,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(posts)
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
            INSERT INTO boards (name, full_images, archive, enable_search)
            VALUES
            ($1, $2, $3, $4)
            ON CONFLICT(name) DO
            UPDATE SET
            full_images = $2,
            archive = $3,
            enable_search = $4
            RETURNING *;
            ",
            board.name,
            board.full_images,
            board.archive,
            board.enable_search
        ).fetch_one(&self.pool)
        .await?;
        Ok(job)
    }
    pub async fn get_image_jobs(&self, limit: i64) -> anyhow::Result<Vec<ImageJob>> {
        let jobs = sqlx::query_as!(ImageJob,
            "
            SELECT * FROM image_backlog
            ORDER BY page DESC, id ASC
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
            ORDER BY page DESC, id ASC
            LIMIT $1
            ",
            limit
        ).fetch_all(&self.pool)
        .await?;
        Ok(jobs)
    }
    fn get_threadinfo_hash(&self, tinfo: &ThreadInfo) -> u64 {
        let mut hasher = DefaultHasher::new();
        tinfo.hash(& mut hasher);
        hasher.finish()
    }
    fn insert_threadinfo_hash(&self, tinfo_hash: u64) {
        // Clear if it goes over 100 million items (~800MB)
        if self.tinfo_hashes.len() > 100000000 {
            warn!("Thread Job hash store reached over 100 million entries, clearing.");
            self.tinfo_hashes.clear();
            self.tinfo_hashes.shrink_to_fit();
        }

        self.tinfo_hashes.insert(tinfo_hash);
        gauge!("thread_jobs_hashes", self.tinfo_hashes.len() as f64);
    }
    pub async fn insert_thread_job(&self, tinfo: &ThreadInfo) -> anyhow::Result<Option<ThreadJob>> {
        let tinfo_hash = self.get_threadinfo_hash(&tinfo);
        if self.tinfo_hashes.contains(&tinfo_hash) {
            debug!("Skip adding duplicate thread job for /{}/{} [{}] ({} - {})", 
            tinfo.board, tinfo.no, tinfo.last_modified, tinfo.replies, tinfo.page);
            return Ok(None)
        }

        if let Some(post) = self.get_post(&tinfo.board, tinfo.no, false).await? {
            // if post is more recent or equal to thread_info date
            if post.last_modified >= tinfo.last_modified {
                self.insert_threadinfo_hash(tinfo_hash);
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
        
        counter!("thread_job_writes", 1);
        
        self.insert_threadinfo_hash(tinfo_hash);

        // Delete earlier updates to thread
        let res: u64 = sqlx::query!(
            "DELETE FROM thread_backlog WHERE board = $1 AND no = $2 AND last_modified < $3",
            tinfo.board,
            tinfo.no,
            tinfo.last_modified
        ).execute(&self.pool)
        .await?
        .rows_affected();

        if res > 0 {
            debug!("Deleting {} obsolete thread jobs", res);
        }
        Ok(Some(job))
    }
    pub async fn image_tim_to_sha256(&self, board: &String, image_tim: i64, thumb: bool, remove_hidden: bool) -> anyhow::Result<Option<String>> {
        let post_opt = sqlx::query_as!(Post,
            "
            SELECT
            posts.*,
            files.sha256 as file_sha256,
            thumbnails.hidden as mitsuba_file_hidden,
            thumbnails.sha256 as thumbnail_sha256,
            CASE 
                WHEN 
                blacklist_thumbnail.sha256 IS NOT NULL 
                OR 
                blacklist_file.sha256 IS NOT NULL
                THEN true
                ELSE false
            END AS mitsuba_file_blacklisted
            FROM posts
            
            LEFT JOIN posts_files
            ON posts_files.post_id = posts.post_id
            AND posts_files.idx = 0
            
            LEFT JOIN files
            ON files.file_id = posts_files.file_id
            
            LEFT JOIN files as thumbnails
            ON thumbnails.file_id = posts_files.thumbnail_id

            LEFT JOIN file_blacklist as blacklist_thumbnail
            ON thumbnails.sha256 = blacklist_thumbnail.sha256
            
            LEFT JOIN file_blacklist as blacklist_file
            ON files.sha256 = blacklist_file.sha256

            WHERE board = $1
            AND tim = $2
            ",
            board,
            image_tim
        ).fetch_optional(&self.pool)
        .await?;
        if let Some(post_raw) = post_opt {
            let post = if remove_hidden {
                if let Some(post_hidden) = process_hidden_post(&post_raw) {
                    post_hidden
                } else {
                    return Ok(None)
                }
            } else {
                post_raw
            };
            if thumb {
                return Ok(post.thumbnail_sha256)
            }
            if !thumb {
                return Ok(post.file_sha256)
            }
        }
        Ok(None)
    }
    pub async fn get_post(&self, board: &String, post_no: i64, remove_hidden: bool) -> anyhow::Result<Option<Post>> {
        let post = sqlx::query_as!(Post,
            "
            SELECT
            posts.*,
            files.sha256 as file_sha256,
            thumbnails.hidden as mitsuba_file_hidden,
            thumbnails.sha256 as thumbnail_sha256,
            CASE 
                WHEN 
                blacklist_thumbnail.sha256 IS NOT NULL 
                OR 
                blacklist_file.sha256 IS NOT NULL
                THEN true
                ELSE false
            END AS mitsuba_file_blacklisted
            FROM posts
            
            LEFT JOIN posts_files
            ON posts_files.post_id = posts.post_id
            AND posts_files.idx = 0
            
            LEFT JOIN files
            ON files.file_id = posts_files.file_id
            
            LEFT JOIN files as thumbnails
            ON thumbnails.file_id = posts_files.thumbnail_id

            LEFT JOIN file_blacklist as blacklist_thumbnail
            ON thumbnails.sha256 = blacklist_thumbnail.sha256
            
            LEFT JOIN file_blacklist as blacklist_file
            ON files.sha256 = blacklist_file.sha256

            WHERE board = $1 AND no = $2
            ",
            board,
            post_no
        )
        .fetch_optional(&self.pool)
        .await?;
        if let Some(post) = post {
            if remove_hidden {
                return Ok(process_hidden_post(&post));
            }
            return Ok(Some(post));
        } else {
            return Ok(None);
        }
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
    pub async fn blacklist_file(&self, sha256: &String, action_id: i64) -> anyhow::Result<(u64, u64)> {
        let res: u64 = sqlx::query!(
            "
            INSERT INTO file_blacklist (sha256, action_id)
            VALUES ($1, $2)
            ON CONFLICT(sha256)
            DO NOTHING
            ",
            sha256,
            action_id
        ).execute(&self.pool)
        .await?
        .rows_affected();
        // Set image hidden on all posts that contain the blacklisted file.
        let res2: u64 = sqlx::query!(
            "
            UPDATE files
            SET hidden = true
            WHERE sha256 = $1
            ",
            sha256
        ).execute(&self.pool).await?.rows_affected();
        Ok((res, res2))
    }
    pub async fn is_file_blacklisted(&self, sha256: &String) -> anyhow::Result<bool> {
        struct Sha256Field {
            sha256: Option<String>
        }
        let hashes: Vec<Sha256Field> = sqlx::query_as!(Sha256Field,
            "
            SELECT sha256 FROM file_blacklist WHERE sha256 = $1
            ",
            sha256
            ).fetch_all(&self.pool)
            .await?;
        Ok(!hashes.is_empty())
    }

    pub async fn remove_file_blacklist(&self, sha256: &String) -> anyhow::Result<(u64, u64)> {
        let res: u64 = sqlx::query!(
            "
            DELETE FROM file_blacklist
            WHERE sha256 = $1
            ",
            sha256,
        ).execute(&self.pool)
        .await?
        .rows_affected();
        // Set image back to visible on all posts that contain the blacklisted file
        let res2: u64 = sqlx::query!(
            "
            UPDATE files
            SET hidden = false
            WHERE sha256 = $1
            ",
            sha256
        ).execute(&self.pool).await?.rows_affected();
        Ok((res, res2))
    }

    pub async fn get_thread_index(&self, board: &String, index: i64, limit: i64, remove_hidden: bool) -> anyhow::Result<Vec<Thread>> {
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
            if let Some(thread) = self.get_thread(board, thread_id.resto, remove_hidden).await? {
                threads.push(thread);
            }
        }
        Ok(threads)
    }
    pub async fn get_thread(&self, board: &String, no: i64, remove_hidden: bool) -> anyhow::Result<Option<Thread>> {
        let posts = sqlx::query_as!(Post,
            "
            SELECT
            posts.*,
            NULL as file_sha256,
            false as mitsuba_file_hidden,
            '' as thumbnail_sha256,
            CASE 
                WHEN 
                blacklist_thumbnail.sha256 IS NOT NULL 
                OR 
                blacklist_file.sha256 IS NOT NULL
                THEN true
                ELSE false
            END AS mitsuba_file_blacklisted
            FROM posts
            
            LEFT JOIN posts_files
            ON posts_files.post_id = posts.post_id
            AND posts_files.idx = 0
            
            LEFT JOIN files
            ON files.file_id = posts_files.file_id
            
            LEFT JOIN files as thumbnails
            ON thumbnails.file_id = posts_files.thumbnail_id

            LEFT JOIN file_blacklist as blacklist_thumbnail
            ON thumbnails.sha256 = blacklist_thumbnail.sha256
            
            LEFT JOIN file_blacklist as blacklist_file
            ON files.sha256 = blacklist_file.sha256

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
        let thread = Thread{posts};
        if remove_hidden {
            return Ok(process_hidden_thread(&thread));
        }
        Ok(Some(thread))
    }
    pub async fn add_post_file(&self, board: &String, no: i64, idx: i32, sha256: &String, ext: &String, is_thumbnail: bool) -> anyhow::Result<u64> {
        // Insert the files into the files table if they don't exist
        let file_id = if sha256.is_empty() {
            None
        } else {
            sqlx::query!(
                "
                INSERT INTO files (sha256, is_thumbnail, hidden, file_ext)
                VALUES ($1, $2, false, $3)
                ON CONFLICT(sha256) DO NOTHING
                RETURNING files.file_id;
                ",
                sha256,
                is_thumbnail,
                ext
            ).fetch_optional(&self.pool).await?
            .map(|f| f.file_id)
        };
        // Obtain the post_id for the post
        let post_id = sqlx::query!(
            "
            SELECT post_id FROM posts WHERE board = $1 AND no = $2
            ",
            board,
            no
        ).fetch_optional(&self.pool).await?
        .map(|f| f.post_id);
        
        if post_id.is_none() {
            return Ok(0);
        }
        let res: u64;
        if let Some(file_id) = file_id {
            // Insert the file references into the posts_files table
            if is_thumbnail {
                res = sqlx::query!(
                    "
                    INSERT INTO posts_files (post_id, thumbnail_id, idx, file_id)
                    VALUES ($1, $2, $3, NULL)
                    ON CONFLICT(post_id, idx) DO UPDATE
                    SET thumbnail_id = $2
                    ",
                    post_id,
                    file_id,
                    idx
                ).execute(&self.pool).await?
                .rows_affected();
            } else {
                res = sqlx::query!(
                    "
                    INSERT INTO posts_files (post_id, file_id, idx, thumbnail_id)
                    VALUES ($1, $2, $3, NULL)
                    ON CONFLICT(post_id, idx) DO UPDATE
                    SET file_id = $2
                    ",
                    post_id,
                    file_id,
                    idx
                ).execute(&self.pool).await?
                .rows_affected();
            }
        } else {
            res = 0;
        }
        Ok(res)
    }
    pub async fn set_post_deleted(&self, board: &String, no: i64, deleted_time: i64) -> anyhow::Result<Option<(i64, String)>> {
        struct PostId {
            no: i64,
            board: String
        }
        let post = sqlx::query_as!(PostId,
            "
            UPDATE posts
            SET 
            deleted_on = $1
            WHERE board = $2 AND no = $3
            RETURNING no, board
            ",
            deleted_time,
            board,
            no
        ).fetch_optional(&self.pool)
        .await?;
        Ok(post.map(|p| (p.no, p.board)))
    }
    pub async fn set_missing_posts_deleted(&self, board: &String, thread_no: i64, current_posts: Vec<i64>, deleted_time: i64) -> anyhow::Result<Vec<(i64, String)>> {
        // Given the current list of post ids in a thread, it sets all posts not in the list as deleted.
        struct PostId {
            no: i64,
            board: String
        }
        let posts = sqlx::query_as!(PostId,
            "
            UPDATE posts
            SET 
            deleted_on = $1
            WHERE board = $2 AND resto = $3 AND deleted_on = 0 AND no != ALL($4)
            RETURNING posts.no as no, posts.board as board
            ",
            deleted_time,
            board,
            thread_no,
            &current_posts
        ).fetch_all(&self.pool)
        .await?;
        Ok(posts.into_iter().map(|p| (p.no, p.board)).collect())
    }

    pub async fn get_orphaned_files(&self) -> anyhow::Result<Vec<StoredFile>> {
        let files = sqlx::query_as!(StoredFile,
            "
            SELECT
            files.file_id, 
            files.sha256,
            files.hidden,
            files.is_thumbnail,
            files.file_ext
            FROM files
            LEFT JOIN posts_files pf1 ON files.file_id = pf1.file_id
            LEFT JOIN posts_files pf2 ON files.file_id = pf2.thumbnail_id
            GROUP BY files.file_id
            HAVING COUNT(pf1.file_id) = 0 AND COUNT(pf2.thumbnail_id) = 0;
            "
        ).fetch_all(&self.pool)
        .await?;
        Ok(files)
    }

    pub async fn is_file_orphaned(&self, file_id: i64) -> anyhow::Result<bool> {
        struct Sha256Field {
            post_id: i64
        }
        let hashes: Vec<Sha256Field> = sqlx::query_as!(Sha256Field,
            "
            SELECT post_id FROM posts_files WHERE file_id = $1 OR thumbnail_id = $1
            ",
            file_id
            ).fetch_all(&self.pool)
            .await?;
        Ok(hashes.is_empty())
    }

    pub async fn remove_full_file_references_for_board(&self, board: &String) -> anyhow::Result<u64> {
        let res: u64 = sqlx::query!(
            "
            UPDATE posts_files
            SET file_id = NULL
            WHERE post_id IN (SELECT post_id FROM posts WHERE board = $1)
            ",
            board
        ).execute(&self.pool)
        .await?
        .rows_affected();
        Ok(res)
    }

    pub async fn delete_file(&self, sha256: &String) -> anyhow::Result<u64> {
        let res: u64 = sqlx::query!(
            "
            DELETE FROM files WHERE sha256 = $1
            ",
            sha256
        ).execute(&self.pool)
        .await?
        .rows_affected();
        Ok(res)
    }

    async fn purge_board_posts(&self, board_name: &String) -> anyhow::Result<u64> {
        let res: u64 = sqlx::query!(
            "
            DELETE FROM posts WHERE board = $1
            ",
            board_name
        ).execute(&self.pool)
        .await?
        .rows_affected();
        Ok(res)
    }
    
    pub async fn purge_board_backlogs(&self, board_name: &String) -> anyhow::Result<(u64, u64)> {
        let res_thread: u64 = sqlx::query!(
            "
            DELETE FROM thread_backlog WHERE board = $1
            ",
            board_name
        ).execute(&self.pool)
        .await?
        .rows_affected();
        let res_image: u64 = sqlx::query!(
            "
            DELETE FROM image_backlog WHERE board = $1
            ",
            board_name
        ).execute(&self.pool)
        .await?
        .rows_affected();
        Ok((res_thread, res_image))
    }

    pub async fn purge_board_data(&self, board_name: &String) -> anyhow::Result<u64> {
        self.delete_board(board_name).await?;
        self.purge_board_backlogs(board_name).await?;
        let posts_deleted = self.purge_board_posts(board_name).await?;
        Ok(posts_deleted)
    }

    fn get_post_hash(&self, post: &Post) -> u64 {
        let mut hasher = DefaultHasher::new();
        let mut hash_post = post.clone();
        hash_post.post_id = 0;
        // For OP we always write on new last_modified values.
        // For other posts, if this is the only change we won't consider it changed.
        if hash_post.resto != 0 {
            hash_post.last_modified = 0;
        }
        // ignore image hashes - image hashes are updated with set_post_files()
        hash_post.file_sha256 = None;
        hash_post.thumbnail_sha256 = None;
        hash_post.hash(& mut hasher);
        hasher.finish()
    }
    
    pub async fn insert_posts(&self, entries: &Vec<Post>) -> anyhow::Result<Vec<Post>> {

        let mut posts = Vec::new();
        for entry in entries {
            let hash = self.get_post_hash(entry);
            if self.post_hashes.contains(&hash) {
                debug!("Post has not changed, skipped (/{}/{})", entry.board, entry.no);
                continue;
            }
            struct PostId {
                post_id: i64
            }
            let _post_id = sqlx::query_as!(PostId,
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
                    deleted_on -- 40
                )
                VALUES
                ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, 
                $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32, $33, $34, $35, $36, $37, $38, $39, $40)
                ON CONFLICT (board, no) DO 
                UPDATE 
                SET
                closed = $5,
                sticky = $4,
                com = $15,
                filedeleted = $25,
                spoiler = $26,
                custom_spoiler = $27,
                replies = $28,
                images = $29,
                bumplimit = $30,
                imagelimit = $31,
                unique_ips = CASE WHEN posts.unique_ips < $35 THEN $35 ELSE posts.unique_ips END,
                archived = $37,
                archived_on = $38,
                last_modified = $39,
                deleted_on = $40

                WHERE posts.board = $1 AND posts.no = $2
                RETURNING post_id;
                ",
                entry.board, //1
                entry.no, //2
                entry.resto, //3
                entry.sticky, //4
                entry.closed, //5
                entry.now, //6
                entry.time, //7
                strip_nullchars(&entry.name), //8
                entry.trip, //9
                entry.id, //10
                entry.capcode, //11
                entry.country, //12
                entry.country_name, //13
                strip_nullchars(&entry.sub), //14
                strip_nullchars(&entry.com), //15
                entry.tim, //16
                strip_nullchars(&entry.filename), //17
                strip_nullchars(&entry.ext), //18
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
                strip_nullchars(&entry.tag), //32
                entry.semantic_url, //33
                entry.since4pass, //34
                entry.unique_ips, //35
                entry.m_img, //36
                entry.archived, //37
                entry.archived_on, //38
                entry.last_modified, //39
                entry.deleted_on // 40
            )
            .fetch_one(&self.pool)
            .await?;
            counter!("post_writes", 1);

            // Clear if it goes over 100 million items (~800MB)
            if self.post_hashes.len() > 100000000 {
                warn!("Post hash store reached over 100 million entries, clearing.");
                self.post_hashes.clear();
                self.post_hashes.shrink_to_fit();
            }
            self.post_hashes.insert(hash);
            gauge!("post_hashes", self.post_hashes.len() as f64);
            // we will only return new or updated posts.
            if let Some(post) = self.get_post(&entry.board, entry.no, false).await? {
                posts.push(post);
            }
        }
        
        Ok(posts)
    }
    pub async fn set_post_hidden_status(&self, board: &String, no: i64, hidden: bool, com_hidden: bool, file_hidden: bool) -> anyhow::Result<u64> {
        let mut res = sqlx::query!(
            "
            UPDATE posts
            SET 
            mitsuba_post_hidden = $1,
            mitsuba_com_hidden = $2
            WHERE board = $3 AND no = $4
            ",
            hidden,
            com_hidden,
            board,
            no
        ).execute(&self.pool)
        .await?
        .rows_affected();
        struct FileIds {
            file_id: Option<i64>,
            thumbnail_id: Option<i64>
        }
        // Retrieve the image for this post and set it hidden if it exists.
        let file_ids = sqlx::query_as!(FileIds,
            "
            SELECT
            posts_files.thumbnail_id as thumbnail_id,
            posts_files.file_id as file_id
            FROM posts
            LEFT JOIN posts_files
            ON posts_files.post_id = posts.post_id
            WHERE board = $1 AND no = $2
            ",
            board,
            no
        ).fetch_optional(&self.pool).await?;

        if let Some(file_ids) = file_ids {
            if let Some(file_id) = file_ids.file_id {
                res += sqlx::query!(
                    "
                    UPDATE files
                    SET hidden = $1
                    WHERE file_id = $2
                    ",
                    file_hidden,
                    file_id
                ).execute(&self.pool).await?.rows_affected();
            }
            if let Some(thumbnail_id) = file_ids.thumbnail_id {
                res += sqlx::query!(
                    "
                    UPDATE files
                    SET hidden = $1
                    WHERE file_id = $2
                    ",
                    file_hidden,
                    thumbnail_id
                ).execute(&self.pool).await?.rows_affected();
            }
        }

        Ok(res)
    }

    pub async fn posts_full_text_search(&self, board: &String, text_query: &String, page: i64, page_size: i64, remove_hidden: bool) -> anyhow::Result<(Vec<Post>, i64)> {
        let offset = page * page_size;
    
        // Query to count the total number of matching results
        let total_count_opt = sqlx::query_scalar!(
            "
            SELECT COUNT(*)
            FROM posts
            WHERE board = $1
            AND (
                $2 = '' OR to_tsvector('english', com) @@ plainto_tsquery('english', $2)
                OR $3 = '' OR to_tsvector('english', name) @@ plainto_tsquery('english', $3)
                OR $4 = '' OR to_tsvector('english', sub) @@ plainto_tsquery('english', $4)
                OR $5 = '' OR to_tsvector('english', filename) @@ plainto_tsquery('english', $5)
            )
            ",
            board,
            text_query,
            text_query,
            text_query,
            text_query
        ).fetch_one(&self.pool).await?;

        
        let total_count = total_count_opt.unwrap_or(0);
    
        // Query to fetch the posts
        let posts = sqlx::query_as!(
            Post,
            "
            SELECT
            posts.*,
            files.sha256 as file_sha256,
            thumbnails.hidden as mitsuba_file_hidden,
            thumbnails.sha256 as thumbnail_sha256,
            CASE 
                WHEN 
                blacklist_thumbnail.sha256 IS NOT NULL 
                OR 
                blacklist_file.sha256 IS NOT NULL
                THEN true
                ELSE false
            END AS mitsuba_file_blacklisted
            FROM posts
            
            LEFT JOIN posts_files
            ON posts_files.post_id = posts.post_id
            AND posts_files.idx = 0
            
            LEFT JOIN files
            ON files.file_id = posts_files.file_id
            
            LEFT JOIN files as thumbnails
            ON thumbnails.file_id = posts_files.thumbnail_id

            LEFT JOIN file_blacklist as blacklist_thumbnail
            ON thumbnails.sha256 = blacklist_thumbnail.sha256
            
            LEFT JOIN file_blacklist as blacklist_file
            ON files.sha256 = blacklist_file.sha256

            WHERE board = $1
            AND (
                $2 = '' OR to_tsvector('english', com) @@ plainto_tsquery('english', $2)
                OR $3 = '' OR to_tsvector('english', name) @@ plainto_tsquery('english', $3)
                OR $4 = '' OR to_tsvector('english', sub) @@ plainto_tsquery('english', $4)
                OR $5 = '' OR to_tsvector('english', filename) @@ plainto_tsquery('english', $5)
            )
            ORDER BY time DESC
            LIMIT $6 OFFSET $7
            ",
            board,
            text_query,
            text_query,
            text_query,
            text_query,
            page_size,
            offset
        ).fetch_all(&self.pool).await?;
    
        let posts = if remove_hidden {
            posts.into_iter().filter_map(|p| process_hidden_post(&p)).collect()
        } else {
            posts
        };
    
        Ok((posts, total_count))
    }

    pub async fn insert_user(&self, user: &User) -> anyhow::Result<()>{
        let role_id: i32 = user.role.into();
        let _ = sqlx::query!(
            "
            INSERT INTO users (name, password_hash, role)
            VALUES ($1, $2, $3)
            ON CONFLICT(name) DO NOTHING
            ",
            user.name,
            user.password_hash,
            role_id
        ).execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_user(&self, name: &String) -> anyhow::Result<Option<User>> {
        let user = sqlx::query_as!(User,
            "
            SELECT name, password_hash, role FROM users WHERE name = $1
            ",
            name
        ).fetch_optional(&self.pool)
        .await?;
        Ok(user)
    }

    pub async fn delete_user(&self, name: &String) -> anyhow::Result<u64> {
        let res: u64 = sqlx::query!(
            "DELETE FROM users WHERE name = $1",
            name,
        ).execute(&self.pool)
        .await?
        .rows_affected();
        Ok(res)
    }

    pub async fn get_users(&self) -> anyhow::Result<Vec<User>> {
        let users = sqlx::query_as!(User,
            "
            SELECT name, password_hash, role FROM users
            "
        ).fetch_all(&self.pool)
        .await?;
        Ok(users)
    }

    pub async fn change_password(&self, name: &String, password_hash: &String) -> anyhow::Result<u64> {
        let res: u64 = sqlx::query!(
            "UPDATE users SET password_hash = $1 WHERE name = $2",
            password_hash,
            name
        ).execute(&self.pool)
        .await?
        .rows_affected();
        Ok(res)
    }

    pub async fn change_role(&self, name: &String, role: UserRole) -> anyhow::Result<u64> {
        let role_id: i32 = role.into();
        let res: u64 = sqlx::query!(
            "UPDATE users SET role = $1 WHERE name = $2",
            role_id,
            name
        ).execute(&self.pool)
        .await?
        .rows_affected();
        Ok(res)
    }

    pub async fn create_moderation_log_entry(
        &self,
        user_name: Option<&String>,
        reason: Option<String>,
        comment: Option<String>
    ) -> anyhow::Result<i64> {
        let user_id = if let Some(name) = user_name {
            sqlx::query!(
            "SELECT user_id FROM users WHERE name = $1",
            name
        ).fetch_one(&self.pool)
            .await?.user_id
        } else {
            0
        };
        let reason_str = reason.unwrap_or("other".to_string());
        let log_id = sqlx::query!(
            "
            INSERT INTO moderation_log (user_id, reason, comment)
            VALUES ($1, $2, $3)
            RETURNING log_id
            ",
            user_id,
            reason_str,
            comment
        ).fetch_one(&self.pool)
        .await?.log_id;
        Ok(log_id)
    }

    pub async fn get_moderation_log(
        &self,
        page: i64,
        page_size: i64
    ) -> anyhow::Result<ModLog> {
        let offset = page * page_size;
        let logs = sqlx::query_as!(ModLogInfo,
            "
            SELECT
            moderation_log.log_id as log_id,
            users.name as user_name,
            moderation_log.reason as reason,
            moderation_log.comment as comment
            FROM moderation_log
            JOIN users
            ON users.user_id = moderation_log.user_id
            ORDER BY moderation_log.executed_at DESC
            LIMIT $1 OFFSET $2
            ",
            page_size,
            offset
        ).fetch_all(&self.pool)
        .await?;
        let mut log_entries = vec![];

        for log_info in logs {
            let actions = sqlx::query_as!(ModLogAction,
                "
                SELECT
                moderation_actions.action as action,
                posts.board as board,
                posts.no as no,
                files.sha256 as file_sha256,
                files.is_thumbnail as is_thumbnail
                FROM moderation_actions
                LEFT JOIN posts
                ON posts.post_id = moderation_actions.post_id
                LEFT JOIN files
                ON files.file_id = moderation_actions.file_id
                WHERE log_id = $1
                ",
                log_info.log_id
            ).fetch_all(&self.pool)
            .await?;
            log_entries.push(ModLogEntry{log_info, actions});
        }
        Ok(ModLog {
            entries: log_entries
        })
    }

    pub async fn get_image_ids(
        &self,
        board: &String,
        post_no: i64
    ) -> anyhow::Result<(Option<i64>, Option<i64>)> {
        let thumbnail_id = sqlx::query!(
            "
            SELECT thumbnail_id FROM posts_files
            LEFT JOIN posts
            ON posts.post_id = posts_files.post_id
            WHERE board = $1 AND no = $2
            ",
            board,
            post_no
        ).fetch_optional(&self.pool)
        .await?.map(|f| f.thumbnail_id);

        let file_id = sqlx::query!(
            "
            SELECT file_id FROM posts_files
            LEFT JOIN posts
            ON posts.post_id = posts_files.post_id
            WHERE board = $1 AND no = $2
            ",
            board,
            post_no
        ).fetch_optional(&self.pool)
        .await?.map(|f| f.file_id);

        Ok((thumbnail_id, file_id))
    }

    pub async fn register_mod_action(
        &self,
        log_id: i64,
        post_no: i64,
        board: &String,
        on_image: bool,
        action: ModActionType
    ) -> anyhow::Result<i64> {
        let post_id = self.get_post_id(board, post_no).await?;
        if post_id.is_none() {
            return Ok(0);
        }

        let (file_id, thumbnail_id) = if on_image {
            self.get_image_ids(board, post_no).await?
        } else {
            (None, None)
        };

        let action_id: i64 = sqlx::query!(
            "
            INSERT INTO moderation_actions (log_id, post_id, file_id, thumbnail_id, action)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING action_id
            ",
            log_id,
            post_id,
            file_id,
            thumbnail_id,
            action.to_string()
        ).fetch_one(&self.pool)
        .await?.action_id;
        Ok(action_id)
    }

    pub async fn get_post_id(&self, board: &String, post_no: i64) -> anyhow::Result<Option<i64>> {
        let post_id = sqlx::query!(
            "
            SELECT post_id FROM posts WHERE board = $1 AND no = $2
            ",
            board,
            post_no
        ).fetch_optional(&self.pool)
        .await?
        .map(|p| p.post_id);
        Ok(post_id)
    }

    pub async fn file_user_report(
        &self,
        post_no: i64,
        board: &String,
        reason: &String,
        comment: &String
    ) -> anyhow::Result<Option<u64>> {
        let post_id = self.get_post_id(board, post_no).await?;
        if post_id.is_none() {
            return Ok(None);
        }

        let res: u64 = sqlx::query!(
            "
            INSERT INTO user_reports (post_id, reason, comment)
            VALUES ($1, $2, $3)
            ",
            post_id,
            reason,
            comment
        ).execute(&self.pool)
        .await?
        .rows_affected();
        Ok(Some(res))
    }

    pub async fn get_user_reports(&self, page: i64, page_size: i64) -> anyhow::Result<Vec<UserReport>> {
        let offset = page * page_size;
        let reports = sqlx::query_as!(
            UserReport,
            "
            SELECT
            posts.board as board,
            posts.no as no,
            user_reports.reason as reason,
            user_reports.comment as comment
            FROM user_reports
            JOIN posts
            ON posts.post_id = user_reports.post_id
            ORDER BY user_reports.created_at DESC
            LIMIT $1 OFFSET $2
            ",
            page_size,
            offset
        ).fetch_all(&self.pool)
        .await?;
        Ok(reports)
    }

    pub async fn delete_user_report(&self, report_id: i64) -> anyhow::Result<u64> {
        let res: u64 = sqlx::query!(
            "
            DELETE FROM user_reports WHERE report_id = $1
            ",
            report_id
        ).execute(&self.pool)
        .await?
        .rows_affected();
        Ok(res)
    }

}

impl std::panic::UnwindSafe for DBClient {}
impl std::panic::RefUnwindSafe for DBClient {}


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
    
        assert_eq!(1111, dbc.get_post(&post1.board, post1.no, false).await.unwrap().unwrap().images);
        assert_eq!(2222, dbc.get_post(&post2.board, post2.no, false).await.unwrap().unwrap().images);
        assert_eq!(4444, dbc.get_post(&post3.board, post3.no, false).await.unwrap().unwrap().time);
        
        assert_eq!(1, dbc.delete_post(&post1.board, post1.no).await.unwrap());
        assert_eq!(1, dbc.delete_post(&post2.board, post2.no).await.unwrap());
        assert_eq!(1, dbc.delete_post(&post3.board, post3.no).await.unwrap());
    }
    #[test]
    fn test_post_nullchars(){
        run_async(post_insert_nullchars());
    }
    async fn post_insert_nullchars() {
        let dbc = DBClient::new().await;
        let mut post1 = Post::default();
        post1.board = "test".to_string();
        post1.no = 10;
        post1.time = 1337;
        post1.images = 77;
        post1.com = "test \u{00} test \u{00}".to_string();
        assert_eq!(1usize, dbc.insert_posts(&vec![post1.clone()]).await.unwrap().len());
    
        assert_eq!(77, dbc.get_post(&post1.board, post1.no, false).await.unwrap().unwrap().images);
        assert_eq!(1, dbc.delete_post(&post1.board, post1.no).await.unwrap());
    }
    #[test]
    fn post_history(){
        run_async(post_update_com());
    }
    async fn post_update_com() {
        let dbc = DBClient::new().await;
        let mut post1 = Post::default();
        post1.board = "test".to_string();
        post1.no = 10;
        post1.com = "Comment2".to_string();
        assert_eq!(1usize, dbc.insert_posts(&vec![post1.clone()]).await.unwrap().len());
        post1.images = 77;
        assert_eq!(1usize, dbc.insert_posts(&vec![post1.clone()]).await.unwrap().len());
        assert_eq!(77, dbc.get_post(&post1.board, post1.no, false).await.unwrap().unwrap().images);
        post1.com = "Comment Changed".to_string();
        assert_eq!(1usize, dbc.insert_posts(&vec![post1.clone()]).await.unwrap().len());
        post1.com = "Comment Changed Again".to_string();
        assert_eq!(1usize, dbc.insert_posts(&vec![post1.clone()]).await.unwrap().len());
    
        assert_eq!(post1.com, dbc.get_post(&post1.board, post1.no, false).await.unwrap().unwrap().com);
        assert_eq!(1, dbc.delete_post(&post1.board, post1.no).await.unwrap());
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
        assert_eq!(55, dbc.get_post(&post1.board, post1.no, false).await.unwrap().unwrap().images);
        post1.time = 500;
        assert_eq!(1usize, dbc.insert_posts(&vec![post1.clone()]).await.unwrap().len());
        assert_eq!(0, dbc.get_post(&post1.board, post1.no, false).await.unwrap().unwrap().time);
        post1.unique_ips = 30;
        assert_eq!(1usize, dbc.insert_posts(&vec![post1.clone()]).await.unwrap().len());
        assert_eq!(30, dbc.get_post(&post1.board, post1.no, false).await.unwrap().unwrap().unique_ips);
        post1.unique_ips = 0;
        assert_eq!(1usize, dbc.insert_posts(&vec![post1.clone()]).await.unwrap().len());
        assert_eq!(30, dbc.get_post(&post1.board, post1.no, false).await.unwrap().unwrap().unique_ips);
        assert_eq!(1u64, dbc.delete_post(&post1.board, post1.no).await.unwrap());
        assert_eq!(None, dbc.get_post(&post1.board, post1.no, false).await.unwrap());
    }
    #[test]
    fn test_post_deleted_detection(){
        run_async(post_deleted());
    }
    async fn post_deleted(){
        let dbc = DBClient::new().await;
        let mut post1 = Post::default();
        post1.board = "test".to_string();
        post1.resto = 1;
        post1.no = 1;
        
        let mut post2 = Post::default();
        post2.board = "test".to_string();
        post2.resto = 1;
        post2.no = 2;
        
        let mut post3 = Post::default();
        post3.board = "test".to_string();
        post3.resto = 1;
        post3.no = 3;

        let mut post4 = Post::default();
        post4.board = "test".to_string();
        post4.resto = 1;
        post4.no = 4;

        dbc.insert_posts(&vec![post1.clone(), post2.clone(), post3.clone(), post4.clone()]).await.unwrap();

        assert_eq!(0usize, dbc.set_missing_posts_deleted(&post1.board, post1.resto, vec![post1.no, post2.no, post3.no, post4.no], 10).await.unwrap().len());

        let nos: Vec<(i64, i64)> = dbc.set_missing_posts_deleted(&post1.board, post1.resto, vec![post1.no, post2.no, post4.no], 10).await
        .unwrap().iter().map(|p| (p.0, 10)).collect();
        assert_eq!(1, nos.len());
        assert_eq!((3, 10), nos[0]);

        let nos: Vec<(i64, i64)> = dbc.set_missing_posts_deleted(&post1.board, post1.resto, vec![post1.no, post2.no], 20).await
        .unwrap().iter().map(|p| (p.0, 20)).collect();
        assert_eq!(1, nos.len());
        assert_eq!((4, 20), nos[0]);
        let nos: Vec<(i64, i64)> = dbc.set_missing_posts_deleted(&post1.board, post1.resto, vec![post1.no], 30).await
        .unwrap().iter().map(|p| (p.0, 30)).collect();
        assert_eq!(1, nos.len());
        assert_eq!((2, 30), nos[0]);
        assert_eq!(0usize, dbc.set_missing_posts_deleted(&post1.board, post1.resto, vec![post1.no], 10).await.unwrap().len());

        assert_eq!(1u64, dbc.delete_post(&post1.board, post1.no).await.unwrap());
        assert_eq!(1u64, dbc.delete_post(&post2.board, post2.no).await.unwrap());
        assert_eq!(1u64, dbc.delete_post(&post3.board, post3.no).await.unwrap());
        assert_eq!(1u64, dbc.delete_post(&post4.board, post4.no).await.unwrap());
        assert_eq!(None, dbc.get_post(&post1.board, post1.no, false).await.unwrap());
    }
    #[test]
    fn test_job_update(){
        run_async(job_insert_update());
    }
    async fn job_insert_update() {
        let dbc = DBClient::new().await;
        let mut img = ImageInfo::default();
        img.board = "test_a".to_string();
        img.no = 777;
        img.url = "url1".to_string();
        img.page = 5;
        let img_job = dbc.insert_image_job(&img).await.unwrap();
        img.url = "url2".to_string();
        img.page = 6;
        assert_eq!(img_job.id, dbc.insert_image_job(&img).await.unwrap().id);

        let mut img_b = ImageInfo::default();
        img_b.board = "test_b".to_string();
        img_b.no = 777;
        img_b.url = "urlB".to_string();
        img_b.page = 77;
        let img_b_job = dbc.insert_image_job(&img_b).await.unwrap();

        assert_eq!(false, img_job.id == img_b_job.id);

        assert_eq!("url1".to_string(), dbc.get_image_job(img_job.id).await.unwrap().unwrap().url);
        assert_eq!(6, dbc.get_image_job(img_job.id).await.unwrap().unwrap().page);
        assert_eq!("urlB".to_string(), dbc.get_image_job(img_b_job.id).await.unwrap().unwrap().url);
        assert_eq!(77, dbc.get_image_job(img_b_job.id).await.unwrap().unwrap().page);

        assert_eq!(1u64, dbc.delete_image_job(img_b_job.id).await.unwrap());
        assert_eq!(1u64, dbc.delete_image_job(img_job.id).await.unwrap());
        assert_eq!(None, dbc.get_image_job(img_job.id).await.unwrap());
        assert_eq!(None, dbc.get_image_job(img_b_job.id).await.unwrap());
    }
    
    #[test]
    fn test_many_post_update(){
        run_async(many_post_update());
    }
    async fn many_post_update(){
        let dbc = DBClient::new().await;
        let mut post = dbc.get_post(&"vip".to_string(), 103205, false).await.unwrap().unwrap();
        post.board = "test".to_string();
        for i in 0..100000 {
            post.last_modified = i;
            dbc.insert_posts(&vec![post.clone()]).await.unwrap();
        }
    }

    #[test]
    fn test_count(){
        run_async(get_image_backlog_count());
    }
    async fn get_image_backlog_count(){
        let dbc = DBClient::new().await;
        println!("{}", dbc.get_image_backlog_size(2).await.unwrap());
    }
}