use std::collections::HashSet;
use std::sync::Arc;
use anyhow::Ok;
#[allow(unused_imports)]
use log::{info, warn, error, debug};

use argon2::{
	password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
	Argon2,
};

use dashmap::DashSet;

mod board_archiver;
mod image_archiver;
mod thread_archiver;
mod archiver_metrics;

use crate::{http::HttpClient, models::{User, UserRole}};
use crate::models::{Board, BoardsList, PurgeReport};
use crate::db::DBClient;

#[derive(Clone)]
pub struct Archiver {
    pub http_client: HttpClient,
    pub db_client: DBClient,
    pub archived_ids: Arc<DashSet<u64>>
}


impl Archiver {
    pub async fn new(client: HttpClient) -> Self {
        Self {
            http_client: client,
            db_client: DBClient::new().await,
            archived_ids: Arc::new(DashSet::new())
        }
    }
    pub fn run_archivers(&self) -> tokio::task::JoinHandle<()> {
        self.run_metrics_cycle();
        self.run_board_cycle();
        self.run_thread_cycle();
        self.run_image_cycle()
    }
    pub async fn get_all_boards_api(&self) -> Result<BoardsList, bool> {
        self.http_client.fetch_json::<BoardsList>("https://a.4cdn.org/boards.json").await
    }
    pub async fn get_boards_set(&self) -> anyhow::Result<HashSet<String>> {
        let boardslist = self.get_all_boards_api().await.map_err(|_| anyhow::anyhow!(""))?;
        let mut name_set = HashSet::new();
        for board in boardslist.boards {
            name_set.insert(board.board);
        }
        Ok(name_set)
    }
    pub async fn set_board(&self, board: Board) -> anyhow::Result<Option<Board>> {
        if !self.get_boards_set().await?.contains(&board.name) {
            error!("Board /{}/ does not exist, skipping", board.name);
            return Ok(None)
        }
        // If full_images is being set to true, check if the board is already in the database
        if board.full_images {
            if let Some(b) = self.db_client.get_board(&board.name).await? {
                if !b.full_images {
                    // The board is being enabled for full images, but it's already in the database with full_images = false
                    let result = self.db_client.insert_board(&board).await?;
                    // We need to make sure existing posts have their full images downloaded
                    let jobs_scheduled = self.db_client.schedule_missing_full_files(&board.name).await?;
                    if jobs_scheduled > 0 {
                        info!("Scheduled {} missing full images for board /{}/", jobs_scheduled, board.name);
                    }
                    return Ok(Some(result))
                }
            }
        }
        Ok(Some(self.db_client.insert_board(&board).await?))
    }
    pub async fn stop_board(&self, board_name: &String) -> anyhow::Result<Option<Board>> {
        if let Some(mut board) = self.db_client.get_board(board_name).await? {
            board.archive = false;
            return Ok(Some(self.db_client.insert_board(&board).await?));
        }
        Ok(None)
    }
    pub async fn get_all_boards(&self) -> anyhow::Result<Vec<Board>> {
        self.db_client.get_all_boards().await
    }

    pub async fn is_board_enabled(&self, board_name: &String) -> anyhow::Result<bool> {
        if let Some(board) = self.db_client.get_board(board_name).await? {
            Ok(board.archive)
        } else {
            Ok(false)
        }
    }

    pub async fn purge_board(&self, board_name: &String, only_full_images: bool) -> anyhow::Result<PurgeReport> {
        let mut report = PurgeReport::default();
        if only_full_images {
            // If we only want to delete full images, we only disable full image downloads for the board
            if let Some(mut board) = self.db_client.get_board(board_name).await? {
                board.full_images = false;
                self.db_client.insert_board(&board).await?;
            }
            // If the board is not in the database, no need to stop full image downloads
        } else {
            // If we want to delete everything, we stop the board archiver entirely
            self.stop_board(board_name).await?;
            // Try to avoid more images being downloaded while we're purging the board
            self.db_client.purge_board_backlogs(board_name).await?;
        }
        
        let full_files = self.db_client.get_files_exclusive_to_board(board_name).await?;
        info!("Purging {} full files", full_files.len());
        for file in &full_files {
            // Double check in case the file was added by another board while we were iterating
            if self.db_client.is_file_on_other_boards(&file.file_sha256, &file.ext, &board_name).await? {
                info!("Skipping full file {}{} which was found on another board", file.file_sha256, file.ext);
                continue;
            }
            if self.http_client.delete_downloaded_file(&file.file_sha256, &file.ext, false).await.is_ok() {
                report.full_files_deleted += 1;
                self.db_client.set_file_purged(&file.file_sha256, &file.ext).await?;
                info!("Deleted full file {}{}", file.file_sha256, file.ext)
            } else {
                report.full_files_failed += 1;
            }
        }

        if !only_full_images {
            let thumbnail_hashes = self.db_client.get_thumbnails_exclusive_to_board(board_name).await?;
            info!("Purging {} thumbnails", thumbnail_hashes.len());
            for hash in &thumbnail_hashes {
                // Double check in case the thumbnail was added by another board while we were iterating
                if self.db_client.is_thumbnail_on_other_boards(hash, &".jpg".to_string(), &board_name).await? {
                    info!("Skipping thumbnail {} which was found on another board", hash);
                    continue;
                }
                if self.http_client.delete_downloaded_file(hash, &".jpg".to_string(), true).await.is_ok() {
                    report.thumbnails_deleted += 1;
                    self.db_client.set_thumbnail_purged(hash, &".jpg".to_string()).await?;
                    info!("Deleted thumbnail {}", hash);
                } else {
                    report.thumbnails_failed += 1;
                }
            }
            let removed_posts = self.db_client.purge_board_data(board_name).await?;
            info!("Purged {} posts", removed_posts);
            report.removed_posts = removed_posts;
        }
        Ok(report)
    }

