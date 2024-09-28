use std::borrow::Cow;

use testcontainers::{core::WaitFor, CopyDataSource, CopyToContainer, Image};

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
/// use testcontainers_modules::{mysql, testcontainers::runners::SyncRunner};
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
    copy_to_sources: Vec<CopyToContainer>,
}
impl Mysql {
    /// Registers sql to be executed automatically when the container starts.
    /// Can be called multiple times to add (not override) scripts.
    ///
    /// # Example
    ///
    /// ```
    /// # use testcontainers_modules::mysql::Mysql;
    /// let mysql_image = Mysql::default().with_init_sql(
    ///     "CREATE TABLE foo (bar varchar(255));"
    ///         .to_string()
    ///         .into_bytes(),
    /// );
    /// ```
    ///
    /// ```rust,ignore
    /// # use testcontainers_modules::mysql::Mysql;
    /// let mysql_image = Mysql::default()
    ///                                .with_init_sql(include_str!("path_to_init.sql").to_string().into_bytes());
    /// ```
    pub fn with_init_sql(mut self, init_sql: impl Into<CopyDataSource>) -> Self {
        let target = format!(
            "/docker-entrypoint-initdb.d/init_{i}.sql",
            i = self.copy_to_sources.len()
        );
        self.copy_to_sources
            .push(CopyToContainer::new(init_sql.into(), target));
        self
    }
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
    fn copy_to_sources(&self) -> impl IntoIterator<Item = &CopyToContainer> {
        &self.copy_to_sources
    }
}

#[cfg(test)]
mod tests {
    use mysql::prelude::Queryable;
    use testcontainers::core::IntoContainerPort;

    use crate::{
        mysql::Mysql as MysqlImage,
        testcontainers::{runners::SyncRunner, ImageExt},
    };

    #[test]
    fn mysql_with_init_sql() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let node = crate::mysql::Mysql::default()
            .with_init_sql(
                "CREATE TABLE foo (bar varchar(255));"
                    .to_string()
                    .into_bytes(),
            )
            .start()?;

        let connection_string = &format!(
            "mysql://root@{}:{}/test",
            node.get_host()?,
            node.get_host_port_ipv4(3306.tcp())?
        );
        let mut conn = mysql::Conn::new(mysql::Opts::from_url(connection_string).unwrap()).unwrap();

        let rows: Vec<String> = conn.query("INSERT INTO foo(bar) VALUES ('blub')").unwrap();
        assert_eq!(rows.len(), 0);

        let rows: Vec<String> = conn.query("SELECT bar FROM foo").unwrap();
        assert_eq!(rows.len(), 1);
        Ok(())
    }

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
