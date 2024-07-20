use std::borrow::Cow;

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
/// use testcontainers_modules::{testcontainers::runners::SyncRunner, mysql};
///
/// let mysql_instance = mysql::Mysql::default().start().unwrap();
/// let mysql_url = format!(
///     "mysql://{}:{}/test",
///     mysql_instance.get_host().unwrap(),
///     mysql_instance.get_host_port_ipv4(3306).unwrap()
/// );
/// ```
///
/// [`MySQL`]: https://www.mysql.com/
/// [`MySQL docker image`]: https://hub.docker.com/_/mysql
#[derive(Debug, Default, Clone)]
pub struct Mysql {
    /// (remove if there is another variable)
    /// Field is included to prevent this struct to be a unit struct.
    /// This allows extending functionality (and thus further variables) without breaking changes
    _priv: (),
}

impl Image for Mysql {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![
            WaitFor::message_on_stderr("X Plugin ready for connections. Bind-address"),
            WaitFor::message_on_stderr("/usr/sbin/mysqld: ready for connections."),
        ]
    }

    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        [
            ("MYSQL_DATABASE", "test"),
            ("MYSQL_ALLOW_EMPTY_PASSWORD", "yes"),
        ]
    }
}

#[cfg(test)]
mod tests {
    use mysql::prelude::Queryable;

    use crate::{
        mysql::Mysql as MysqlImage,
        testcontainers::{runners::SyncRunner, ImageExt},
    };

    #[test]
    fn mysql_one_plus_one() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let mysql_image = MysqlImage::default();
        let node = mysql_image.start()?;

        let connection_string = &format!(
            "mysql://root@{}:{}/mysql",
            node.get_host()?,
            node.get_host_port_ipv4(3306)?
        );
        let mut conn = mysql::Conn::new(mysql::Opts::from_url(connection_string).unwrap()).unwrap();

        let first_row = conn.query_first("SELECT 1 + 1;").unwrap();
        assert_eq!(first_row, Some(2));

        let first_column: i32 = first_row.unwrap();
        assert_eq!(first_column, 2);
        Ok(())
    }

    #[test]
    fn mysql_custom_version() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let image = MysqlImage::default().with_tag("8.0.34");
        let node = image.start()?;

        let connection_string = &format!(
            "mysql://root@{}:{}/mysql",
            node.get_host()?,
            node.get_host_port_ipv4(3306)?
        );

        let mut conn = mysql::Conn::new(mysql::Opts::from_url(connection_string).unwrap()).unwrap();
        let first_row = conn.query_first("SELECT version()").unwrap();
        assert_eq!(first_row, Some(String::from("8.0.34")));
        Ok(())
    }
}
