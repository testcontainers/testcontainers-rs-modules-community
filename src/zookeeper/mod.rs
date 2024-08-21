use std::borrow::Cow;

use testcontainers::{core::WaitFor, Image};

const NAME: &str = "bitnami/zookeeper";
const TAG: &str = "3.9.0";

/// # [Apache ZooKeeper] image for [testcontainers](https://crates.io/crates/testcontainers).
///
/// This image is based on the [`bitnami/zookeeper` docker image].
/// By default, anonymous logins are allowed.
/// See the [Zookeeper documentation] for additional options.
///
/// # Example
///
/// ```
/// use testcontainers_modules::{testcontainers::runners::AsyncRunner, zookeeper};
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let node = zookeeper::Zookeeper::default().start().await.unwrap();
/// let zk_url = format!(
///     "{}:{}",
///     node.get_host().await.unwrap(),
///     node.get_host_port_ipv4(2181).await.unwrap(),
/// );
/// let client = zookeeper_client::Client::connect(&zk_url)
///     .await
///     .expect("connect to Zookeeper");
///
/// let path = "/test";
/// let (_, stat_watcher) = client
///     .check_and_watch_stat(path)
///     .await
///     .expect("stat watcher created");
/// # })
/// ```
///
///
/// [Apache ZooKeeper]: https://zookeeper.apache.org/
/// [`bitnami/zookeeper` docker image]: https://hub.docker.com/r/bitnami/openldap
/// [Zookeeper documentation]: https://zookeeper.apache.org/documentation.html
#[derive(Debug, Default, Clone)]
pub struct Zookeeper {
    _priv: (),
}

impl Image for Zookeeper {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![
            WaitFor::message_on_stdout("Started AdminServer"),
            WaitFor::message_on_stdout("PrepRequestProcessor (sid:0) started"),
        ]
    }

    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        [("ALLOW_ANONYMOUS_LOGIN", "yes")]
    }
}

#[cfg(test)]
mod tests {
    use rustls::crypto::CryptoProvider;
    use zookeeper_client::{Acls, Client, CreateMode, EventType};

    use crate::{testcontainers::runners::AsyncRunner, zookeeper::Zookeeper as ZookeeperImage};

    #[tokio::test]
    async fn zookeeper_check_directories_existence(
    ) -> Result<(), Box<dyn std::error::Error + 'static>> {
        let _ = pretty_env_logger::try_init();
        if CryptoProvider::get_default().is_none() {
            rustls::crypto::ring::default_provider()
                .install_default()
                .expect("Error initializing rustls provider");
        }

        let node = ZookeeperImage::default().start().await?;

        let host_ip = node.get_host().await?;
        let host_port = node.get_host_port_ipv4(2181).await?;
        let zk_url = format!("{host_ip}:{host_port}");
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
        Ok(())
    }
}
