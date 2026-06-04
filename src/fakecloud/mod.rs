use testcontainers::{core::WaitFor, Image};

/// Container port for the fakecloud HTTP API.
pub const FAKECLOUD_PORT: u16 = 4566;

const NAME: &str = "ghcr.io/faiscadev/fakecloud";
const TAG: &str = "0.4.0";

/// [fakecloud](https://fakecloud.dev) is a free, open-source local AWS cloud emulator.
///
/// Supports S3, SQS, SNS, EventBridge, IAM/STS, SSM, DynamoDB, Lambda,
/// Secrets Manager, CloudWatch Logs, KMS, SES, and CloudFormation.
///
/// Currently pinned to [version `0.4.0`](https://github.com/faiscadev/fakecloud/releases/tag/v0.4.0).
///
/// # Configuration
///
/// fakecloud uses environment variables for configuration. See the
/// [documentation](https://fakecloud.dev) for the full list.
///
/// ```
/// use testcontainers_modules::{fakecloud::FakeCloud, testcontainers::ImageExt};
///
/// let container_request = FakeCloud::default().with_env_var("FAKECLOUD_LOG", "debug");
/// ```
///
/// No environment variables are required.
#[derive(Default, Debug, Clone)]
pub struct FakeCloud {
    _priv: (),
}

impl Image for FakeCloud {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("fakecloud is ready")]
    }
}

#[cfg(test)]
mod tests {
    use aws_config::{meta::region::RegionProviderChain, BehaviorVersion};
    use aws_sdk_sqs as sqs;
    use testcontainers::runners::AsyncRunner;

    use super::FakeCloud;

    #[tokio::test]
    #[allow(clippy::result_large_err)]
    async fn create_and_list_queue() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let node = FakeCloud::default().start().await?;
        let host_ip = node.get_host().await?;
        let host_port = node.get_host_port_ipv4(4566).await?;

        let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
        let creds = sqs::config::Credentials::new("test", "test", None, None, "test");
        let config = aws_config::defaults(BehaviorVersion::v2025_08_07())
            .region(region_provider)
            .credentials_provider(creds)
            .endpoint_url(format!("http://{host_ip}:{host_port}"))
            .load()
            .await;
        let client = sqs::Client::new(&config);

        client
            .create_queue()
            .queue_name("example-queue")
            .send()
            .await?;

        let list_result = client.list_queues().send().await?;
        assert_eq!(list_result.queue_urls().len(), 1);

        Ok(())
    }
}
