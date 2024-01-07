use testcontainers::{core::WaitFor, Image};

const NAME: &str = "localstack/localstack";
const TAG: &str = "3.0.2";
const DEFAULT_WAIT: u64 = 3000;

#[derive(Default, Debug)]
pub struct LocalStack;

impl Image for LocalStack {
    type Args = ();

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![
            WaitFor::message_on_stdout("Ready."),
            WaitFor::millis(DEFAULT_WAIT)
        ]
    }

    // TODO Env vars
}

#[cfg(test)]
mod tests {
    use aws_config::BehaviorVersion;
    use testcontainers::clients;
    use aws_sdk_sqs as sqs;
    use super::LocalStack;

    #[tokio::test]
    async fn create_and_list_queue() -> Result<(), sqs::Error> {
        let docker = clients::Cli::default();
        let node = docker.run(LocalStack);
        let host_port = node.get_host_port_ipv4(4566);

        let config = aws_config::defaults(BehaviorVersion::v2023_11_09())
            .endpoint_url(format!("http://localhost:{}", host_port))
            .load()
            .await;
        let client = sqs::Client::new(&config);

        client.create_queue()
            .queue_name("example-queue")
            .send().await?;

        let list_result = client.list_queues().send().await?;
        assert_eq!(list_result.queue_urls().len(), 1);

        Ok(())
    }
}