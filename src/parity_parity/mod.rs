use testcontainers::{core::WaitFor, Image, ImageArgs};

const NAME: &str = "parity/parity";
const TAG: &str = "v2.5.0";

#[derive(Debug, Default)]
pub struct ParityEthereum;

#[derive(Debug, Default, Clone)]
pub struct ParityEthereumArgs;

impl ImageArgs for ParityEthereumArgs {
    fn into_iterator(self) -> Box<dyn Iterator<Item = String>> {
        Box::new(
            vec![
                "--config=dev".to_string(),
                "--jsonrpc-apis=all".to_string(),
                "--unsafe-expose".to_string(),
                "--tracing=on".to_string(),
            ]
            .into_iter(),
        )
    }
}

impl Image for ParityEthereum {
    type Args = ParityEthereumArgs;

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stderr("Public node URL:")]
    }
}

#[cfg(test)]
mod tests {
    use testcontainers::clients;

    use crate::parity_parity;

    #[test]
    fn parity_parity_net_version() {
        let _ = pretty_env_logger::try_init();
        let docker = clients::Cli::default();
        let node = docker.run(parity_parity::ParityEthereum);
        let host_port = node.get_host_port_ipv4(8545);

        let response = reqwest::blocking::Client::new()
            .post(format!("http://127.0.0.1:{host_port}"))
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
    }
}
