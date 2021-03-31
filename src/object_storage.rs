use s3::bucket::Bucket;
use s3::creds::Credentials;
use s3::region::Region;
use crate::util::bool_from_env;

pub struct ObjectStorage {
    pub region: Region,
    pub credentials: Credentials,
    pub bucket_name: String,
    pub bucket: Bucket,
    pub enabled: bool
}
impl ObjectStorage {
    pub fn new() -> Self {
        if bool_from_env(&"ENABLE_S3_STORAGE".to_string()) {
            Self::new_env()
        } else {
            // if not enabled we give the dummy version
            Self::default()
        }
    }
    pub fn new_env() -> Self {
        let credentials = Credentials::from_env_specific(
            Some("S3_ACCESS_KEY_ID"),
            Some("S3_SECRET_ACCESS_KEY"),
            None,
            None,
        ).unwrap();
        let bucket_name = std::env::var("S3_BUCKET").unwrap().to_string();
        let region = Region::Custom {
            region: std::env::var("S3_REGION").unwrap().into(),
            endpoint: std::env::var("S3_ENDPOINT").unwrap().into(),
        };
        let mut bucket = Bucket::new_with_path_style(&bucket_name, region.clone(), credentials.clone()).unwrap();
        bucket.add_header("x-amz-acl", "public-read");
        bucket.add_header("Content-Disposition", "inline");
        Self {
            credentials,
            bucket_name,
            region,
            bucket,
            enabled: true
        }
    }
}
impl Default for ObjectStorage {
    fn default() -> Self {
        Self {
            // dummy values
            credentials: Credentials::anonymous().unwrap(),
            bucket_name: "bucket".to_string(),
            region: "us-east-1".parse().unwrap(),
            bucket: Bucket::new(&"bucket".to_string(), "us-east-1".parse().unwrap(), Credentials::anonymous().unwrap()).unwrap(),
            enabled: false
        }
    }
}