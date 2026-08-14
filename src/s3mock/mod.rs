use testcontainers::{
    core::{wait::HttpWaitStrategy, ContainerPort, WaitFor},
    Image,
};

const NAME: &str = "adobe/s3mock";
const TAG: &str = "5.1.0";

/// Port that [`S3Mock`] uses internally for its HTTP S3 API.
pub const S3MOCK_PORT: ContainerPort = ContainerPort::Tcp(9090);

/// Module to work with [`S3Mock`] inside tests.
///
/// Starts an instance of S3Mock based on the official [S3Mock Docker image].
/// S3Mock is an S3-compatible object storage mock intended for integration tests.
///
/// # Example
/// ```
/// use testcontainers_modules::{
///     s3mock::{S3Mock, S3MOCK_PORT},
///     testcontainers::runners::AsyncRunner,
/// };
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let s3mock = S3Mock::default().start().await?;
/// let port = s3mock.get_host_port_ipv4(S3MOCK_PORT).await?;
///
/// // Use the S3-compatible API at http://127.0.0.1:{port}
/// # Ok(())
/// # }
/// ```
///
/// [S3Mock Docker image]: https://hub.docker.com/r/adobe/s3mock
#[derive(Debug, Default, Clone)]
pub struct S3Mock {
    _priv: (),
}

impl Image for S3Mock {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::http(
            HttpWaitStrategy::new("/favicon.ico")
                .with_port(S3MOCK_PORT)
                .with_expected_status_code(200_u16),
        )]
    }

    fn expose_ports(&self) -> &[ContainerPort] {
        &[S3MOCK_PORT]
    }
}

#[cfg(test)]
mod tests {
    use aws_config::{meta::region::RegionProviderChain, BehaviorVersion};
    use aws_sdk_s3::{config::Credentials, primitives::ByteStream, Client};
    use testcontainers::runners::AsyncRunner;

    use crate::s3mock::{S3Mock, S3MOCK_PORT};

    #[tokio::test]
    async fn supports_bucket_and_object_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
        let node = S3Mock::default().start().await?;
        let host_port = node.get_host_port_ipv4(S3MOCK_PORT).await?;
        let client = build_s3_client(host_port).await;

        let bucket = "test-bucket";
        let key = "test-object";
        let content = b"s3mock content";

        client.create_bucket().bucket(bucket).send().await?;
        client
            .put_object()
            .bucket(bucket)
            .key(key)
            .body(ByteStream::from_static(content))
            .send()
            .await?;

        let object = client
            .get_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await?
            .body
            .collect()
            .await?
            .into_bytes();
        assert_eq!(object.as_ref(), content);

        let objects = client.list_objects_v2().bucket(bucket).send().await?;
        assert_eq!(objects.contents().len(), 1);
        assert_eq!(objects.contents()[0].key(), Some(key));

        client
            .delete_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await?;
        let objects = client.list_objects_v2().bucket(bucket).send().await?;
        assert!(objects.contents().is_empty());

        client.delete_bucket().bucket(bucket).send().await?;

        Ok(())
    }

    async fn build_s3_client(host_port: u16) -> Client {
        let endpoint_uri = format!("http://127.0.0.1:{host_port}");
        let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
        let credentials = Credentials::new("test", "test", None, None, "test");
        let shared_config = aws_config::defaults(BehaviorVersion::latest())
            .region(region_provider)
            .endpoint_url(endpoint_uri)
            .credentials_provider(credentials)
            .load()
            .await;
        let s3_config = aws_sdk_s3::config::Builder::from(&shared_config)
            .force_path_style(true)
            .build();

        Client::from_conf(s3_config)
    }
}
