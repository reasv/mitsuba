use std::path::Path;
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use governor::{Quota, RateLimiter, Jitter, state::keyed::DashMapStateStore, clock::QuantaClock};
use nonzero_ext::nonzero;
use backoff::{default, ExponentialBackoff};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use log::{info, warn, error, debug};
#[allow(unused_imports)]
use metrics::{gauge, increment_gauge, decrement_gauge, counter, histogram};

use tokio::fs::create_dir_all;
use weighted_rs::Weight;

use crate::util::{hash_file, get_file_folder, get_file_url, get_proxy_config, get_host_string};
use crate::object_storage::ObjectStorage;

async fn write_bytes_to_file(filename: &Path, file_bytes: bytes::Bytes) -> anyhow::Result<()> {
    Ok(File::create(filename).await?.write_all(&file_bytes).await?)
}

#[derive(Clone)]
pub struct HttpClient {
    limiter: Arc<RateLimiter<String, DashMapStateStore<String>, QuantaClock>>,
    jitter: Arc<Jitter>,
    max_time: u64,
    rclient: reqwest::Client,
    oclient: Arc<ObjectStorage>,
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new(nonzero!(60u32),nonzero!(10u32), 200, 800, 600)
    }
}

impl HttpClient {
    pub fn new(quota: NonZeroU32, burst: NonZeroU32, jitter_min: u64, jitter_interval: u64, max_time: u64) -> HttpClient {
        let proxy_balancer = Arc::new(Mutex::new(get_proxy_config()));
        let rclient = reqwest::Client::builder()
        .proxy(reqwest::Proxy::custom(move |url| {
            debug!("Proxy call");
            // Unwrap here is safe, because .next() can not panic.
            if let Some(proxy_url_opt) = proxy_balancer.lock().unwrap().next() {
                debug!("Used proxy {:?} for {}{}", get_host_string(&proxy_url_opt), url, url.path());
                proxy_url_opt.clone()
            } else {
                debug!("No proxy for {}{}", url, url.path());
                None
            }
        }))
        .build().unwrap();

        HttpClient {
            limiter:  Arc::new(RateLimiter::dashmap(Quota::per_minute(quota).allow_burst(burst))),
            jitter:  Arc::new(Jitter::new(Duration::from_millis(jitter_min), Duration::from_millis(jitter_interval))),
            max_time,
            rclient,
            oclient:  Arc::new(ObjectStorage::new()),
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
        self.limiter.until_key_ready_with_jitter(rlimit_key, *self.jitter).await; // wait for rate limiter
        increment_gauge!("http_requests_running", 1.0);
        let s = Instant::now();
        let resp = self.rclient.get(url).send().await.map_err(backoff::Error::transient)?;
        histogram!("http_request_duration", s.elapsed().as_millis() as f64);
        decrement_gauge!("http_requests_running", 1.0);
        
        

        info!("Fetching: {} (Attempt {})", url, attempt);
        match resp.status() {
            StatusCode::OK => Ok(resp.bytes().await.map_err(backoff::Error::transient)?),
            StatusCode::NOT_FOUND => {
                error!("Error fetching {} (Status: 404)", url);
                counter!("http_404", 1);
                Err(backoff::Error::Permanent(resp.error_for_status().unwrap_err()))
            },
            _ => {
                warn!("Retry fetching {} after bad status code (Status: {})", url, resp.status());
                counter!("http_warn", 1);
                Err(backoff::Error::transient(resp.error_for_status().unwrap_err()))
            }
        }
    }

    pub async fn fetch_url_backoff(&self, url: &str, rlimit_key: &String) -> Result<bytes::Bytes, reqwest::Error> {
        let back = self.new_backoff();
        let mut attempt: u64 = 0;
        let bytes = backoff::future::retry(back, || {
            attempt += 1;
            debug!("Scheduling: {} (Attempt {})", url, attempt);
            self.fetch_url_bytes(url, attempt, rlimit_key)
        }).await?;
        counter!("bytes_fetched", bytes.len() as u64);
        Ok(bytes)
    }

    // Returns Err(true) if error was 404 (non recoverable)
    pub async fn fetch_json<T: DeserializeOwned>(&self, url: &str) -> Result<T, bool> {
        let bytes = self.fetch_url_backoff(url, &"api".to_string()).await
        .map_err(|e| e.status().unwrap_or(StatusCode::OK) == StatusCode::NOT_FOUND)?;
        
        let obj = serde_json::from_slice(&bytes)
        .map_err(|e| {error!("Failed to deserialize {} Error: {}", url, e); false})?;
        Ok(obj)
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
        if let Some(response_data) = self.oclient.bucket.put_object(filename.clone(), &bytes).await
        .map_err(|e| {error!("Error uploading file ({}) to object storage: {}", filename, e);})
        .ok() {
            let code = response_data.status_code();
            if code == 200 {
                return Some(hash);
            }
            error!("Error response code from object storage after upload request ({}): {}", filename, code);
        }
        None
    }
    pub async fn download_file_checksum(&self, url: &String, ext: &String, is_thumb: bool) -> Result<String, ()> {
        let bytes = match self.fetch_url_backoff(url, &"download".to_string()).await {
            Ok(b) => b,
            Err(err) => {
                error!("Failed to download {} Error: {}", url, err);
                if let Some(status) = err.status() {
                    if status == StatusCode::NOT_FOUND {
                        return Ok("".to_string());
                    }
                }
                return Err(())
            }
        };
        if is_thumb {
            histogram!("http_size_thumbnail", bytes.len() as f64);
        } else {
            histogram!("http_size_file", bytes.len() as f64);
        }
        if self.oclient.enabled {
            self.upload_file(bytes, ext, is_thumb).await.ok_or(())
        } else {
            self.save_file(bytes, ext, is_thumb).await.ok_or(())
        }
    }
}

impl std::panic::UnwindSafe for HttpClient {}
impl std::panic::RefUnwindSafe for HttpClient {}