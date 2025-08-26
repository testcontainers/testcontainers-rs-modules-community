use std::borrow::Cow;

use testcontainers::{core::WaitFor, Image};

const NAME: &str = "orientdb";
const TAG: &str = "3.2.19";

/// Module to work with [`OrientDB`] inside of tests.
///
/// Starts an instance of OrientDB based on the official [`OrientDB docker image`].
///
/// OrientDB is a multi-model database, supporting graph, document, object, and key-value models.
/// This module provides a local OrientDB instance for testing purposes.
/// The container exposes port `2424` for binary connections and port `2480` for HTTP connections by default.
///
/// The default root password is set to `"root"`.
///
/// # Example
/// ```
/// use testcontainers_modules::{orientdb::OrientDb, testcontainers::runners::SyncRunner};
///
/// let orientdb_instance = OrientDb::default().start().unwrap();
/// let host = orientdb_instance.get_host().unwrap();
/// let http_port = orientdb_instance.get_host_port_ipv4(2480).unwrap();
/// let binary_port = orientdb_instance.get_host_port_ipv4(2424).unwrap();
///
/// // Use the HTTP endpoint at http://{host}:{http_port}
/// // Use the binary endpoint at {host}:{binary_port}
/// ```
///
/// [`OrientDB`]: https://orientdb.org/
/// [`OrientDB docker image`]: https://hub.docker.com/_/orientdb
#[derive(Debug, Default, Clone)]
pub struct OrientDb {
    /// (remove if there is another variable)
    /// Field is included to prevent this struct to be a unit struct.
    /// This allows extending functionality (and thus further variables) without breaking changes
    _priv: (),
}

impl Image for OrientDb {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stderr("OrientDB Studio available at")]
    }

    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        [("ORIENTDB_ROOT_PASSWORD", "root")]
    }
}

#[cfg(test)]
mod tests {
    use reqwest::StatusCode;
    use retry::{delay::Fixed, retry};

    use crate::{orientdb::OrientDb, testcontainers::runners::SyncRunner};

    #[test]
    fn orientdb_exists_database() {
        let _ = pretty_env_logger::try_init();
        let node = OrientDb::default().start().unwrap();
        let client = reqwest::blocking::Client::new();

        let response = retry(Fixed::from_millis(500).take(5), || {
            client
                .get(format!(
                    "http://{}:{}/listDatabases",
                    node.get_host().unwrap(),
                    node.get_host_port_ipv4(2480).unwrap()
                ))
                .header("Accept-Encoding", "gzip,deflate")
                .send()
        });

        assert_eq!(response.unwrap().status(), StatusCode::OK);
    }
}
