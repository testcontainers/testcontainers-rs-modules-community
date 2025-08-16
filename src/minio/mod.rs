use std::{borrow::Cow, collections::HashMap};

use testcontainers::{core::WaitFor, Image};

const NAME: &str = "minio/minio";
const TAG: &str = "RELEASE.2025-02-28T09-55-16Z";

const DIR: &str = "/data";
const CONSOLE_ADDRESS: &str = ":9001";

/// Module to work with [`MinIO`] inside of tests.
///
/// Starts an instance of MinIO based on the official [`MinIO docker image`].
///
/// MinIO is a high-performance object storage server compatible with Amazon S3 APIs.
/// The container exposes port `9000` for API access and port `9001` for the web console by default.
///
/// # Example
/// ```
/// use testcontainers_modules::{minio::MinIO, testcontainers::runners::SyncRunner};
///
/// let minio_instance = MinIO::default().start().unwrap();
/// let host = minio_instance.get_host().unwrap();
/// let api_port = minio_instance.get_host_port_ipv4(9000).unwrap();
/// let console_port = minio_instance.get_host_port_ipv4(9001).unwrap();
///
/// // Use the S3-compatible API at http://{host}:{api_port}
/// // Access the web console at http://{host}:{console_port}
/// ```
///
/// [`MinIO`]: https://min.io/
/// [`MinIO docker image`]: https://hub.docker.com/r/minio/minio
#[derive(Debug, Clone)]
pub struct MinIO {
    env_vars: HashMap<String, String>,
    cmd: MinIOServerCmd,
}

impl Default for MinIO {
    fn default() -> Self {
        let mut env_vars = HashMap::new();
        env_vars.insert(
            "MINIO_CONSOLE_ADDRESS".to_owned(),
            CONSOLE_ADDRESS.to_owned(),
        );

        Self {
            env_vars,
            cmd: MinIOServerCmd::default(),
        }
    }
}

/// Configuration for MinIO server command-line arguments.
///
/// This struct allows you to customize the MinIO server startup configuration
/// by setting various options like the data directory, TLS certificates, and logging format.
#[derive(Debug, Clone)]
pub struct MinIOServerCmd {
    /// The directory where MinIO will store data.
    /// Defaults to "/data" if not specified.
    pub dir: String,
    /// Optional directory containing TLS certificates for HTTPS.
    /// If provided, MinIO will enable TLS/SSL.
    pub certs_dir: Option<String>,
    /// Whether to enable JSON formatted logging.
    /// When true, MinIO outputs logs in JSON format for easier parsing.
    pub json_log: bool,
}

impl Default for MinIOServerCmd {
    fn default() -> Self {
        Self {
            dir: DIR.to_owned(),
            certs_dir: None,
            json_log: false,
        }
    }
}

impl IntoIterator for &MinIOServerCmd {
    type Item = String;
    type IntoIter = <Vec<String> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        let mut args = vec!["server".to_owned(), self.dir.to_owned()];

        if let Some(ref certs_dir) = self.certs_dir {
            args.push("--certs-dir".to_owned());
            args.push(certs_dir.to_owned())
        }

        if self.json_log {
            args.push("--json".to_owned());
        }

        args.into_iter()
    }
}

impl Image for MinIO {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stderr("API:")]
    }

    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        &self.env_vars
    }

    fn cmd(&self) -> impl IntoIterator<Item = impl Into<Cow<'_, str>>> {
        &self.cmd
    }
}

#[cfg(test)]
mod tests {
    use aws_config::{meta::region::RegionProviderChain, BehaviorVersion};
    use aws_sdk_s3::{config::Credentials, Client};
    use testcontainers::runners::AsyncRunner;

    use crate::minio;

    #[tokio::test]
    async fn minio_buckets() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let minio = minio::MinIO::default();
        let node = minio.start().await?;

        let host_port = node.get_host_port_ipv4(9000).await?;
        let client = build_s3_client(host_port).await;

        let bucket_name = "test-bucket";

        client
            .create_bucket()
            .bucket(bucket_name)
            .send()
            .await
            .expect("Failed to create test bucket");

        let buckets = client
            .list_buckets()
            .send()
            .await
            .expect("Failed to get list of buckets")
            .buckets
            .unwrap();
        assert_eq!(1, buckets.len());
        assert_eq!(bucket_name, buckets[0].name.as_ref().unwrap());
        Ok(())
    }

    async fn build_s3_client(host_port: u16) -> Client {
        let endpoint_uri = format!("http://127.0.0.1:{host_port}");
        let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
        let creds = Credentials::new("minioadmin", "minioadmin", None, None, "test");

        // Default MinIO credentials (Can be overridden by ENV container variables)
        let shared_config = aws_config::defaults(BehaviorVersion::latest())
            .region(region_provider)
            .endpoint_url(endpoint_uri)
            .credentials_provider(creds)
            .load()
            .await;

        Client::new(&shared_config)
    }
}
