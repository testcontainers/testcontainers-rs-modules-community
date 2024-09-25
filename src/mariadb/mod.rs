use std::borrow::Cow;

use testcontainers::{core::WaitFor, CopyDataSource, CopyToContainer, Image};

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
/// use testcontainers_modules::{mariadb, testcontainers::runners::SyncRunner};
///
/// let mariadb_instance = mariadb::Mariadb::default().start().unwrap();
/// let mariadb_url = format!(
///     "mariadb://{}:{}/test",
///     mariadb_instance.get_host().unwrap(),
///     mariadb_instance.get_host_port_ipv4(3306).unwrap(),
/// );
/// ```
///
/// [`MariaDB`]: https://www.mariadb.com/
/// [`MariaDB docker image`]: https://hub.docker.com/_/mariadb
#[derive(Debug, Default, Clone)]
pub struct Mariadb {
    copy_to_sources: Vec<CopyToContainer>,
}

impl Mariadb {
    /// Registers sql to be executed automatically when the container starts.
    ///
    /// # Example
    ///
    /// ```
    /// # use testcontainers_modules::mariadb::Mariadb;
    /// let mariadb_image = Mariadb::default().with_init_sql(
    ///     "CREATE TABLE foo (bar varchar(255));"
    ///         .to_string()
    ///         .into_bytes(),
    /// );
    /// ```
    ///
    /// ```rust,ignore
    /// # use testcontainers_modules::mariadb::Mariadb;
    /// let mariadb_image = Mariadb::default()
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

impl Image for Mariadb {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![
            WaitFor::message_on_stderr("mariadbd: ready for connections."),
            WaitFor::message_on_stderr("port: 3306"),
        ]
    }

    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        [
            ("MARIADB_DATABASE", "test"),
            ("MARIADB_ALLOW_EMPTY_ROOT_PASSWORD", "1"),
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
        mariadb::Mariadb as MariadbImage,
        testcontainers::{runners::SyncRunner, ImageExt},
    };

    #[test]
    fn mariadb_with_init_sql() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let node = MariadbImage::default()
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
    fn mariadb_one_plus_one() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let mariadb_image = MariadbImage::default();
        let node = mariadb_image.start()?;

        let connection_string = &format!(
            "mysql://root@{}:{}/test",
            node.get_host()?,
            node.get_host_port_ipv4(3306.tcp())?
        );
        let mut conn = mysql::Conn::new(mysql::Opts::from_url(connection_string).unwrap()).unwrap();

        let first_row = conn.query_first("SELECT 1 + 1;").unwrap();
        assert_eq!(first_row, Some(2));

        let first_column: i32 = first_row.unwrap();
        assert_eq!(first_column, 2);
        Ok(())
    }

    #[test]
    fn mariadb_custom_version() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let image = MariadbImage::default().with_tag("11.2.3");
        let node = image.start()?;

        let connection_string = &format!(
            "mysql://root@{}:{}/test",
            node.get_host()?,
            node.get_host_port_ipv4(3306.tcp())?
        );

        let mut conn = mysql::Conn::new(mysql::Opts::from_url(connection_string).unwrap()).unwrap();
        let first_row: Option<String> = conn.query_first("SELECT version()").unwrap();
        let first_column: String = first_row.unwrap();
        assert!(
            first_column.starts_with("11.2.3"),
            "Expected version to start with 11.2.3, got: {}",
            first_column
        );
        Ok(())
    }
}
