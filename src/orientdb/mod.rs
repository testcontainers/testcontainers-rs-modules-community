use std::collections::HashMap;

use testcontainers::{core::WaitFor, Image};

const NAME: &str = "orientdb";
const TAG: &str = "3.2.19";

#[derive(Debug)]
pub struct OrientDb {
    env_vars: HashMap<String, String>,
}

impl Default for OrientDb {
    fn default() -> Self {
        let mut env_vars = HashMap::new();
        env_vars.insert("ORIENTDB_ROOT_PASSWORD".to_owned(), "root".to_owned());

        OrientDb { env_vars }
    }
}

impl Image for OrientDb {
    type Args = ();

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stderr("OrientDB Studio available at")]
    }

    fn env_vars(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(self.env_vars.iter())
    }
}

#[cfg(test)]
mod tests {
    use reqwest::StatusCode;
    use retry::{delay::Fixed, retry};

    use crate::{orientdb::OrientDb, testcontainers::runners::SyncRunner};

    #[test]
    fn orientdb_exists_database() {
        let _ = pretty_env_logger::try_init();
        let node = OrientDb::default().start().unwrap();
        let client = reqwest::blocking::Client::new();

        let response = retry(Fixed::from_millis(500).take(5), || {
            client
                .get(format!(
                    "http://{}:{}/listDatabases",
                    node.get_host().unwrap(),
                    node.get_host_port_ipv4(2480).unwrap()
                ))
                .header("Accept-Encoding", "gzip,deflate")
                .send()
        });

        assert_eq!(response.unwrap().status(), StatusCode::OK);
    }
}
