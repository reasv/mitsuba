use std::path::Path;
use std::num::NonZeroU32;

use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use governor::{Quota, RateLimiter, Jitter, state::{keyed::DashMapStateStore}, clock::QuantaClock};
use nonzero_ext::nonzero;
use backoff::{default, ExponentialBackoff};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::time::Duration;
use log::{info, warn, error, debug};
use tokio::fs::create_dir_all;

use crate::util::{hash_file, get_file_folder, get_file_url};
use crate::object_storage::ObjectStorage;

async fn write_bytes_to_file(filename: &Path, file_bytes: bytes::Bytes) -> anyhow::Result<()> {
    Ok(File::create(filename).await?.write_all(&file_bytes).await?)
}

pub struct HttpClient {
    limiter: RateLimiter<String, DashMapStateStore<String>, QuantaClock>,
    jitter: Jitter,
    max_time: u64,
    rclient: reqwest::Client,
    oclient: ObjectStorage,
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new(nonzero!(60u32),nonzero!(10u32), 200, 800, 600)
    }
}

impl HttpClient {
    pub fn new(quota: NonZeroU32, burst: NonZeroU32, jitter_min: u64, jitter_interval: u64, max_time: u64) -> HttpClient {
        HttpClient {
            limiter: RateLimiter::dashmap(Quota::per_minute(quota).allow_burst(burst)),
            jitter: Jitter::new(Duration::from_millis(jitter_min), Duration::from_millis(jitter_interval)),
            max_time,
            rclient: reqwest::Client::new(),
            oclient: ObjectStorage::new(),
        }
    }

    fn new_backoff(&self) -> ExponentialBackoff {
        ExponentialBackoff{
            current_interval: Duration::from_millis(default::INITIAL_INTERVAL_MILLIS),
            initial_interval: Duration::from_millis(default::INITIAL_INTERVAL_MILLIS),
            randomization_factor: default::RANDOMIZATION_FACTOR,
            multiplier: default::MULTIPLIER,
            max_interval: Duration::from_millis(default::MAX_INTERVAL_MILLIS),
            max_elapsed_time: Some(Duration::from_secs(self.max_time)),
            clock: backoff::SystemClock::default(),
            start_time: instant::Instant::now(),
        }
    }

    async fn fetch_url_bytes(&self, url: &str, attempt: u64, rlimit_key: &String) -> Result<bytes::Bytes, backoff::Error<reqwest::Error>> {
        self.limiter.until_key_ready_with_jitter(rlimit_key, self.jitter).await; // wait for rate limiter
        let resp = self.rclient.get(url).send().await.map_err(backoff::Error::Transient)?;
        info!("Fetching: {} (Attempt {})", url, attempt);
        match resp.status() {
            StatusCode::OK => Ok(resp.bytes().await.map_err(backoff::Error::Transient)?),
            StatusCode::NOT_FOUND => {
                error!("Error fetching {} (Status: 404)", url);
                Err(backoff::Error::Permanent(resp.error_for_status().unwrap_err()))
            },
            _ => {
                warn!("Retry fetching {} after bad status code (Status: {})", url, resp.status());
                Err(backoff::Error::Transient(resp.error_for_status().unwrap_err()))
            }
        }
    }

    pub async fn fetch_url_backoff(&self, url: &str, rlimit_key: &String) -> Result<bytes::Bytes, reqwest::Error> {
        let back = self.new_backoff();
        let mut attempt: u64 = 0;
        backoff::future::retry(back, || {
            debug!("Scheduling: {} (Attempt {})", url, attempt);
            attempt += 1;
            self.fetch_url_bytes(url, attempt, rlimit_key)
        }).await
    }

    // Returns Err(true) if error was 404 (non recoverable)
    pub async fn fetch_json<T: DeserializeOwned>(&self, url: &str) -> Result<T, bool> {
        let bytes = self.fetch_url_backoff(url, &"api".to_string()).await
        .map_err(|e| e.status().unwrap_or(StatusCode::OK) == StatusCode::NOT_FOUND)?;
        
        let obj = serde_json::from_slice(&bytes)
        .map_err(|e| {error!("Failed to deserialize {} Error: {}", url, e); false})?;
        Ok(obj)
    }

    pub async fn _download_file(&self, url: &String, filename: &Path) -> bool {
        let bytes = match self.fetch_url_backoff(url, &"download".to_string()).await {
            Ok(b) => b,
            Err(msg) => {
                error!("Failed to download {} Error: {}", url, msg);
                return false
            }
        };
        match write_bytes_to_file(filename, bytes).await {
            Ok(()) => return true,
            Err(msg) => {
                error!("Could not write to file {}: {}", filename.to_str().unwrap_or_default(), msg);
                return false
            }
        }
    }
    async fn save_file(&self, bytes: bytes::Bytes, ext: &String, is_thumb: bool) -> Option<String> {
        let hash = hash_file(&bytes);
        let folder = get_file_folder(&hash, is_thumb);
        create_dir_all(&folder).await.ok();
        let filename = folder.join(hash.clone() + ext);
        match write_bytes_to_file(&filename, bytes).await {
            Ok(()) => Some(hash),
            Err(msg) => {
                error!("Could not write to file {}: {}", filename.to_str().unwrap_or_default(), msg);
                None
            }
        }
    }
    async fn upload_file(&self, bytes: bytes::Bytes, ext: &String, is_thumb: bool) -> Option<String> {
        let hash = hash_file(&bytes);
        let filename = get_file_url(&hash, &ext, is_thumb);
        info!("Uploading: {}", filename);
        if let Some((_, code)) = self.oclient.bucket.put_object(filename.clone(), &bytes).await
        .map_err(|e| {error!("Error uploading file ({}) to object storage: {}", filename, e);})
        .ok() {
            if code == 200 {
                return Some(hash);
            }
            error!("Error response code from object storage after upload request ({}): {}", filename, code);
        }
        None
    }
    pub async fn download_file_checksum(&self, url: &String, ext: &String, is_thumb: bool) -> Option<String> {
        let bytes = match self.fetch_url_backoff(url, &"download".to_string()).await {
            Ok(b) => b,
            Err(msg) => {
                error!("Failed to download {} Error: {}", url, msg);
                return None
            }
        };
        if self.oclient.enabled {
            self.upload_file(bytes, ext, is_thumb).await
        } else {
            self.save_file(bytes, ext, is_thumb).await
        }
    }
}