use std::borrow::Cow;

use testcontainers::{
    core::{ContainerPort, WaitFor},
    Image,
};

const NAME: &str = "trufflesuite/ganache-cli";
const TAG: &str = "v6.1.3";

/// Port that the [`Ganache CLI`] container has internally.
/// Can be rebound externally via [`testcontainers::core::ImageExt::with_mapped_port`]
///
/// [Ganache CLI]: https://github.com/trufflesuite/ganache
pub const GANACHE_CLI_PORT: ContainerPort = ContainerPort::Tcp(8545);

/// # Module to work with the [`Ganache CLI`] inside of tests.
///
/// Starts an instance of Meilisearch.
/// This module is based on the official [`trufflesuite/ganache-cli` docker image] documented in the [documentation].
///
/// # Example
/// ```
/// use testcontainers_modules::{testcontainers::runners::SyncRunner, trufflesuite_ganachecli};
///
/// let instance = trufflesuite_ganachecli::GanacheCli::default()
///     .start()
///     .unwrap();
/// let url = format!(
///     "http://{host_ip}:{host_port}",
///     host_ip = instance.get_host().unwrap(),
///     host_port = instance.get_host_port_ipv4(GANACHE_CLI_PORT).unwrap()
/// );
/// // do something with the started GanacheCli instance..
/// ```
///
/// [Ganache CLI]: https://github.com/trufflesuite/ganache
/// [documentation]: https://github.com/trufflesuite/ganache?tab=readme-ov-file#documentation
/// [`trufflesuite/ganache-cli` docker image]: https://hub.docker.com/r/trufflesuite/ganache-cli/
#[derive(Debug, Default, Clone)]
pub struct GanacheCli {
    cmd: GanacheCliCmd,
}

/// Options to pass to the `ganache-cli` command
#[derive(Debug, Clone)]
pub struct GanacheCliCmd {
    /// Specify the network id ganache-core will use to identify itself (defaults to the current time or the network id of the forked blockchain if configured)
    pub network_id: u32,
    /// Specify the number of accounts to generate at startup
    pub number_of_accounts: u32,
    /// Use a bip39 mnemonic phrase for generating a PRNG seed, which is in turn used for hierarchical deterministic (HD) account generation.
    pub mnemonic: String,
}

impl Default for GanacheCliCmd {
    fn default() -> Self {
        GanacheCliCmd {
            network_id: 42,
            number_of_accounts: 7,
            mnemonic: "supersecure".to_string(),
        }
    }
}

impl IntoIterator for &GanacheCliCmd {
    type Item = String;
    type IntoIter = <Vec<String> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        let mut args = Vec::new();

        if !self.mnemonic.is_empty() {
            args.push("-m".to_string());
            args.push(self.mnemonic.to_string());
        }

        args.push("-a".to_string());
        args.push(self.number_of_accounts.to_string());
        args.push("-i".to_string());
        args.push(self.network_id.to_string());

        args.into_iter()
    }
}

impl Image for GanacheCli {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn expose_ports(&self) -> &[ContainerPort] {
        &[GANACHE_CLI_PORT]
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Listening on localhost:")]
    }

    fn cmd(&self) -> impl IntoIterator<Item = impl Into<Cow<'_, str>>> {
        &self.cmd
    }
}

#[cfg(test)]
mod tests {
    use testcontainers::runners::SyncRunner;

    use super::*;

    #[test]
    fn trufflesuite_ganachecli_listaccounts() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let _ = pretty_env_logger::try_init();
        let node = GanacheCli::default().start()?;
        let host_ip = node.get_host()?;
        let host_port = node.get_host_port_ipv4(GANACHE_CLI_PORT)?;

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

        assert_eq!(response["result"], "42");
        Ok(())
    }
}
