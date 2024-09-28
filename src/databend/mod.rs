use std::{borrow::Cow, collections::BTreeMap};

use testcontainers::{
    core::{wait::HttpWaitStrategy, ContainerPort, WaitFor},
    Image,
};

const DEFAULT_IMAGE_NAME: &str = "datafuselabs/databend";
const DEFAULT_IMAGE_TAG: &str = "v1.2.615";

/// Port that the [`Databend`] container has internally
/// Can be rebound externally via [`testcontainers::core::ImageExt::with_mapped_port`]
///
/// [`Databend`]: https://databend.rs/
pub const DATABEND_PORT: ContainerPort = ContainerPort::Tcp(8000);

/// Module to work with [`Databend`] inside of tests.
///
/// This module is based on the official [`Databend docker image`].
///
/// # Example
/// ```
/// use testcontainers_modules::{databend, testcontainers::runners::SyncRunner};
///
/// let databend = databend::Databend::default().start().unwrap();
/// let http_port = databend.get_host_port_ipv4(8000).unwrap();
///
/// // do something with the started databend instance.
/// ```
///
/// [`Databend`]: https://databend.rs/
/// [`Databend docker image`]: https://hub.docker.com/r/datafuselabs/databend
#[derive(Debug, Clone)]
pub struct Databend {
    env_vars: BTreeMap<String, String>,
}

impl Databend {
    /// Sets the user for the Databend instance.
    pub fn with_query_user(mut self, user: &str) -> Self {
        self.env_vars
            .insert("QUERY_DEFAULT_USER".to_owned(), user.to_owned());
        self
    }

    /// Sets the password for the Databend instance.
    pub fn with_query_password(mut self, password: &str) -> Self {
        self.env_vars
            .insert("QUERY_DEFAULT_PASSWORD".to_owned(), password.to_owned());
        self
    }
}

impl Default for Databend {
    fn default() -> Self {
        let mut env_vars = BTreeMap::new();
        env_vars.insert("QUERY_DEFAULT_USER".to_owned(), "databend".to_owned());
        env_vars.insert("QUERY_DEFAULT_PASSWORD".to_owned(), "databend".to_owned());

        Self { env_vars }
    }
}

impl Image for Databend {
    fn name(&self) -> &str {
        DEFAULT_IMAGE_NAME
    }

    fn tag(&self) -> &str {
        DEFAULT_IMAGE_TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::http(
            HttpWaitStrategy::new("/").with_expected_status_code(200_u16),
        )]
    }

    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        &self.env_vars
    }

    fn expose_ports(&self) -> &[ContainerPort] {
        &[DATABEND_PORT]
    }
}

#[cfg(test)]
mod tests {
    use databend_driver::{Client};

    use crate::{databend::Databend as DatabendImage, testcontainers::runners::AsyncRunner};

    #[tokio::test]
    async fn test_databend() {
        let databend = DatabendImage::default().start().await.unwrap();
        let http_port = databend.get_host_port_ipv4(8000).await.unwrap();
        // "databend://user:password@localhost:8000/default?sslmode=disable
        let dsn = format!(
            "databend://databend:databend@localhost:{}/default?sslmode=disable",
            http_port
        );
        let client = Client::new(dsn.to_string());
        let conn = client.get_conn().await.unwrap();
        let row = conn.query_row("select 'hello'").await.unwrap();
        assert!(row.is_some());
        let row = row.unwrap();
        let (val,): (String,) = row.try_into().unwrap();
        assert_eq!(val, "hello");

        let conn2 = conn.clone();
        let row = conn2.query_row("select 'world'").await.unwrap();
        assert!(row.is_some());
        let row = row.unwrap();
        let (val,): (String,) = row.try_into().unwrap();
        assert_eq!(val, "world");
    }
}
