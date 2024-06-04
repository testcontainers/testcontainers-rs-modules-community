use std::collections::BTreeMap;

use testcontainers::{core::WaitFor, Image};

const DEFAULT_IMAGE_NAME: &str = "clickhouse/clickhouse-server";
const DEFAULT_IMAGE_TAG: &str = "23.3.8.21-alpine";

/// Module to work with [`ClickHouse`] inside of tests.
///
/// This module is based on the official [`ClickHouse docker image`].
///
/// # Example
/// ```
/// use testcontainers_modules::{clickhouse, testcontainers::runners::SyncRunner};
///
/// let clickhouse = clickhouse::ClickHouse::default().start().unwrap();
/// let http_port = clickhouse.get_host_port_ipv4(8123).unwrap();
///
/// // do something with the started clickhouse instance..
/// ```
///
/// [`ClickHouse`]: https://clickhouse.com/
/// [`Clickhouse docker image`]: https://hub.docker.com/r/clickhouse/clickhouse-server
#[derive(Debug)]
pub struct ClickHouse {
    name: String,
    tag: String,
    env_vars: BTreeMap<String, String>,
}

impl Default for ClickHouse {
    fn default() -> Self {
        ClickHouse::new(
            DEFAULT_IMAGE_NAME.to_string(),
            DEFAULT_IMAGE_TAG.to_string(),
        )
    }
}

impl ClickHouse {
    fn new(name: String, tag: String) -> Self {
        ClickHouse {
            name,
            tag,
            env_vars: Default::default(),
        }
    }
}

impl Image for ClickHouse {
    type Args = ();

    fn name(&self) -> String {
        self.name.clone()
    }

    fn tag(&self) -> String {
        self.tag.clone()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::seconds(10)]
    }

    fn env_vars(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(self.env_vars.iter())
    }
}

#[cfg(test)]
mod tests {
    use clickhouse::Row;
    use reqwest::Client;
    use serde::Deserialize;

    use crate::{clickhouse::ClickHouse as ClickhouseImage, testcontainers::runners::AsyncRunner};

    #[tokio::test]
    async fn clickhouse_db() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let clickhouse = ClickhouseImage::default();
        let node = clickhouse.start().await?;

        let host = node.get_host().await?;
        let port = node.get_host_port_ipv4(8123).await?;
        let url = format!("http://{}:{}", host, port);

        // testing http endpoint
        // curl http://localhost:8123/ping and check if the response is "Ok."
        let response = Client::new().get(&format!("{}/ping", url)).send().await?;
        assert_eq!(response.status(), 200);

        // create table
        let query = "CREATE TABLE t (a UInt8) ENGINE = Memory";
        let response = Client::new().post(url.clone()).body(query).send().await?;
        assert_eq!(response.status(), 200);

        // insert data
        let query = "INSERT INTO t VALUES (1),(2),(3)";
        let response = Client::new().post(url.clone()).body(query).send().await?;
        assert_eq!(response.status(), 200);

        // query data
        let query = "SELECT * FROM t";
        let response = Client::new().post(url.clone()).body(query).send().await?;
        assert_eq!(response.status(), 200);

        // testing tcp endpoint
        let client = clickhouse::Client::default().with_url(&format!("tcp://{}:{}", host, port));
        #[derive(Row, Deserialize)]
        struct MyRow {
            a: u8,
        }
        let rows = client.query("SELECT * FROM t").fetch_all::<MyRow>().await?;
        assert_eq!(rows.len(), 3);

        Ok(())
    }
}
