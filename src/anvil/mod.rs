use std::borrow::Cow;

use testcontainers::{
    core::{ContainerPort, Mount, WaitFor},
    Image,
};

const NAME: &str = "ghcr.io/foundry-rs/foundry";
const TAG: &str = "v1.1.0";

/// Port that the [`AnvilNode`] container exposes for JSON-RPC connections.
/// Can be rebound externally via [`testcontainers::core::ImageExt::with_mapped_port`]
///
/// [`AnvilNode`]: https://book.getfoundry.sh/anvil/
pub const ANVIL_PORT: ContainerPort = ContainerPort::Tcp(8545);

/// # Community Testcontainers Implementation for [Foundry Anvil](https://book.getfoundry.sh/anvil/)
///
/// This is a community implementation of the [Testcontainers](https://testcontainers.org/) interface for [Foundry Anvil](https://book.getfoundry.sh/anvil/).
///
/// Anvil is Foundry's fast local Ethereum node for development and testing. It's an ideal tool for rapid
/// iteration on smart contract development and provides a clean, lightweight alternative to running a full node.
///
/// # Example
///
/// ```rust,no_run
/// use testcontainers_modules::{
///     anvil::{AnvilNode, ANVIL_PORT},
///     testcontainers::runners::AsyncRunner,
/// };
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Start an Anvil node
/// let node = AnvilNode::default().start().await?;
///
/// // Get the RPC endpoint URL
/// let host_port = node.get_host_port_ipv4(ANVIL_PORT).await?;
/// let rpc_url = format!("http://localhost:{host_port}");
///
/// // Use with your favorite Ethereum library (alloy, ethers, web3, etc.)
/// // let provider = Provider::try_from(rpc_url)?;
/// # Ok(())
/// # }
/// ```
///
/// # Advanced Configuration
///
/// ```rust,ignore
/// use testcontainers_modules::anvil::AnvilNode;
///
/// // Configure chain ID and forking
/// let node = AnvilNode::default()
///     .with_chain_id(1337)
///     .with_fork_url("https://eth.llamarpc.com")
///     .with_fork_block_number(18_000_000)
///     .start().await?;
/// ```
///
/// # Usage
///
/// The endpoint of the container is intended to be injected into your provider configuration, so that you can
/// easily run tests against a local Anvil instance.
///
/// To use the latest Foundry image, you can use the `latest()` method:
///
/// ```rust,ignore
/// let node = AnvilNode::latest().start().await?;
/// ```
///
/// Users can use a specific Foundry image in their code with [`ImageExt::with_tag`](https://docs.rs/testcontainers/latest/testcontainers/core/trait.ImageExt.html#tymethod.with_tag).
///
/// ```rust,ignore
/// use testcontainers::core::ImageExt;
/// let node = AnvilNode::default().with_tag("nightly").start().await?;
/// ```
#[derive(Debug, Clone, Default)]
pub struct AnvilNode {
    chain_id: Option<u64>,
    fork_url: Option<String>,
    fork_block_number: Option<u64>,
    tag: Option<String>,
    load_state_path: Option<String>,
    dump_state_path: Option<String>,
    state_interval_secs: Option<u64>,
    state_mount: Option<Mount>,
}

impl AnvilNode {
    /// Create a new AnvilNode with the latest Foundry image
    pub fn latest() -> Self {
        Self {
            tag: Some("latest".to_string()),
            ..Default::default()
        }
    }

    /// Use a specific Foundry image tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    /// Specify the chain ID - this will be Ethereum Mainnet by default
    pub fn with_chain_id(mut self, chain_id: u64) -> Self {
        self.chain_id = Some(chain_id);
        self
    }

    /// Specify the fork URL to fork from a live network
    ///
    /// # Example
    /// ```rust,ignore
    /// let node = AnvilNode::default()
    ///     .with_fork_url("https://eth.llamarpc.com")
    ///     .start().await?;
    /// ```
    pub fn with_fork_url(mut self, fork_url: impl Into<String>) -> Self {
        self.fork_url = Some(fork_url.into());
        self
    }

    /// Specify the fork block number (requires `with_fork_url` to be set)
    ///
    /// # Example
    /// ```rust,ignore
    /// let node = AnvilNode::default()
    ///     .with_fork_url("https://eth.llamarpc.com")
    ///     .with_fork_block_number(18_000_000)
    ///     .start().await?;
    /// ```
    pub fn with_fork_block_number(mut self, block_number: u64) -> Self {
        self.fork_block_number = Some(block_number);
        self
    }

    /// Mount a host directory for anvil state at `/state` inside the container
    ///
    /// Use this method to bind mount a host directory to the container's `/state` directory.
    /// This allows you to persist state across container restarts.
    ///
    /// # Arguments
    /// * `host_dir` - Path on the host machine to mount (will be mounted to `/state` in container)
    ///
    /// # Note
    /// When using this method, specify container paths (starting with `/state/`) in
    /// `with_load_state_path` and `with_dump_state_path`.
    ///
    /// # Example
    /// ```rust,ignore
    /// let temp_dir = std::env::temp_dir().join("anvil-state");
    /// let node = AnvilNode::default()
    ///     .with_state_mount(&temp_dir)
    ///     .with_dump_state_path("/state/state.json")
    ///     .start().await?;
    /// ```
    pub fn with_state_mount(mut self, host_dir: impl AsRef<std::path::Path>) -> Self {
        let Some(host_dir_str) = host_dir.as_ref().to_str() else {
            return self;
        };
        self.state_mount = Some(Mount::bind_mount(host_dir_str, "/state"));
        self
    }

