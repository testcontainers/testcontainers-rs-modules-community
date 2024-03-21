use std::collections::BTreeMap;

use testcontainers::core::WaitFor;
use testcontainers::Image;

const DEFAULT_IMAGE_NAME: &str = "hashicorp/consul";
const DEFAULT_IMAGE_TAG: &str = "1.16.1";
const CONSUL_LOCAL_CONFIG: &str = "CONSUL_LOCAL_CONFIG";

#[derive(Debug)]
pub struct Consul {
    name: String,
    tag: String,
    env_vars: BTreeMap<String, String>,
}

impl Default for Consul {
    fn default() -> Self {
        Consul::new(
            DEFAULT_IMAGE_NAME.to_string(),
            DEFAULT_IMAGE_TAG.to_string(),
        )
    }
}

impl Consul {
    fn new(name: String, tag: String) -> Self {
        Consul {
            name,
            tag,
            env_vars: Default::default(),
        }
    }

    pub fn with_local_config(self, config: String) -> Self {
        let mut env_vars = self.env_vars;
        env_vars.insert(CONSUL_LOCAL_CONFIG.to_owned(), config);
        Self { env_vars, ..self }
    }
}

impl Image for Consul {
    type Args = ();

    fn name(&self) -> String {
        self.name.clone()
    }

    fn tag(&self) -> String {
        self.tag.clone()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("agent: Consul agent running!")]
    }

    fn env_vars(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(self.env_vars.iter())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;
    use testcontainers::clients;

    use crate::consul::Consul;

    #[tokio::test]
    async fn consul_container() {
        let docker = clients::Cli::default();

        let consul = Consul::default().with_local_config("{\"datacenter\":\"dc-rust\"}".to_owned());
        let node = docker.run(consul);
        let port = node.get_host_port_ipv4(8500);

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
        assert_eq!("dc-rust", dc)
    }
}
