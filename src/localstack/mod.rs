pub use pro::LocalStackPro;
use testcontainers::{core::WaitFor, Image};

/// LocalStack Pro
pub mod pro;

const NAME: &str = "localstack/localstack";
const TAG: &str = "4.5";
const DEFAULT_WAIT: u64 = 3000;

/// This module provides [LocalStack](https://www.localstack.cloud/) (Community Edition).
///
/// Currently pinned to [version `4.5`](https://hub.docker.com/layers/localstack/localstack/4.5/images/sha256-acc5bf76bd8542897e6326c82f737a980791b998e4d641bcd1560902938ac305?context=explore)
///
/// # Configuration
///
/// For configuration, LocalStack uses environment variables. You can go [here](https://docs.localstack.cloud/references/configuration/)
/// for the full list.
///
/// Testcontainers support setting environment variables with the method
/// `RunnableImage::with_env_var((impl Into<String>, impl Into<String>))`. You will have to convert
/// the Image into a RunnableImage first.
///
/// ```
/// use testcontainers_modules::{localstack::LocalStack, testcontainers::ImageExt};
///
/// let container_request = LocalStack::default().with_env_var("SERVICES", "s3");
/// ```
///
/// No environment variables are required.
#[derive(Default, Debug, Clone)]
pub struct LocalStack {
    /// (remove if there is another variable)
    /// Field is included to prevent this struct to be a unit struct.
    /// This allows extending functionality (and thus further variables) without breaking changes
    _priv: (),
}

impl Image for LocalStack {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![
            WaitFor::message_on_stdout("Ready."),
            WaitFor::millis(DEFAULT_WAIT),
        ]
    }
}

#[cfg(test)]
mod tests {
    use aws_config::{meta::region::RegionProviderChain, BehaviorVersion};
    use aws_sdk_sqs as sqs;
    use testcontainers::runners::AsyncRunner;

    use super::LocalStack;

    #[tokio::test]
    #[allow(clippy::result_large_err)]
    async fn create_and_list_queue() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let node = LocalStack::default().start().await?;
        let host_ip = node.get_host().await?;
        let host_port = node.get_host_port_ipv4(4566).await?;

        let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
        let creds = sqs::config::Credentials::new("fake", "fake", None, None, "test");
        let config = aws_config::defaults(BehaviorVersion::v2025_01_17())
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
