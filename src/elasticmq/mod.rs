use testcontainers::{core::WaitFor, Image};

const NAME: &str = "softwaremill/elasticmq";
const TAG: &str = "1.5.2";

#[derive(Debug, Default, Clone)]
pub struct ElasticMq {
    /// (remove if there is another variable)
    /// Field is included to prevent this struct to be a unit struct.
    /// This allows extending functionality (and thus further variables) without breaking changes
    _priv: (),
}

impl Image for ElasticMq {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Started SQS rest server")]
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Display;

    use aws_config::{meta::region::RegionProviderChain, BehaviorVersion};
    use aws_sdk_sqs::{config::Credentials, Client};

    use crate::{elasticmq::ElasticMq, testcontainers::runners::AsyncRunner};

    #[tokio::test]
    async fn sqs_list_queues() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let node = ElasticMq::default().start().await?;
        let host_ip = node.get_host().await?;
        let host_port = node.get_host_port_ipv4(9324).await?;
        let client = build_sqs_client(host_ip, host_port).await;

        let result = client.list_queues().send().await.unwrap();
        // list should be empty
        assert!(result.queue_urls.filter(|urls| !urls.is_empty()).is_none());
        Ok(())
    }

    async fn build_sqs_client(host_ip: impl Display, host_port: u16) -> Client {
        let endpoint_uri = format!("http://{host_ip}:{host_port}");
        let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
        let creds = Credentials::new("fakeKey", "fakeSecret", None, None, "test");

        let shared_config = aws_config::defaults(BehaviorVersion::latest())
            .region(region_provider)
            .endpoint_url(endpoint_uri)
            .credentials_provider(creds)
            .load()
            .await;

        Client::new(&shared_config)
    }
}
