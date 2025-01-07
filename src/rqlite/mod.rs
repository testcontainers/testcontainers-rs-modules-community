use testcontainers::{
    core::{wait::HttpWaitStrategy, ContainerPort, WaitFor},
    Image,
};

/// Port that the [`RQLite`] container has internally
/// Can be rebound externally via [`testcontainers::core::ImageExt::with_mapped_port`]
///
/// [`RQLite`]: https://rqlite.io/
pub const RQLITE_PORT: ContainerPort = ContainerPort::Tcp(4001);

const NAME: &str = "rqlite/rqlite";
const TAG: &str = "8.36.3";

/// Module to work with [`RQLite`] inside of tests.
///
/// This module is based on the official [`RQLite docker image`].
///
/// # Example
/// ```
/// use testcontainers_modules::{rqlite, testcontainers::runners::SyncRunner};
///
/// let rqlite = rqlite::RQLite::default().start().unwrap();
/// let http_port = rqlite.get_host_port_ipv4(4001).unwrap();
///
/// // do something with the started rqlite instance..
/// ```
///
/// [`RQLite`]: https://rqlite.io/
/// [`RQLite docker image`]: https://hub.docker.com/r/rqlite/rqlite/
#[derive(Debug, Default, Clone)]
pub struct RQLite {
    /// (remove if there is another variable)
    /// Field is included to prevent this struct to be a unit struct.
    /// This allows extending functionality (and thus further variables) without breaking changes
    _priv: (),
}

impl Image for RQLite {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![
            WaitFor::http(HttpWaitStrategy::new("/status").with_expected_status_code(200_u16)),
            WaitFor::message_on_stderr("is now Leader"),
        ]
    }

    fn expose_ports(&self) -> &[ContainerPort] {
        &[RQLITE_PORT]
    }
}

#[cfg(test)]
mod tests {
    use testcontainers::runners::AsyncRunner;

    use crate::rqlite::RQLite;

    #[tokio::test]
    async fn rqlite_db() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let _ = pretty_env_logger::try_init();
        let node = RQLite::default().start().await?;
        let host_ip = node.get_host().await?;
        let host_port = node.get_host_port_ipv4(4001).await?;

        let client = rqlite_rs::RqliteClientBuilder::new()
            .known_host(format!("{}:{}", host_ip, host_port))
            .build()?;

        let query = rqlite_rs::query!("SELECT 1+1")?;
        let rows = client.fetch(query).await?;
        assert_eq!(rows.len(), 1);

        let first_row = &rows[0];
        let first_column: i32 = first_row.get("1+1")?;
        assert_eq!(first_column, 2);

        Ok(())
    }
}
