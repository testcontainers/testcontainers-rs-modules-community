use std::{borrow::Cow, collections::HashMap, time::Duration};

use testcontainers::{
    core::{wait::HttpWaitStrategy, ContainerPort, WaitFor},
    Image,
};

const NAME: &str = "semitechnologies/weaviate";

const TAG: &str = "1.28.2";

const HTTP_PORT: u16 = 8080;
const GRPC_PORT: u16 = 50051;

const PORTS: [ContainerPort; 2] = [ContainerPort::Tcp(HTTP_PORT), ContainerPort::Tcp(GRPC_PORT)];

/// Module to work with [`Weaviate`] inside of tests.
///
/// Starts an instance of Weaviate based on the official
/// [Docker image](https://hub.docker.com/r/semitechnologies/weaviate)
#[derive(Default)]
pub struct Weaviate {
    env_vars: HashMap<String, String>,
    tag: Option<String>,
}

impl Weaviate {
    /// Configure an environment variable.
    ///
    /// See https://weaviate.io/developers/weaviate/config-refs/env-vars for
    /// a complete overview.
    pub fn with_env_var(mut self, key: &str, value: &str) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }

    /// Set the image tag to be used.
    pub fn with_tag(mut self, tag: &str) {
        self.tag = Some(tag.to_string())
    }
}

impl Image for Weaviate {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        self.tag.as_deref().unwrap_or(TAG)
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::http(
            HttpWaitStrategy::new("/")
                .with_poll_interval(Duration::from_millis(100))
                .with_response_matcher(|resp| resp.status().is_success()),
        )]
    }

    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        &self.env_vars
    }

    fn expose_ports(&self) -> &[ContainerPort] {
        &PORTS
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use reqwest::blocking::Client;

    use super::*;
    use crate::testcontainers::runners::SyncRunner;

    #[test]
    fn test_connect_simple() -> Result<(), Box<dyn Error>> {
        let container = Weaviate::default().start()?;
        let client = Client::new();

        let host = container.get_host()?.to_string();
        let port = container.get_host_port_ipv4(8080)?;
        let base_url = format!("http://{host}:{port}");

        let response = client.get(&base_url).send()?;

        assert!(response.status().is_success());

        let schema_url = format!("{base_url}/v1/schema");
        let schema_response: String = client.get(&schema_url).send()?.text()?;

        assert_eq!(&schema_response, "{\"classes\":[]}\n");

        Ok(())
    }
}
