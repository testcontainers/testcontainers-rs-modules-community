use testcontainers::{core::WaitFor, Image};

const NAME: &str = "scylladb/scylla";
const TAG: &str = "2025.1.0";

/// Module to work with [`ScyllaDB`] inside of tests.
///
/// This module is based on the official [`ScyllaDB docker image`].
///
/// # Example
/// ```
/// use scylla::client::{session::Session, session_builder::SessionBuilder};
/// use testcontainers_modules::{scylladb::ScyllaDB, testcontainers::runners::AsyncRunner};
///
/// #[tokio::test]
/// async fn default_scylladb() -> Result<(), Box<dyn std::error::Error + 'static>> {
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
///     assert_eq!(version, "3.0.8");
///     Ok(())
/// }
/// ```
///
/// [`ScyllaDB`]: https://www.scylladb.com/
/// [`ScyllaDB docker image`]: https://hub.docker.com/r/scylladb/scylla
#[derive(Clone, Debug, Default)]
pub struct ScyllaDB {}

impl Image for ScyllaDB {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stderr("init - serving")]
    }
}

#[cfg(test)]
mod tests {
    use scylla::client::{session::Session, session_builder::SessionBuilder};
    use testcontainers::runners::AsyncRunner;

    use super::*;

    #[tokio::test]
    async fn scylladb_select_version() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let image = ScyllaDB::default();
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
        assert_eq!(version, "3.0.8");
        Ok(())
    }
}
