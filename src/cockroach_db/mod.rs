use std::borrow::Cow;

use testcontainers::{core::WaitFor, Image};

const DEFAULT_IMAGE_NAME: &str = "cockroachdb/cockroach";
const DEFAULT_IMAGE_TAG: &str = "v23.2.3";

/// Module to work with [`Cockroach DB`] inside of tests.
///
/// This module is based on the official [`Cockroach docker image`].
///
/// # Example
/// ```
/// use testcontainers_modules::{cockroach_db, testcontainers::runners::SyncRunner};
///
/// let cockroach = cockroach_db::CockroachDb::default().start().unwrap();
/// let http_port = cockroach.get_host_port_ipv4(26257).unwrap();
///
/// // do something with the started cockroach instance..
/// ```
///
/// [`Cockroach`]: https://www.cockroachlabs.com/
/// [`Cockroach docker image`]: https://hub.docker.com/r/cockroachdb/cockroach
/// [`Cockroach commands`]: https://www.cockroachlabs.com/docs/stable/cockroach-commands
#[derive(Debug, Default, Clone)]
pub struct CockroachDb {
    cmd: CockroachDbCmd,
}

impl CockroachDb {
    /// Create a new instance of a CockroachDb image.
    pub fn new(cmd: CockroachDbCmd) -> Self {
        CockroachDb { cmd }
    }
}

/// Specifies the command how CockroachDb should be started
#[derive(Debug, Clone, Copy)]
pub enum CockroachDbCmd {
    /// Start a single CockroachDB node
    StartSingleNode {
        /// `insecure` being set indicates that the container is intended for ***non-production
        /// testing only***. To run CockroachDB in production, use a secure cluster instead.
        ///
        /// Start a node with all security controls disabled.
        /// There is no encryption, no authentication and internal security checks are also disabled.
        /// This makes any client able to take over the entire cluster.
        /// This flag is only intended for non-production testing.
        ///
        /// Beware that using this flag on a public network while exposing the port is likely to
        /// cause the entire host container to become compromised.
        ///
        /// To simply accept non-TLS connections for SQL clients while keeping the cluster secure,
        /// consider using `--accept-sql-without-tls` instead.
        /// Also see: <https://go.crdb.dev/issue-v/53404/v24.2>
        insecure: bool,
    },
}

impl Default for CockroachDbCmd {
    fn default() -> Self {
        Self::StartSingleNode { insecure: true }
    }
}

impl Image for CockroachDb {
    fn name(&self) -> &str {
        DEFAULT_IMAGE_NAME
    }

    fn tag(&self) -> &str {
        DEFAULT_IMAGE_TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("CockroachDB node starting at")]
    }

    fn cmd(&self) -> impl IntoIterator<Item = impl Into<Cow<'_, str>>> {
        self.cmd
    }
}

impl IntoIterator for CockroachDbCmd {
    type Item = String;
    type IntoIter = <Vec<String> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            CockroachDbCmd::StartSingleNode { insecure } => {
                let mut cmd = vec!["start-single-node".to_string()];
                if insecure {
                    cmd.push("--insecure".to_string());
                }
                cmd.into_iter()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use testcontainers::core::IntoContainerPort;

    use super::*;
    use crate::testcontainers::runners::SyncRunner;

    #[test]
    fn cockroach_db_one_plus_one() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let cockroach = CockroachDb::default();
        let node = cockroach.start()?;

        let connection_string = &format!(
            "postgresql://root@127.0.0.1:{}/defaultdb?sslmode=disable",
            node.get_host_port_ipv4(26257.tcp())?
        );
        let mut conn = postgres::Client::connect(connection_string, postgres::NoTls).unwrap();

        let rows = conn.query("SELECT 1 + 1", &[]).unwrap();
        assert_eq!(rows.len(), 1);

        let first_row = &rows[0];
        let first_column: i64 = first_row.get(0);
        assert_eq!(first_column, 2);
        Ok(())
    }
}
