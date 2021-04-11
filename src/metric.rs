use metrics::{register_histogram, register_counter, register_gauge, gauge, Unit};
use metrics_exporter_prometheus::{PrometheusBuilder, Matcher};
use log::info;

pub fn init_metrics(){
    let port = std::env::var("PROMETHEUS_PORT").unwrap_or("9000".to_string());
    let ip = std::env::var("PROMETHEUS_IP").unwrap_or("127.0.0.1".to_string());
    let address = format!("{}:{}", ip, port);
    info!("Prometheus metrics export: {}", address);
    let socket: std::net::SocketAddr = address.parse().unwrap();
    let builder = PrometheusBuilder::new();
    builder
    .set_buckets_for_metric(Matcher::Prefix("http_size".to_string()), 
    &[64f64, 256f64, 1024f64, 4096f64, 16384f64, 65536f64, 262144f64, 1048576f64, 2097152f64, 
    4194304f64, 6291456f64, 8388608f64, 10485760f64, 12582910f64, 16777216f64, 33554430f64, 67108860f64])
    .set_buckets_for_metric(Matcher::Full("http_request_duration".to_string()),
    &[30f64, 40f64, 50f64, 60f64, 70f64, 80f64, 100f64, 120f64, 150f64, 
    200f64, 250f64, 300f64, 500f64, 700f64, 800f64, 1000f64, 1250f64, 1500f64, 
    1750f64, 2000f64, 2500f64, 3000f64, 4000f64, 5000f64,
    10000f64, 15000f64, 30000f64, 45000f64, 60000f64])
    .set_buckets_for_metric(Matcher::Full("boards_scan_duration".to_string()),
    &[50f64, 100f64, 200f64, 500f64, 800f64, 1000f64, 1500f64, 2000f64, 2500f64, 3000f64, 5000f64,
    10000f64, 12000f64, 15000f64, 20000f64, 30000f64])
    .set_buckets_for_metric(Matcher::Suffix("job_duration".to_string()), 
    &[50f64, 100f64, 200f64, 500f64, 800f64, 1000f64, 1250f64, 1500f64, 
    1750f64, 2000f64, 2500f64, 3000f64, 4000f64, 5000f64, 
    10000f64, 15000f64, 30000f64, 45000f64, 60000f64, 120000f64, 180000f64, 240000f64, 
    600000f64, 1200000f64])
    .set_buckets_for_metric(Matcher::Suffix("batch_duration".to_string()), 
    &[100f64, 500f64, 800f64, 1000f64, 1500f64, 2000f64, 3000f64, 5000f64, 
    10000f64, 15000f64, 30000f64, 45000f64, 60000f64, 120000f64, 180000f64, 240000f64, 
    600000f64, 1200000f64])
    .listen_address(socket)
    .install().expect("Failed to install Prometheus recorder");

    register_metrics();
    reset_metrics();
}

fn register_metrics() {
    register_histogram!("boards_scan_duration", Unit::Milliseconds, "Time to completion for a scan of all boards");
    register_histogram!("thread_job_duration", Unit::Milliseconds, "Time to completion for thread jobs");
    register_histogram!("thread_batch_duration", Unit::Milliseconds, "Time to completion for a batch of thread jobs");
    register_histogram!("file_job_duration", Unit::Milliseconds, "Time to completion for file jobs");
    register_histogram!("file_batch_duration", Unit::Milliseconds, "Time to completion for a batch of file jobs");
    register_histogram!("http_request_duration", Unit::Milliseconds, "Time to completion for each http request");
    register_histogram!("http_size_file", Unit::Bytes, "File sizes");
    register_histogram!("http_size_thumbnail", Unit::Bytes, "Thumbnail sizes");
    register_counter!("files_fetched", "Total number of files fetched");
    register_counter!("thumbnails_fetched", "Total number of thumbnails fetched");
    register_counter!("file_jobs_scheduled", "Total number of file jobs that were scheduled");
    register_counter!("threads_fetched", "Number of times threads were fetched");
    register_counter!("thread_404", "Total number of http 404 threads");
    register_counter!("http_404", "Total number of http 404 errors");
    register_counter!("http_warn", "Total number of http non-404 error codes");
    register_counter!("bytes_fetched", Unit::Bytes, "Total number of bytes fetched");
    register_counter!("post_writes", "Posts inserted or updated to database");
    register_gauge!("thread_jobs_running", "Number of thread jobs running at any given time");
    register_gauge!("file_jobs_running", "Number of file jobs running at any given time");
    register_gauge!("http_requests_running", "Number of http requests currently being executed");
}

fn reset_metrics() {
    gauge!("thread_jobs_running", 0.0);
    gauge!("file_jobs_running", 0.0);
    gauge!("http_requests_running", 0.0);
}