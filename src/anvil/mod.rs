use std::borrow::Cow;

use testcontainers::{
    core::{ContainerPort, WaitFor},
    Image,
};

const NAME: &str = "ghcr.io/foundry-rs/foundry";
const TAG: &str = "stable@sha256:daeeaaf4383ee0cbfc9f31f079a04ffb0123e49e5f67f2a20b5ce1ac1959a4d6";
const PORT: ContainerPort = ContainerPort::Tcp(8545);

/// # Community Testcontainers Implementation for [Foundry Anvil](https://book.getfoundry.sh/anvil/)
///
/// This is a community implementation of the [Testcontainers](https://testcontainers.org/) interface for [Foundry Anvil](https://book.getfoundry.sh/anvil/).
///
/// It is not officially supported by Foundry, but it is a community effort to provide a more user-friendly interface for running Anvil inside a Docker container.
///
/// The endpoint of the container is intended to be injected into your provider configuration, so that you can easily run tests against a local Anvil instance.
/// See the `test_anvil_node_container` test for an example of how to use this.
///
/// To use the latest Foundry image, you can use the `latest()` method:
///
/// ```rust
/// let node = AnvilNode::latest().start().await?;
/// ```
///
/// Users can use a specific Foundry image in their code with [`ImageExt::with_tag`](https://docs.rs/testcontainers/0.23.1/testcontainers/core/trait.ImageExt.html#tymethod.with_tag).
///
/// ```rust
/// let node = AnvilNode::with_tag("master").start().await?;
/// ```
#[derive(Debug, Clone, Default)]
pub struct AnvilNode {
    chain_id: Option<u64>,
    fork_url: Option<String>,
    fork_block_number: Option<u64>,
    tag: Option<String>,
}

impl AnvilNode {
    /// Create a new AnvilNode with the latest Foundry image
    pub fn latest() -> Self {
        Self {
            tag: Some("latest".to_string()),
            ..Default::default()
        }
    }

    /// Specify the chain ID - this will be Ethereum Mainnet by default
    pub fn with_chain_id(mut self, chain_id: u64) -> Self {
        self.chain_id = Some(chain_id);
        self
    }

    /// Specify the fork URL
    pub fn with_fork_url(mut self, fork_url: impl Into<String>) -> Self {
        self.fork_url = Some(fork_url.into());
        self
    }

    /// Specify the fork block number
    pub fn with_fork_block_number(mut self, block_number: u64) -> Self {
        self.fork_block_number = Some(block_number);
        self
    }
}

impl Image for AnvilNode {
    fn cmd(&self) -> impl IntoIterator<Item = impl Into<Cow<'_, str>>> {
        let mut cmd = vec![];

        if let Some(chain_id) = self.chain_id {
            cmd.push("--chain-id".to_string());
            cmd.push(chain_id.to_string());
        }

        if let Some(ref fork_url) = self.fork_url {
            cmd.push("--fork-url".to_string());
            cmd.push(fork_url.to_string());
        }

        if let Some(fork_block_number) = self.fork_block_number {
            cmd.push("--fork-block-number".to_string());
            cmd.push(fork_block_number.to_string());
        }

        cmd.into_iter().map(Cow::from)
    }

    fn entrypoint(&self) -> Option<&str> {
        Some("anvil")
    }

    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        [("ANVIL_IP_ADDR".to_string(), "0.0.0.0".to_string())].into_iter()
    }

    fn expose_ports(&self) -> &[ContainerPort] {
        &[PORT]
    }

    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        self.tag.as_deref().unwrap_or(TAG)
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Listening on 0.0.0.0:8545")]
    }
}

#[cfg(test)]
mod tests {
    use alloy_network::AnyNetwork;
    use alloy_provider::{Provider, RootProvider};
    use alloy_transport_http::Http;
    use testcontainers::runners::AsyncRunner;

    use super::*;

    #[tokio::test]
    async fn test_anvil_node_container() {
        let _ = pretty_env_logger::try_init();

        let node = AnvilNode::default().start().await.unwrap();
        let port = node.get_host_port_ipv4(PORT).await.unwrap();

        let provider: RootProvider<Http<_>, AnyNetwork> =
            RootProvider::new_http(format!("http://localhost:{port}").parse().unwrap());

        let block_number = provider.get_block_number().await.unwrap();

        assert_eq!(block_number, 0);
    }

    #[test]
    fn test_command_construction() {
        let node = AnvilNode::default()
            .with_chain_id(1337)
            .with_fork_url("http://example.com");

        let cmd: Vec<String> = node
            .cmd()
            .into_iter()
            .map(|c| c.into().into_owned())
            .collect();

        assert_eq!(
            cmd,
            vec!["--chain-id", "1337", "--fork-url", "http://example.com"]
        );

        assert_eq!(node.entrypoint(), Some("anvil"));
    }
}