    /// Configure Anvil to initialize from a previously saved state snapshot.
    /// Equivalent to passing `--load-state <PATH>`.
    ///
    /// # Arguments
    /// * `path` - Path to the state file (container-internal path, typically `/state/...` if using `with_state_mount`)
    ///
    /// # Example
    /// ```rust,ignore
    /// let node = AnvilNode::default()
    ///     .with_state_mount("/host/path/to/state")
    ///     .with_load_state_path("/state/state.json")
    ///     .start().await?;
    /// ```
    pub fn with_load_state_path(mut self, path: impl Into<String>) -> Self {
        self.load_state_path = Some(path.into());
        self
    }

    /// Configure Anvil to dump the state on exit to the given file or directory.
    /// Equivalent to passing `--dump-state <PATH>`.
    ///
    /// # Arguments
    /// * `path` - Path where state should be saved (container-internal path, typically `/state/...` if using `with_state_mount`)
    ///
    /// # Example
    /// ```rust,ignore
    /// let node = AnvilNode::default()
    ///     .with_state_mount("/host/path/to/state")
    ///     .with_dump_state_path("/state/state.json")
    ///     .start().await?;
    /// ```
    pub fn with_dump_state_path(mut self, path: impl Into<String>) -> Self {
        self.dump_state_path = Some(path.into());
        self
    }

    /// Configure periodic state persistence interval in seconds.
    /// Equivalent to passing `--state-interval <SECONDS>`.
    ///
    /// # Example
    /// ```rust,ignore
    /// let node = AnvilNode::default()
    ///     .with_state_mount("/host/path/to/state")
    ///     .with_dump_state_path("/state/state.json")
    ///     .with_state_interval(30) // Save every 30 seconds
    ///     .start().await?;
    /// ```
    pub fn with_state_interval(mut self, seconds: u64) -> Self {
        self.state_interval_secs = Some(seconds);
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

        if let Some(ref load_path) = self.load_state_path {
            cmd.push("--load-state".to_string());
            cmd.push(load_path.clone());
        }

        if let Some(ref dump_path) = self.dump_state_path {
            cmd.push("--dump-state".to_string());
            cmd.push(dump_path.clone());
        }

        if let Some(interval) = self.state_interval_secs {
            cmd.push("--state-interval".to_string());
            cmd.push(interval.to_string());
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

    fn mounts(&self) -> impl IntoIterator<Item = &Mount> {
        self.state_mount.iter()
    }

    fn expose_ports(&self) -> &[ContainerPort] {
        &[ANVIL_PORT]
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

    use super::*;

    #[tokio::test]
    async fn test_anvil_node_container() {
        use testcontainers::runners::AsyncRunner;

        let _ = pretty_env_logger::try_init();

        let node = AnvilNode::default().start().await.unwrap();
        let port = node.get_host_port_ipv4(ANVIL_PORT).await.unwrap();

        let provider: RootProvider<AnyNetwork> =
            RootProvider::new_http(format!("http://localhost:{port}").parse().unwrap());

        let block_number = provider.get_block_number().await.unwrap();

        assert_eq!(block_number, 0);
    }

    #[test]
    fn test_anvil_node_container_sync() {
        use testcontainers::runners::SyncRunner;

        let _ = pretty_env_logger::try_init();

        let node = AnvilNode::default().start().unwrap();
        let port = node.get_host_port_ipv4(ANVIL_PORT).unwrap();

        let provider: RootProvider<AnyNetwork> =
            RootProvider::new_http(format!("http://localhost:{port}").parse().unwrap());

        let block_number = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(provider.get_block_number())
            .unwrap();

        assert_eq!(block_number, 0);
    }

    #[tokio::test]
    async fn test_anvil_latest() {
        use testcontainers::runners::AsyncRunner;

        let _ = pretty_env_logger::try_init();

        let node = AnvilNode::latest().start().await.unwrap();
        let port = node.get_host_port_ipv4(ANVIL_PORT).await.unwrap();

        let provider: RootProvider<AnyNetwork> =
            RootProvider::new_http(format!("http://localhost:{port}").parse().unwrap());

        let block_number = provider.get_block_number().await.unwrap();

        assert_eq!(block_number, 0);
    }

    #[test]
    fn test_command_construction() {
        let node = AnvilNode::default()
            .with_chain_id(1337)
            .with_fork_url("http://example.com")
            .with_load_state_path("/state/state.json")
            .with_dump_state_path("/state/state.json")
            .with_state_interval(5);

        let cmd: Vec<String> = node
            .cmd()
            .into_iter()
            .map(|c| c.into().into_owned())
            .collect();

        assert_eq!(
            cmd,
            vec![
                "--chain-id",
                "1337",
                "--fork-url",
                "http://example.com",
                "--load-state",
                "/state/state.json",
                "--dump-state",
                "/state/state.json",
                "--state-interval",
                "5",
            ]
        );

        assert_eq!(node.entrypoint(), Some("anvil"));
    }
}
