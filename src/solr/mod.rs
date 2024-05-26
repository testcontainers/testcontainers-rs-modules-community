use std::collections::HashMap;

use testcontainers::{core::WaitFor, Image};

pub const SOLR_PORT: u16 = 8983;

const NAME: &str = "solr";
const TAG: &str = "9.5.0-slim";

/// Module to work with [`Solr`] inside of tests.
///
/// Starts an instance of Solr based on the official [`Solr docker image`].
///
/// By default Solr is exposed via HTTP on Port 8983 ([`SOLR_PORT`]) and has no access control. Please refer to the [`Solr reference guide`] for more informations on how to interact with the API.
///
/// # Example
/// ```
/// use testcontainers_modules::{solr, testcontainers::runners::SyncRunner};
///
/// let solr_instance = solr::Solr::default().start()?;
/// let host_port = solr_instance.get_host_port_ipv4(solr::SOLR_PORT)?;

/// let solr_url = format!("http://127.0.0.1:{}", host_port);
///
/// // use HTTP client to interact with the solr API
/// ```
///
/// [`Solr`]: https://solr.apache.org/
/// [`Solr docker image`]: https://hub.docker.com/_/solr
/// [`Solr reference guide`]: https://solr.apache.org/guide/solr/latest/
#[derive(Debug, Default)]
pub struct Solr {
    env_vars: HashMap<String, String>,
}

impl Image for Solr {
    type Args = ();

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("o.e.j.s.Server Started Server")]
    }

    fn env_vars(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(self.env_vars.iter())
    }
}

#[cfg(test)]
mod tests {
    use reqwest::{self, StatusCode};
    use testcontainers::runners::SyncRunner;

    use super::*;

    #[test]
    fn solr_ping() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let solr_image = Solr::default();
        let container = solr_image.start()?;
        let host_ip = container.get_host()?;
        let host_port = container.get_host_port_ipv4(SOLR_PORT)?;

        let url = format!(
            "http://{host_ip}:{}/solr/admin/cores?action=STATUS",
            host_port
        );
        let res = reqwest::blocking::get(url).expect("valid HTTP response");

        assert_eq!(res.status(), StatusCode::OK);

        let json: serde_json::Value = res.json().expect("valid JSON body");

        assert_eq!(json["responseHeader"]["status"], 0);
        Ok(())
    }
}
