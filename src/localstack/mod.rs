use testcontainers::{core::WaitFor, Image};

const NAME: &str = "localstack/localstack";
const TAG: &str = "3.0";
const DEFAULT_WAIT: u64 = 3000;

/// This module provides [LocalStack](https://www.localstack.cloud/) (Community Edition).
///
/// Currently pinned to [version `3.0`](https://hub.docker.com/layers/localstack/localstack/3.0/images/sha256-73698e485240939490134aadd7e429ac87ff068cd5ad09f5de8ccb76727c13e1?context=explore)
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
#[derive(Default, Debug)]
pub struct LocalStack {
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
        let config = aws_config::defaults(BehaviorVersion::v2024_03_28())
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
