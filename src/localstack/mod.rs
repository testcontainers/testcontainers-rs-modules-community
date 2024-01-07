use testcontainers::{core::WaitFor, Image};
use std::collections::HashMap;

const NAME: &str = "localstack/localstack";
const TAG: &str = "3.0";
const DEFAULT_WAIT: u64 = 3000;

/// This module provides [LocalStack](https://www.localstack.cloud/) (Community Edition).
/// 
/// Currently pinned to [version `3.0`](https://hub.docker.com/layers/localstack/localstack/3.0/images/sha256-73698e485240939490134aadd7e429ac87ff068cd5ad09f5de8ccb76727c13e1?context=explore)
#[derive(Default, Debug)]
pub struct LocalStack {
    env_vars: HashMap<String, String>
}

impl LocalStack {
    pub fn with_environment_variable(
        self,
        var_name: impl Into<String>,
        value: impl Into<String>
    ) -> Self {
        let mut env_vars = self.env_vars;
        env_vars.insert(var_name.into(), value.into());
        Self { env_vars }
    }
}

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

    fn env_vars(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(self.env_vars.iter())
    }
}

#[cfg(test)]
mod tests {
    use aws_config::BehaviorVersion;
    use testcontainers::{clients, Image};
    use aws_sdk_sqs as sqs;
    use super::LocalStack;

    #[tokio::test]
    async fn create_and_list_queue() -> Result<(), sqs::Error> {
        let docker = clients::Cli::default();
        let node = docker.run(LocalStack::default());
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

    #[tokio::test]
    async fn set_env_variables() {
        let localstack = LocalStack::default()
            .with_environment_variable("DEBUG", "1")
            .with_environment_variable("USE_SSL", "1");
        let env_vars: Vec<_> = localstack.env_vars().as_mut()
            .map(|(k, v)| {
                (k.to_owned(), v.to_owned())
            })
            .collect();
        assert_eq!(env_vars.len(), 2);
        assert!(env_vars.contains(&("DEBUG".to_owned(), "1".to_owned())));
        assert!(env_vars.contains(&("USE_SSL".to_owned(), "1".to_owned())));
    }
}