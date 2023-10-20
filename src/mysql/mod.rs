use std::{collections::HashMap, time::Duration};

use testcontainers::{core::WaitFor, Image};

const NAME: &str = "mysql";
const TAG: &str = "8.1";

/// Module to work with [`MySQL`] inside of tests.
///
/// Starts an instance of MySQL with no password set for the root user and a default database named `test` created.
///
/// This module is based on the officlal [`MySQL docker image`].
///
/// # Example
/// ```
/// use testcontainers::clients;
/// use testcontainers_modules::mysql;
///
/// let docker = clients::Cli::default();
/// let mysql_instance = docker.run(mysql::Mysql::default());
///
/// let mysql_url = format!("mysql://127.0.0.1:{}/test", mysql_instance.get_host_port_ipv4(3306));
/// ```
///
/// [`MySQL`]: https://www.mysql.com/
/// [`MySQL docker image`]: https://hub.docker.com/_/mysql
#[derive(Debug)]
pub struct Mysql {
    env_vars: HashMap<String, String>,
}

impl Default for Mysql {
    fn default() -> Self {
        let mut env_vars = HashMap::new();
        env_vars.insert("MYSQL_DATABASE".to_owned(), "test".to_owned());
        env_vars.insert("MYSQL_ALLOW_EMPTY_PASSWORD".into(), "yes".into());

        Self { env_vars }
    }
}

impl Image for Mysql {
    type Args = ();

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![
            WaitFor::message_on_stderr("/usr/sbin/mysqld: ready for connections."),
            WaitFor::Duration {
                length: Duration::new(10, 0),
            },
        ]
    }

    fn env_vars(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(self.env_vars.iter())
    }
}

#[cfg(test)]
mod tests {
    use mysql::prelude::Queryable;
    use testcontainers::{clients, RunnableImage};

    use crate::mysql::Mysql as MysqlImage;

    #[test]
    fn mysql_one_plus_one() {
        let docker = clients::Cli::default();
        let mysql_image = MysqlImage::default();
        let node = docker.run(mysql_image);

        let connection_string = &format!(
            "mysql://root@127.0.0.1:{}/mysql",
            node.get_host_port_ipv4(3306)
        );
        let mut conn = mysql::Conn::new(mysql::Opts::from_url(connection_string).unwrap()).unwrap();

        let first_row = conn.query_first("SELECT 1 + 1;").unwrap();
        assert_eq!(first_row, Some(2));

        let first_column: i32 = first_row.unwrap();
        assert_eq!(first_column, 2);
    }

    #[test]
    fn mysql_custom_version() {
        let docker = clients::Cli::default();
        let image = RunnableImage::from(MysqlImage::default()).with_tag("8.0.34");
        let node = docker.run(image);

        let connection_string = &format!(
            "mysql://root@localhost:{}/mysql",
            node.get_host_port_ipv4(3306)
        );

        let mut conn = mysql::Conn::new(mysql::Opts::from_url(connection_string).unwrap()).unwrap();
        let first_row = conn.query_first("SELECT version()").unwrap();
        assert_eq!(first_row, Some(String::from("8.0.34")));
    }
}
