use std::borrow::Cow;

use testcontainers::{core::WaitFor, Image};

const NAME: &str = "trufflesuite/ganache-cli";
const TAG: &str = "v6.1.3";

#[derive(Debug, Default)]
pub struct GanacheCli {
    cmd: GanacheCliCmd,
}

#[derive(Debug, Clone)]
pub struct GanacheCliCmd {
    pub network_id: u32,
    pub number_of_accounts: u32,
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

    use crate::trufflesuite_ganachecli;

    #[test]
    fn trufflesuite_ganachecli_listaccounts() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let _ = pretty_env_logger::try_init();
        let node = trufflesuite_ganachecli::GanacheCli::default().start()?;
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

        assert_eq!(response["result"], "42");
        Ok(())
    }
}
