use std::borrow::Cow;

use testcontainers::{core::WaitFor, Image};

const NAME: &str = "parity/parity";
const TAG: &str = "v2.5.0";

/// Module to work with [`Parity Ethereum`] inside of tests.
///
/// Starts an instance of Parity Ethereum based on the official [`Parity docker image`].
///
/// Parity Ethereum is a fast and feature-rich multi-network Ethereum client.
/// This module provides a local Ethereum node for testing purposes with JSON-RPC API enabled.
/// The container exposes port `8545` for HTTP JSON-RPC connections by default.
///
/// The node is configured with development settings including:
/// - Development chain configuration
/// - All JSON-RPC APIs enabled
/// - Unsafe expose mode (for testing)
/// - Tracing enabled
///
/// # Example
/// ```
/// use testcontainers_modules::{
///     parity_parity::ParityEthereum, testcontainers::runners::SyncRunner,
/// };
///
/// let ethereum_instance = ParityEthereum::default().start().unwrap();
/// let host = ethereum_instance.get_host().unwrap();
/// let port = ethereum_instance.get_host_port_ipv4(8545).unwrap();
///
/// // Use the JSON-RPC endpoint at http://{host}:{port}
/// ```
///
/// [`Parity Ethereum`]: https://www.parity.io/
/// [`Parity docker image`]: https://hub.docker.com/r/parity/parity
#[derive(Debug, Default, Clone)]
pub struct ParityEthereum {
    /// (remove if there is another variable)
    /// Field is included to prevent this struct to be a unit struct.
    /// This allows extending functionality (and thus further variables) without breaking changes
    _priv: (),
}

impl Image for ParityEthereum {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stderr("Public node URL:")]
    }

    fn cmd(&self) -> impl IntoIterator<Item = impl Into<Cow<'_, str>>> {
        [
            "--config=dev",
            "--jsonrpc-apis=all",
            "--unsafe-expose",
            "--tracing=on",
        ]
    }
}

#[cfg(test)]
mod tests {
    use testcontainers::runners::SyncRunner;

    use crate::parity_parity;

    #[test]
    fn parity_parity_net_version() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let _ = pretty_env_logger::try_init();
        let node = parity_parity::ParityEthereum::default().start()?;
        let host_ip = node.get_host()?;
        let host_port = node.get_host_port_ipv4(8545)?;

        let response = reqwest::blocking::Client::new()
            .post(format!("http://{host_ip}:{host_port}"))
            .body(
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "net_version",
                    "params": [],
                    "id": 1
                })
                .to_string(),
            )
            .header("content-type", "application/json")
            .send()
            .unwrap();

        let response = response.text().unwrap();
        let response: serde_json::Value = serde_json::from_str(&response).unwrap();

        assert_eq!(response["result"], "17");
        Ok(())
    }
}
