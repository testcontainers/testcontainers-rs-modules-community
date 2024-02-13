use std::collections::HashMap;

use testcontainers::{core::WaitFor, Image};

const NAME: &str = "solr";
const TAG: &str = "9.5.0-slim";

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
    use testcontainers::clients;
    use reqwest::{self, StatusCode};

    use super::*;

    #[test]
    fn solr_ping() {
        let docker = clients::Cli::default();
        let solr_image = Solr::default();
        let container = docker.run(solr_image);
        let host_port = container.get_host_port_ipv4(8983);

        let url = format!("http://localhost:{}/solr/admin/cores?action=STATUS", host_port);
        let res = reqwest::blocking::get(url)
            .expect("valid HTTP response");

        assert_eq!(res.status(), StatusCode::OK);

        let json: serde_json::Value = res.json()
            .expect("valid JSON body");

        assert_eq!(json["responseHeader"]["status"], 0);
    }
}
