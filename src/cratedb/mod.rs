use std::{borrow::Cow, collections::HashMap};

use testcontainers::{core::WaitFor, CopyToContainer, Image};

const NAME: &str = "crate";
const TAG: &str = "5.8";

/// Represents a CrateDB docker instance.
/// It is based on the official [`crate docker image`]
///
/// Defaults are not modified:
/// super_user: crate
/// password: <empty>
/// pg_port: 5432
/// http_port: 4200
///
/// You can connect run instructions by sending an HTTP call to /_sql in port 4200.
/// [`crate http interface`]
///
/// or using the postgres wire compatible protocol at port 5432.
///
/// # Example of postgres wire.
/// use testcontainers_modules::{postgres, testcontainers::runners::SyncRunner};
///
/// let postgres_instance = cratedb::CrateDB::default().start().unwrap();
///
/// let connection_string = format!(
///     "postgres://crate@{}:{}/postgres",
///     postgres_instance.get_host().unwrap(),
///     postgres_instance.get_host_port_ipv4(5432).unwrap()
/// );
///
/// let mut conn = postgres::Client::connect(connection_string, postgres::NoTls).unwrap();
/// let rows = conn.query("SELECT 1 + 1", &[]).unwrap();
///
/// [`crate docker image`]: https://hub.docker.com/_/crate
/// [`crate http interface`]: https://cratedb.com/docs/crate/reference/en/latest/interfaces/http.html
#[derive(Debug, Clone)]
pub struct CrateDB {
    env_vars: HashMap<String, String>,
    copy_to_sources: Vec<CopyToContainer>,
}

impl CrateDB {
    /// Sets CrateDB's heap size, only increase it if you are testing big high volumes of data.
    pub fn with_heap_size(mut self, ram_gb: usize) -> Self {
        self.env_vars
            .insert("CRATE_HEAP_SIZE".to_string(), format!("{}g", ram_gb));
        self
    }
}
impl Default for CrateDB {
    fn default() -> Self {
        let mut env_vars = HashMap::new();
        env_vars.insert("CRATE_HEAP_SIZE".to_string(), "1g".to_string());
        Self {
            env_vars,
            copy_to_sources: Vec::new(),
        }
    }
}

impl Image for CrateDB {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("started")]
    }

    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        &self.env_vars
    }

    fn copy_to_sources(&self) -> impl IntoIterator<Item = &CopyToContainer> {
        &self.copy_to_sources
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use testcontainers::{runners::SyncRunner, ImageExt};
    #[test]
    fn cratedb_one_pls_one() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let crate_image = CrateDB::default();

        let node = crate_image.start()?;
        let connection_string = &format!(
            "postgres://crate@{}:{}/postgres",
            node.get_host()?,
            node.get_host_port_ipv4(5432)?
        );

        let mut conn = postgres::Client::connect(connection_string, postgres::NoTls).unwrap();
        let rows = conn.query("SELECT 1 + 1", &[]).unwrap();
        assert_eq!(rows.len(), 1);

        let first_row = &rows[0];
        let first_column: i32 = first_row.get(0);
        assert_eq!(first_column, 2);

        Ok(())
    }

    #[test]
    fn cratedb_custom_version() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let node = CrateDB::default().with_tag("5.4.3").start()?;

        let connection_string = &format!(
            "postgres://crate:crate@{}:{}/postgres",
            node.get_host()?,
            node.get_host_port_ipv4(5432)?
        );
        let mut conn = postgres::Client::connect(connection_string, postgres::NoTls).unwrap();

        let rows = conn.query("SELECT version()", &[]).unwrap();
        assert_eq!(rows.len(), 1);

        let first_row = &rows[0];
        let first_column: String = first_row.get(0);
        assert!(first_column.contains("5.4.3"));
        Ok(())
    }
}