    pub async fn hide_post(&self, board_name: &String, no: i64, hide_comment: bool, hide_image: bool) -> anyhow::Result<()> {
        // Only hide the whole post if neither the comment nor the image are hidden
        let post_hidden = !hide_comment && !hide_image;
        self.db_client.set_post_hidden_status(&board_name, no, post_hidden, hide_comment, hide_image).await?;
        Ok(())
    }

    pub async fn unhide_post(&self, board_name: &String, no: i64) -> anyhow::Result<()> {
        self.db_client.set_post_hidden_status(&board_name, no, false, false, false).await?;
        Ok(())
    }

    pub async fn purge_image(&self, board_name: &String, no: i64, reason: &String) -> anyhow::Result<Vec<String>> {
        let mut purged_files = Vec::new();
        let post = self.db_client.get_post(board_name, no, false).await?;
        if let Some(post) = post {
            if !post.thumbnail_sha256.is_empty() {
                self.http_client.delete_downloaded_file(&post.thumbnail_sha256, &".jpg".to_string(), true).await?;
                purged_files.push(post.thumbnail_sha256.clone());
            }
            if !post.file_sha256.is_empty() {
                self.http_client.delete_downloaded_file(&post.file_sha256, &post.ext, false).await?;
                purged_files.push(post.file_sha256.clone());
            }
        } else {
            warn!("Post /{}/{} not found.", board_name, no);
        }
        for sha256 in &purged_files {
            self.db_client.blacklist_file(&sha256, &reason).await?;
        }
        Ok(purged_files)
    }

    pub async fn unpurge_image(&self, board_name: &String, no: i64) -> anyhow::Result<Vec<String>> {
        let mut purged_files = Vec::new();
        let post = self.db_client.get_post(board_name, no, false).await?;
        if let Some(post) = post {
            if !post.thumbnail_sha256.is_empty() {
                self.db_client.remove_file_blacklist(&post.thumbnail_sha256).await?;
                purged_files.push(post.thumbnail_sha256.clone());
            }
            if !post.file_sha256.is_empty() {
                self.db_client.remove_file_blacklist(&post.file_sha256).await?;
                purged_files.push(post.thumbnail_sha256.clone());
            }
        } else {
            warn!("Post /{}/{} not found.", board_name, no);
        }
        Ok(purged_files)
    }

    pub async fn add_user(&self, username: &String, password: &String, role: UserRole) -> anyhow::Result<()> {
        if self.db_client.get_user(username).await?.is_some() {
            return Err(anyhow::anyhow!("User already exists"));
        }
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(&password.as_bytes(), &salt)
            .map_err(|_| anyhow::Error::msg("Couldn't hash"))?.to_string();

        let user = User {
            name: username.clone(),
            password_hash: password_hash.clone(),
            role: role.clone()
        };
        self.db_client.insert_user(&user).await?;
        Ok(())
    }

    pub async fn delete_user(&self, username: &String) -> anyhow::Result<()> {
        self.db_client.delete_user(username).await?;
        Ok(())
    }

    pub async fn login(&self, username: &String, password: &String) -> anyhow::Result<Option<User>> {
        let user = self.db_client.get_user(username).await?;
        if let Some(user) = user {
            let parsed_hash = PasswordHash::new(&user.password_hash)
                .map_err(|_| anyhow::Error::msg("Couldn't parse hash"))?;
            if Argon2::default()
                .verify_password(
                    password.as_bytes(),
                    &parsed_hash
                ).is_ok() {
                Ok(Some(user))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    pub async fn change_password(&self, username: &String, password: &String) -> anyhow::Result<()> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(&password.as_bytes(), &salt)
            .map_err(|_| anyhow::Error::msg("Couldn't hash"))?.to_string();
        self.db_client.change_password(username, &password_hash).await?;
        Ok(())
    }

    pub async fn change_role(&self, username: &String, role: UserRole) -> anyhow::Result<()> {
        self.db_client.change_role(username, role).await?;
        Ok(())
    }

    pub async fn ensure_admin_exists(&self, password: &String) -> anyhow::Result<()> {
        if self.db_client.get_user(&"admin".to_string()).await?.is_none() {
            self.add_user(&"admin".to_string(), password, UserRole::Admin).await?;
        } else {
            // Change password if admin exists
            self.change_password(&"admin".to_string(), password).await?;
            // Ensure the role is admin
            self.change_role(&"admin".to_string(), UserRole::Admin).await?;
        }
        Ok(())
    }
}

impl std::panic::UnwindSafe for Archiver {}
impl std::panic::RefUnwindSafe for Archiver {}