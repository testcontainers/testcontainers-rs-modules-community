use std::collections::HashMap;

use testcontainers::{core::WaitFor, Image};

const NAME: &str = "mariadb";
const TAG: &str = "11.3";

/// Module to work with [`MariaDB`] inside of tests.
///
/// Starts an instance of MariaDB with no password set for the root user and a default database named `test` created.
///
/// This module is based on the official [`MariaDB docker image`].
///
/// # Example
/// ```
/// use testcontainers_modules::{testcontainers::runners::SyncRunner, mariadb};
///
/// let mariadb_instance = mariadb::Mariadb::default().start();
/// let mariadb_url = format!("mariadb://{}:{}/test", mariadb_instance.get_host(), mariadb_instance.get_host_port_ipv4(3306));
/// ```
///
/// [`MariaDB`]: https://www.mariadb.com/
/// [`MariaDB docker image`]: https://hub.docker.com/_/mariadb
#[derive(Debug)]
pub struct Mariadb {
    env_vars: HashMap<String, String>,
}

impl Default for Mariadb {
    fn default() -> Self {
        let mut env_vars = HashMap::new();
        env_vars.insert("MARIADB_DATABASE".to_owned(), "test".to_owned());
        env_vars.insert("MARIADB_ALLOW_EMPTY_ROOT_PASSWORD".into(), "1".into());

        Self { env_vars }
    }
}

impl Image for Mariadb {
    type Args = ();

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![
            WaitFor::message_on_stderr("mariadbd: ready for connections."),
            WaitFor::message_on_stderr("port: 3306"),
        ]
    }

    fn env_vars(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(self.env_vars.iter())
    }
}

#[cfg(test)]
mod tests {
    use mysql::prelude::Queryable;

    use crate::{
        mariadb::Mariadb as MariadbImage,
        testcontainers::{runners::SyncRunner, RunnableImage},
    };

    #[test]
    fn mariadb_one_plus_one() {
        let mariadb_image = MariadbImage::default();
        let node = mariadb_image.start();

        let connection_string = &format!(
            "mysql://root@{}:{}/test",
            node.get_host(),
            node.get_host_port_ipv4(3306)
        );
        let mut conn = mysql::Conn::new(mysql::Opts::from_url(connection_string).unwrap()).unwrap();

        let first_row = conn.query_first("SELECT 1 + 1;").unwrap();
        assert_eq!(first_row, Some(2));

        let first_column: i32 = first_row.unwrap();
        assert_eq!(first_column, 2);
    }

    #[test]
    fn mariadb_custom_version() {
        let image = RunnableImage::from(MariadbImage::default()).with_tag("11.2.3");
        let node = image.start();

        let connection_string = &format!(
            "mysql://root@{}:{}/test",
            node.get_host(),
            node.get_host_port_ipv4(3306)
        );

        let mut conn = mysql::Conn::new(mysql::Opts::from_url(connection_string).unwrap()).unwrap();
        let first_row: Option<String> = conn.query_first("SELECT version()").unwrap();
        let first_column: String = first_row.unwrap();
        assert!(
            first_column.starts_with("11.2.3"),
            "Expected version to start with 11.2.3, got: {}",
            first_column
        );
    }
}
