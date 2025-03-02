use std::{borrow::Cow, collections::HashMap};

use testcontainers::{core::WaitFor, Image};

const NAME: &str = "minio/minio";
const TAG: &str = "RELEASE.2025-02-28T09-55-16Z";

const DIR: &str = "/data";
const CONSOLE_ADDRESS: &str = ":9001";

#[allow(missing_docs)]
// not having docs here is currently allowed to address the missing docs problem one place at a time. Helping us by documenting just one of these places helps other devs tremendously
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

#[allow(missing_docs)]
// not having docs here is currently allowed to address the missing docs problem one place at a time. Helping us by documenting just one of these places helps other devs tremendously
#[derive(Debug, Clone)]
pub struct MinIOServerCmd {
    #[allow(missing_docs)]
    // not having docs here is currently allowed to address the missing docs problem one place at a time. Helping us by documenting just one of these places helps other devs tremendously
    pub dir: String,
    #[allow(missing_docs)]
    // not having docs here is currently allowed to address the missing docs problem one place at a time. Helping us by documenting just one of these places helps other devs tremendously
    pub certs_dir: Option<String>,
    #[allow(missing_docs)]
    // not having docs here is currently allowed to address the missing docs problem one place at a time. Helping us by documenting just one of these places helps other devs tremendously
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
