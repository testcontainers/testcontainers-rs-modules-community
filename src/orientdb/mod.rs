use std::borrow::Cow;

use testcontainers::{core::WaitFor, Image};

const NAME: &str = "orientdb";
const TAG: &str = "3.2.19";

#[allow(missing_docs, reason = "not having docs here is currently allowed to adress the missing docs problem one place at a time. If you would like to help us, documenting one of these places helps other devs tremendously")]
#[derive(Debug, Default, Clone)]
pub struct OrientDb {
    _priv: (),
}

impl Image for OrientDb {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stderr("OrientDB Studio available at")]
    }

    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        [("ORIENTDB_ROOT_PASSWORD", "root")]
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
