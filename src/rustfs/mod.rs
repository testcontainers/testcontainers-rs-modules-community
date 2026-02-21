use std::{borrow::Cow, collections::HashMap};

use testcontainers::{core::WaitFor, Image};

const NAME: &str = "rustfs/rustfs";
const TAG: &str = "latest";

const API_PORT: u16 = 9000;

/// Module to work with [`RustFS`] inside of tests.
///
/// Starts an instance of RustFS based on the official [`RustFS docker image`].
///
/// RustFS is a high-performance object storage server compatible with Amazon S3 APIs.
/// The container exposes port `9000` for API access and port `9001` for the web console by default.
///
/// # Example
/// ```
/// use testcontainers_modules::{rustfs::RustFS, testcontainers::runners::SyncRunner};
///
/// let rustfs_instance = RustFS::default().start().unwrap();
/// let host = rustfs_instance.get_host().unwrap();
/// let api_port = rustfs_instance.get_host_port_ipv4(9000).unwrap();
///
/// // Use the S3-compatible API at http://{host}:{api_port}
/// ```
///
/// [`RustFS`]: https://rustfs.com/
/// [`RustFS docker image`]: https://hub.docker.com/r/rustfs/rustfs
#[derive(Debug, Clone)]
pub struct RustFS {
    env_vars: HashMap<String, String>,
}

impl Default for RustFS {
    fn default() -> Self {
        let mut env_vars = HashMap::new();
        env_vars.insert("RUSTFS_ADDRESS".to_owned(), format!(":{}", API_PORT));
        env_vars.insert("RUSTFS_CONSOLE_ENABLE".to_owned(), "true".to_owned());

        Self { env_vars }
    }
}

impl Image for RustFS {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("RustFS Http API:")]
    }

    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        &self.env_vars
    }

    fn cmd(&self) -> impl IntoIterator<Item = impl Into<Cow<'_, str>>> {
        ["/data"]
    }
}

#[cfg(test)]
mod tests {
    use aws_config::{meta::region::RegionProviderChain, BehaviorVersion};
    use aws_sdk_s3::{config::Credentials, Client};
    use testcontainers::runners::AsyncRunner;

    use crate::rustfs;

    #[tokio::test]
    async fn rustfs_buckets() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let rustfs = rustfs::RustFS::default();
        let node = rustfs.start().await?;

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

        let bucket_exists = buckets
            .iter()
            .any(|b| b.name.as_deref() == Some(bucket_name));
        assert!(bucket_exists, "Bucket {} not found", bucket_name);
        Ok(())
    }

    async fn build_s3_client(host_port: u16) -> Client {
        let endpoint_uri = format!("http://127.0.0.1:{host_port}");
        let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
        let creds = Credentials::new("rustfsadmin", "rustfsadmin", None, None, "test");

        let shared_config = aws_config::defaults(BehaviorVersion::latest())
            .region(region_provider)
            .endpoint_url(endpoint_uri)
            .credentials_provider(creds)
            .load()
            .await;

        Client::new(&shared_config)
    }
}
