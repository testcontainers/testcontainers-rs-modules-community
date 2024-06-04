use std::collections::HashMap;

use testcontainers::{core::WaitFor, Image};

const DEFAULT_IMAGE_NAME: &str = "gvenzl/oracle-free";
const DEFAULT_IMAGE_TAG: &str = "23-slim-faststart";

/// Module to work with [`Oracle Database Free`] inside of tests.
/// The default image is [`gvenzl/oracle-free:23-slim-faststart`] (unofficial).
/// Official dockerfiles can be found [here][Oracle official dockerfiles].
///
/// The default schema is `test`, with a password `test`.
///
/// NOTE: Currently, there is no Oracle Database Free port for ARM chips,
/// hence Oracle Database Free images cannot run on the new Apple M chips via Docker Desktop.
///
/// # Example
/// ```
/// use std::time::Duration;
/// use testcontainers_modules::{oracle::free::Oracle, testcontainers::runners::SyncRunner};
///
/// // On slower machines the image sometimes needs to be pulled before,
/// // and there is more time needed than 60 seconds
/// // (the default startup timeout; pull is not timed).
///
/// // On a faster machine this should suffice:
/// // let oracle = Oracle::default().unwrap();
///
/// let oracle = Oracle::default()
///     .pull_image()
///     .unwrap()
///     .with_startup_timeout(Duration::from_secs(75))
///     .start()
///     .unwrap();
///
/// let http_port = oracle.get_host_port_ipv4(1521).unwrap();
///
/// // do something with the started Oracle instance..
/// ```
///
/// [`Oracle Database Free`]: https://www.oracle.com/database/free/
/// [Oracle official dockerfiles]: https://github.com/oracle/docker-images/tree/main/OracleDatabase
/// [`gvenzl/oracle-free:23-slim-faststart`]: https://hub.docker.com/r/gvenzl/oracle-free
#[derive(Debug)]
pub struct Oracle {
    name: String,
    tag: String,
    env_vars: HashMap<String, String>,
}

impl Default for Oracle {
    fn default() -> Self {
        let name = DEFAULT_IMAGE_NAME.to_owned();
        let tag = DEFAULT_IMAGE_TAG.to_owned();

        let mut env_vars = HashMap::new();
        env_vars.insert("ORACLE_PASSWORD".to_owned(), "testsys".to_owned());
        env_vars.insert("APP_USER".to_owned(), "test".to_owned());
        env_vars.insert("APP_USER_PASSWORD".to_owned(), "test".to_owned());

        Self {
            name,
            tag,
            env_vars,
        }
    }
}

impl Image for Oracle {
    type Args = ();

    fn name(&self) -> String {
        self.name.clone()
    }

    fn tag(&self) -> String {
        self.tag.clone()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("DATABASE IS READY TO USE!")]
    }

    fn env_vars(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(self.env_vars.iter())
    }

    fn expose_ports(&self) -> Vec<u16> {
        vec![1521]
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::testcontainers::runners::SyncRunner;

    // remember to provide Oracle client 11.2 or later (see https://crates.io/crates/oracle)

    #[test]
    fn oracle_one_plus_one() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let oracle = Oracle::default()
            .pull_image()?
            .with_startup_timeout(Duration::from_secs(75));

        let node = oracle.start()?;

        let connection_string = format!(
            "//{}:{}/FREEPDB1",
            node.get_host()?,
            node.get_host_port_ipv4(1521)?
        );
        let conn = oracle::Connection::connect("test", "test", connection_string)?;

        let mut rows = conn.query("SELECT 1 + 1", &[])?;
        let row = rows.next().unwrap()?;
        let col: i32 = row.get(0)?;
        assert_eq!(col, 2);
        Ok(())
    }
}
