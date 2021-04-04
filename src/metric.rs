use metrics::{register_histogram, register_counter, register_gauge, gauge, Unit};
use metrics_exporter_prometheus::{PrometheusBuilder, Matcher};


pub fn init_metrics(){
    let builder = PrometheusBuilder::new();
    builder
    .set_buckets_for_metric(Matcher::Prefix("http_size".to_string()), 
    &[64f64, 256f64, 1024f64, 4096f64, 16384f64, 65536f64, 262144f64, 1048576f64, 2097152f64, 4194304f64, 16777216f64, 67108864f64])
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
    register_counter!("file_jobs_completed", "Total number of file jobs that were completed");
    register_counter!("threads_fetched", "Number of times threads were fetched");
    register_counter!("thread_jobs_completed", "Total number of thread jobs that were completed");
    register_counter!("thread_404", "Total number of http 404 threads");

    register_counter!("http_requests", "Total number of http requests started");
    register_counter!("http_404", "Total number of http 404 errors");
    register_counter!("http_warn", "Total number of http non-404 error codes");
    register_counter!("bytes_fetched", Unit::Bytes, "Total number of bytes fetched");
    register_gauge!("thread_jobs_running", "Number of thread jobs running at any given time");
    register_gauge!("file_jobs_running", "Number of file jobs running at any given time");
    register_gauge!("http_requests_running", "Number of http requests currently being executed");
}

fn reset_metrics() {
    gauge!("thread_jobs_running", 0.0);
    gauge!("file_jobs_running", 0.0);
    gauge!("http_requests_running", 0.0);
}