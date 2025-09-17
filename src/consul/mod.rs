use std::{borrow::Cow, collections::BTreeMap};

use testcontainers::{core::WaitFor, Image};

const DEFAULT_IMAGE_NAME: &str = "hashicorp/consul";
const DEFAULT_IMAGE_TAG: &str = "1.16.1";
const CONSUL_LOCAL_CONFIG: &str = "CONSUL_LOCAL_CONFIG";

/// Module to work with [`Consul`] inside of tests.
///
/// This module is based on the official [`Consul docker image`].
///
/// # Example
/// ```
/// use testcontainers_modules::{consul, testcontainers::runners::SyncRunner};
///
/// let consul = consul::Consul::default().start().unwrap();
/// let http_port = consul.get_host_port_ipv4(8500).unwrap();
///
/// // do something with the started consul instance..
/// ```
///
/// [`Consul`]: https://www.consul.io/
/// [`Consul docker image`]: https://hub.docker.com/r/hashicorp/consul
#[derive(Debug, Default, Clone)]
pub struct Consul {
    env_vars: BTreeMap<String, String>,
}

impl Consul {
    /// Passes a JSON string of configuration options to the Consul agent
    ///
    /// For details on which options are avalible, please see [the consul docs](https://developer.hashicorp.com/consul/docs/reference/agent/configuration-file).
    ///
    /// # Example
    /// ```
    /// use testcontainers_modules::{consul, testcontainers::runners::SyncRunner};
    ///
    /// let consul = consul::Consul::default()
    ///     .with_local_config(
    ///         r#"{
    ///         "datacenter": "us_west",
    ///         "server": true,
    ///         "enable_debug": true
    ///     }"#,
    ///     )
    ///     .start()
    ///     .unwrap();
    /// ```
    pub fn with_local_config(self, config: impl ToString) -> Self {
        let mut env_vars = self.env_vars;
        env_vars.insert(CONSUL_LOCAL_CONFIG.to_owned(), config.to_string());
        Self { env_vars }
    }
}

impl Image for Consul {
    fn name(&self) -> &str {
        DEFAULT_IMAGE_NAME
    }

    fn tag(&self) -> &str {
        DEFAULT_IMAGE_TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("agent: Consul agent running!")]
    }

    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        &self.env_vars
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use crate::{consul::Consul, testcontainers::runners::AsyncRunner};

    #[tokio::test]
    async fn consul_container() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let consul = Consul::default().with_local_config("{\"datacenter\":\"dc-rust\"}".to_owned());
        let node = consul.start().await?;
        let port = node.get_host_port_ipv4(8500).await?;

        let response = reqwest::Client::new()
            .get(format!("http://localhost:{}/v1/agent/self", port))
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap();
        let config = response.as_object().unwrap().get("Config").unwrap();
        let dc = config
            .as_object()
            .unwrap()
            .get("Datacenter")
            .unwrap()
            .as_str()
            .unwrap();
        assert_eq!("dc-rust", dc);
        Ok(())
    }
}
