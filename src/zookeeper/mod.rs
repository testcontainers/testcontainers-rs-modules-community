use std::borrow::Cow;

use testcontainers::{core::WaitFor, Image};

const NAME: &str = "bitnamilegacy/zookeeper";
const TAG: &str = "3.9.0";

/// # [Apache ZooKeeper] image for [testcontainers](https://crates.io/crates/testcontainers).
///
/// This image is based on the [`bitnamilegacy/zookeeper` docker image].
/// By default, anonymous logins are allowed.
/// See the [Zookeeper documentation] for additional options.
///
/// # Example
///
/// ```
/// async {
///     use testcontainers_modules::{testcontainers::runners::AsyncRunner, zookeeper};
///
///     let node = zookeeper::Zookeeper::default().start().await.unwrap();
///     let zk_url = format!(
///         "{}:{}",
///         node.get_host().await.unwrap(),
///         node.get_host_port_ipv4(2181).await.unwrap(),
///     );
///     let zk_socket_addr = tokio::net::lookup_host(&zk_url)
///         .await
///         .unwrap()
///         .next()
///         .unwrap();
///
///     let (zk, default_watcher) = tokio_zookeeper::ZooKeeper::connect(&zk_socket_addr)
///         .await
///         .expect("connect to Zookeeper");
///
///     let path = "/test";
///     let _stat = zk.watch().exists(path).await.expect("stat received");
/// };
/// ```
///
///
/// [Apache ZooKeeper]: https://zookeeper.apache.org/
/// [`bitnamilegacy/zookeeper` docker image]: https://hub.docker.com/r/bitnamilegacy/zookeeper
/// [Zookeeper documentation]: https://zookeeper.apache.org/documentation.html
#[derive(Debug, Default, Clone)]
pub struct Zookeeper {
    /// (remove if there is another variable)
    /// Field is included to prevent this struct to be a unit struct.
    /// This allows extending functionality (and thus further variables) without breaking changes
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
    use futures::StreamExt;
    use rustls::crypto::CryptoProvider;
    use tokio::net::lookup_host;
    use tokio_zookeeper::*;

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

        let host = node.get_host().await?;
        let host_port = node.get_host_port_ipv4(2181).await?;
        let zk_url = format!("{host}:{host_port}");
        let zk_socket_addr = lookup_host(&zk_url).await?.next().unwrap();

        let (zk, mut default_watcher) = ZooKeeper::connect(&zk_socket_addr).await.unwrap();

        let path = "/test";
        let _stat = zk.watch().exists(path).await.expect("stat requested");

        let path = zk
            .create(path, &[1, 2], Acl::open_unsafe(), CreateMode::Ephemeral)
            .await?
            .expect("create a node");

        let event = default_watcher.next().await.expect("event received");
        assert_eq!(event.event_type, WatchedEventType::NodeCreated);
        assert_eq!(event.path, path);
        Ok(())
    }
}
