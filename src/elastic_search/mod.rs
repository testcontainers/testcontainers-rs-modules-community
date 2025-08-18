use std::borrow::Cow;

use testcontainers::{
    core::{ContainerPort, WaitFor},
    Image,
};

const NAME: &str = "docker.elastic.co/elasticsearch/elasticsearch";
const TAG: &str = "7.16.1";
/// Port that the [`Elasticsearch`] container has internally
/// Used **for API calls over http**, including search, aggregation, monitoring, ...
/// Client libraries have switched to using this to communicate to elastic.
/// Can be rebound externally via [`testcontainers::core::ImageExt::with_mapped_port`]
///
/// [`Elasticsearch`]: https://elastic.co/
pub const ELASTICSEARCH_API_PORT: ContainerPort = ContainerPort::Tcp(9200);
/// Port that the [`Elasticsearch`] container has internally.
/// Used **for nodes to communicate between each other** and handles cluster updates naster elections, nodes leaving/joining, ...
/// Can be rebound externally via [`testcontainers::core::ImageExt::with_mapped_port`]
///
/// [`Elasticsearch`]: https://elastic.co/
pub const ELASTICSEARCH_INTER_NODE_PORT: ContainerPort = ContainerPort::Tcp(9300);

/// Module to work with [`Elasticsearch`] inside of tests.
///
/// Starts an instance of Elasticsearch based on the official [`Elasticsearch docker image`].
///
/// Elasticsearch is a distributed, RESTful search and analytics engine capable of addressing
/// a growing number of use cases. This module provides a local Elasticsearch instance for testing purposes.
/// The container exposes port `9200` for API calls ([`ELASTICSEARCH_API_PORT`]) and port `9300` for
/// inter-node communication ([`ELASTICSEARCH_INTER_NODE_PORT`]) by default.
///
/// # Example
/// ```
/// use testcontainers_modules::{
///     elastic_search::ElasticSearch, testcontainers::runners::SyncRunner,
/// };
///
/// let elasticsearch_instance = ElasticSearch::default().start().unwrap();
/// let host = elasticsearch_instance.get_host().unwrap();
/// let port = elasticsearch_instance.get_host_port_ipv4(9200).unwrap();
///
/// // Use the Elasticsearch API at http://{host}:{port}
/// ```
///
/// [`Elasticsearch`]: https://www.elastic.co/elasticsearch/
/// [`Elasticsearch docker image`]: https://www.docker.elastic.co/r/elasticsearch/elasticsearch
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
        &[ELASTICSEARCH_API_PORT, ELASTICSEARCH_INTER_NODE_PORT]
    }
}

#[cfg(test)]
mod tests {}
