use metrics::{gauge, Unit, describe_counter, describe_histogram, describe_gauge};
use metrics_exporter_prometheus::{PrometheusBuilder, Matcher};
use log::info;

// So many unwraps just to get code working...
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
    4194304f64, 6291456f64, 8388608f64, 10485760f64, 12582910f64, 16777216f64, 33554430f64, 67108860f64]).unwrap()
    .set_buckets_for_metric(Matcher::Full("http_request_duration".to_string()),
    &[30f64, 40f64, 50f64, 60f64, 70f64, 80f64, 100f64, 120f64, 150f64, 
    200f64, 250f64, 300f64, 500f64, 700f64, 800f64, 1000f64, 1250f64, 1500f64, 
    1750f64, 2000f64, 2500f64, 3000f64, 4000f64, 5000f64,
    10000f64, 15000f64, 30000f64, 45000f64, 60000f64]).unwrap()
    .set_buckets_for_metric(Matcher::Full("boards_scan_duration".to_string()),
    &[50f64, 100f64, 200f64, 500f64, 800f64, 1000f64, 1500f64, 2000f64, 2500f64, 3000f64, 5000f64,
    10000f64, 12000f64, 15000f64, 20000f64, 30000f64]).unwrap()
    .set_buckets_for_metric(Matcher::Suffix("job_duration".to_string()), 
    &[50f64, 100f64, 200f64, 500f64, 800f64, 1000f64, 1250f64, 1500f64, 
    1750f64, 2000f64, 2500f64, 3000f64, 4000f64, 5000f64, 
    10000f64, 15000f64, 30000f64, 45000f64, 60000f64, 120000f64, 180000f64, 240000f64, 
    600000f64, 1200000f64]).unwrap()
    .with_http_listener(socket)
    .install().expect("Failed to install Prometheus recorder");

    register_metrics();
    reset_metrics();
}

fn register_metrics() {
    describe_counter!("boards_scan_duration", Unit::Milliseconds, "Time to completion for a scan of all boards");
    describe_histogram!("thread_job_duration", Unit::Milliseconds, "Time to completion for thread jobs");
    describe_histogram!("file_job_duration", Unit::Milliseconds, "Time to completion for file jobs");
    describe_histogram!("http_request_duration", Unit::Milliseconds, "Time to completion for each http request");
    describe_histogram!("http_size_file", Unit::Bytes, "File sizes");
    describe_histogram!("http_size_thumbnail", Unit::Bytes, "Thumbnail sizes");
    describe_counter!("thread_archived_jobs_scheduled", "Total number of archived threads scheduled for retrieval");
    describe_counter!("files_fetched", "Total number of files fetched");
    describe_counter!("thumbnails_fetched", "Total number of thumbnails fetched");
    describe_counter!("file_jobs_scheduled", "Total number of file jobs that were scheduled");
    describe_counter!("threads_fetched", "Number of times threads were fetched");
    describe_counter!("thread_404", "Total number of http 404 threads");
    describe_counter!("http_404", "Total number of http 404 errors");
    describe_counter!("http_warn", "Total number of http non-404 error codes");
    describe_counter!("bytes_fetched", Unit::Bytes, "Total number of bytes fetched");
    describe_counter!("post_writes", "Posts inserted or updated to database");
    describe_counter!("thread_job_writes", "Thread jobs inserted or updated to database");
    describe_counter!("post_deleted", "Posts that we detected as having been deleted");
    describe_gauge!("files_stored", "Stored files");
    describe_gauge!("thumbnails_stored", "Stored thumbnails");
    describe_gauge!("thumbnails_missing", "Missing thumbnails");
    describe_gauge!("thread_jobs_running", "Number of thread jobs running at any given time");
    describe_gauge!("file_jobs_running", "Number of file jobs running at any given time");
    describe_gauge!("http_requests_running", "Number of http requests currently being executed");
    describe_gauge!("thread_archived_hashes", "Number of known thread id hashes of archived threads");
    describe_gauge!("thread_jobs_hashes", "Number of known thread job hashes of previously added jobs");
    describe_gauge!("post_hashes", "Number of known post hashes of previously added or updated posts");
    describe_gauge!("file_backlog_size", "Current size of the file backlog");
    describe_gauge!("thread_backlog_size", "Current size of the thread backlog");
    describe_gauge!("file_backlog_size_live", "Current size of the file backlog, counting only non-archive data");
    describe_gauge!("thread_backlog_size_live", "Current size of the thread backlog, counting only non-archive data");
    describe_histogram!("metric_scan_duration", Unit::Milliseconds, "Time to completion for metric cycle");
}
//        .set_buckets_for_metric(Matcher::Full("api_http_requests_duration_seconds".to_string()),&([0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,]).build();
fn reset_metrics() {
    gauge!("thread_jobs_running", 0.0);
    gauge!("file_jobs_running", 0.0);
    gauge!("http_requests_running", 0.0);
}