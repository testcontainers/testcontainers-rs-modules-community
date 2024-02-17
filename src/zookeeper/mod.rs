use std::collections::HashMap;

use testcontainers::{core::WaitFor, Image};

const NAME: &str = "bitnami/zookeeper";
const TAG: &str = "3.9.0";

#[derive(Debug)]
pub struct Zookeeper {
    env_vars: HashMap<String, String>,
}

impl Default for Zookeeper {
    fn default() -> Self {
        let mut env_vars = HashMap::new();
        env_vars.insert("ALLOW_ANONYMOUS_LOGIN".to_owned(), "yes".to_owned());

        Self { env_vars }
    }
}

impl Image for Zookeeper {
    type Args = ();

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![
            WaitFor::message_on_stdout("Started AdminServer"),
            WaitFor::message_on_stdout("PrepRequestProcessor (sid:0) started"),
        ]
    }

    fn env_vars(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(self.env_vars.iter())
    }
}

#[cfg(test)]
mod tests {
    use testcontainers::clients;
    use zookeeper_client::{Acls, Client, CreateMode, EventType};

    use crate::zookeeper::Zookeeper as ZookeeperImage;

    #[tokio::test]
    async fn zookeeper_check_directories_existence() {
        let _ = pretty_env_logger::try_init();

        let docker = clients::Cli::default();
        let node = docker.run(ZookeeperImage::default());

        let host_port = node.get_host_port_ipv4(2181);
        let zk_url = format!("127.0.0.1:{host_port}");
        let client = Client::connect(&zk_url)
            .await
            .expect("connect to Zookeeper");

        let path = "/test";
        let (_, stat_watcher) = client
            .check_and_watch_stat(path)
            .await
            .expect("stat watcher created");

        let create_options = CreateMode::Ephemeral.with_acls(Acls::anyone_all());
        let (_, _) = client
            .create(path, &[1, 2], &create_options)
            .await
            .expect("create a node");

        let event = stat_watcher.changed().await;
        assert_eq!(event.event_type, EventType::NodeCreated);
        assert_eq!(event.path, path);
    }
}
