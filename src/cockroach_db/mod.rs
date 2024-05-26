use std::collections::BTreeMap;

use testcontainers::{core::WaitFor, Image, ImageArgs};

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
#[derive(Debug)]
pub struct CockroachDb {
    name: String,
    tag: String,
    env_vars: BTreeMap<String, String>,
}

impl Default for CockroachDb {
    fn default() -> Self {
        CockroachDb::new(
            DEFAULT_IMAGE_NAME.to_string(),
            DEFAULT_IMAGE_TAG.to_string(),
        )
    }
}

impl CockroachDb {
    fn new(name: String, tag: String) -> Self {
        CockroachDb {
            name,
            tag,
            env_vars: Default::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CockroachDbArgs {
    command: String,
    args: Vec<String>,
}

impl CockroachDbArgs {
    pub fn new(command: String, args: Vec<String>) -> Self {
        Self { command, args }
    }
}

impl Default for CockroachDbArgs {
    fn default() -> Self {
        Self {
            command: "start-single-node".to_string(),
            args: vec!["--insecure".to_string()],
        }
    }
}

impl ImageArgs for CockroachDbArgs {
    fn into_iterator(self) -> Box<dyn Iterator<Item = String>> {
        let mut command_and_args = self.args.clone();
        command_and_args.insert(0, self.command.clone());
        Box::new(command_and_args.into_iter())
    }
}

impl Image for CockroachDb {
    type Args = CockroachDbArgs;

    fn name(&self) -> String {
        self.name.clone()
    }

    fn tag(&self) -> String {
        self.tag.clone()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("CockroachDB node starting at")]
    }

    fn env_vars(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(self.env_vars.iter())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testcontainers::runners::SyncRunner;

    #[test]
    fn cockroach_db_one_plus_one() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let cockroach = CockroachDb::default();
        let node = cockroach.start()?;

        let connection_string = &format!(
            "postgresql://root@127.0.0.1:{}/defaultdb?sslmode=disable",
            node.get_host_port_ipv4(26257)?
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
