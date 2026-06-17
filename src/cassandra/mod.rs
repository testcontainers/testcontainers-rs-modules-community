use testcontainers::{core::WaitFor, Image};

const NAME: &str = "cassandra";
const TAG: &str = "5.0.6";

/// Module to work with [`Cassandra`] inside of tests.
///
/// This module is based on the official [`Cassandra docker image`].
///
/// # Example
/// ```
/// use scylla::client::{session::Session, session_builder::SessionBuilder};
/// use std::time::Duration;
/// use testcontainers::{runners::AsyncRunner, ImageExt};
///
/// #[tokio::test]
/// async fn default_cassandra() -> Result<(), Box<dyn std::error::Error + 'static>> {
///     let image = ScyllaDB::default();
///     let instance = image.start().await?;
///     let host = instance.get_host().await?;
///     let port = instance.get_host_port_ipv4(9042).await?;
///     let hostname = format!("{host}:{port}");
///     let session: Session = SessionBuilder::new().known_node(hostname).build().await?;
///
///     let prepared_statement = session
///         .prepare("SELECT release_version FROM system.local")
///         .await?;
///     let rows = session
///         .execute_unpaged(&prepared_statement, &[])
///         .await?
///         .into_rows_result()?;
///     let (version,) = rows.single_row::<(String,)>()?;
///     assert_eq!(version, "5.0.6");
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

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_either_std("Startup complete")]
    }
}

#[cfg(test)]
mod tests {
    use scylla::client::{session::Session, session_builder::SessionBuilder};
    use std::time::Duration;
    use testcontainers::{runners::AsyncRunner, ImageExt};

    use super::*;

    #[tokio::test]
    async fn cassandra_select_version() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let image = Cassandra::default().with_startup_timeout(Duration::from_secs(240));
        let instance = image.start().await?;
        let host = instance.get_host().await?;
        let port = instance.get_host_port_ipv4(9042).await?;
        let hostname = format!("{host}:{port}");
        let session: Session = SessionBuilder::new().known_node(hostname).build().await?;

        let prepared_statement = session
            .prepare("SELECT release_version FROM system.local")
            .await?;
        let rows = session
            .execute_unpaged(&prepared_statement, &[])
            .await?
            .into_rows_result()?;
        let (version,) = rows.single_row::<(String,)>()?;
        assert_eq!(version, "5.0.6");
        Ok(())
    }
}
