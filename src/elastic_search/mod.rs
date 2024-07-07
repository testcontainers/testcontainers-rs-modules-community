use std::borrow::Cow;

use testcontainers::{
    core::{ContainerPort, WaitFor},
    Image,
};

const NAME: &str = "docker.elastic.co/elasticsearch/elasticsearch";
const TAG: &str = "7.16.1";

#[derive(Debug, Default, Clone)]
pub struct ElasticSearch {
    /// (remove if there is another variable)
    /// Field is included to prevent this struct to be a unit struct.
    /// This allows extending functionality (and thus further variables) without breaking changes
    _priv: (),
}

impl Image for ElasticSearch {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("[YELLOW] to [GREEN]")]
    }

    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        [("discovery.type", "single-node")]
    }

    fn expose_ports(&self) -> &[ContainerPort] {
        &[ContainerPort::Tcp(9200), ContainerPort::Tcp(9300)]
    }
}

#[cfg(test)]
mod tests {}
