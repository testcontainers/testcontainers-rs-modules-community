use std::borrow::Cow;

use testcontainers::{core::WaitFor, Image};

const NAME: &str = "cassandra";
const TAG: &str = "5.0.6";

/// Module to work with [`Cassandra`] inside of tests.
///
/// This module is based on the official [`Cassandra docker image`].
///
/// # Example
/// ```
/// use std::time::Duration;
///
/// use testcontainers::{runners::AsyncRunner, ImageExt};
///
/// #[tokio::test]
/// async fn default_cassandra() -> Result<(), Box<dyn std::error::Error + 'static>> {
///     let image = Cassandra::default().with_startup_timeout(Duration::from_secs(120));
///     let instance = image.start().await?;
///     let host = instance.get_host().await?;
///     let port = instance.get_host_port_ipv4(9042).await?;
///     let hostname = format!("{host}:{port}");
///     // do something using a driver
///     Ok(())
/// }
/// ```
///
/// [`Cassandra`]: https://cassandra.apache.org
/// [`Cassandra docker image`]: https://hub.docker.com/_/cassandra
#[derive(Default, Clone, Debug)]
pub struct Cassandra {}

impl Image for Cassandra {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        [
            (
                "JVM_EXTRA_OPTS",
                "-Dcassandra.skip_wait_for_gossip_to_settle=0 -Dcassandra.initial_token=0",
            ),
            ("CASSANDRA_DC", "dc1"),
            ("CASSANDRA_SNITCH", "GossipingPropertyFileSnitch"),
            ("CASSANDRA_ENDPOINT_SNITCH", "GossipingPropertyFileSnitch"),
            ("HEAP_NEWSIZE", "128M"),
            ("MAX_HEAP_SIZE", "1024M"),
        ]
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_either_std("Startup complete")]
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use cdrs_tokio::{
        cluster::{
            session::{SessionBuilder, TcpSessionBuilder},
            NodeTcpConfigBuilder,
        },
        load_balancing::RoundRobinLoadBalancingStrategy,
        types::ByName,
    };
    use testcontainers::{runners::AsyncRunner, ImageExt};

    use super::*;

    #[tokio::test]
    async fn cassandra_select_version() -> Result<(), Box<dyn std::error::Error + 'static>> {
        pretty_env_logger::init();
        let image = Cassandra::default().with_startup_timeout(Duration::from_secs(120));
        let instance = image.start().await?;
        let host = instance.get_host().await?;
        let port = instance.get_host_port_ipv4(9042).await?;
        let hostname = format!("{host}:{port}");

        let cluster_config = NodeTcpConfigBuilder::new()
            .with_contact_point(hostname.into())
            .build()
            .await?;

        let session =
            TcpSessionBuilder::new(RoundRobinLoadBalancingStrategy::new(), cluster_config)
                .build()
                .await?;

        let result = session
            .query("SELECT release_version FROM system.local")
            .await?;

        let body = result.response_body()?;
        let rows = body.into_rows().unwrap();
        let version = rows
            .first()
            .unwrap()
            .by_name::<String>("release_version")?
            .unwrap();

        assert_eq!(version, TAG);
        Ok(())
    }
}
