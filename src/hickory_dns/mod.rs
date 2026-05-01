use std::borrow::Cow;

use testcontainers::{
    core::{ContainerPort, WaitFor},
    CopyDataSource, CopyToContainer, Image,
};

const NAME: &str = "hickorydns/hickory-dns";
const TAG: &str = "latest";

const CONFIG_PATH: &str = "/etc/named.toml";
const ZONE_DIR: &str = "/var/named";

/// Module to work with a [`Hickory DNS`] server inside of tests.
///
/// Based on the official [`Hickory DNS docker image`].
///
/// # Example
/// ```
/// use testcontainers_modules::{hickory_dns::HickoryDns, testcontainers::runners::SyncRunner};
///
/// const CONFIG: &str = r#"
///     [[zones]]
///     file = "example.com.zone"
///     zone = "example.com"
///     zone_type = "Primary"
/// "#;
///
/// const ZONE: &str = r#"
/// @ 3600 IN SOA ns.example.com. admin.example.com. (
///     1 ; SerialNA
///     1h ; Refresh
///     10m ; Retry
///     10d ; Expire
///     10h ; Negative Caching TTL
/// )
///
/// simple IN A 10.0.0.1
/// "#;
///
/// let instance = HickoryDns::new(CONFIG.as_bytes().to_vec())
///     .with_zone("example.com.zone", ZONE.as_bytes().to_vec())
///     .start()
///     .unwrap();
///
/// let port = instance
///     .get_host_port_ipv4(HickoryDns::INTERNAL_PORT)
///     .unwrap();
///
/// let dig_str = format!("dig @127.0.0.1 -p {port} simple.example.com.");
/// ```
///
/// [`Hickory DNS`]: https://github.com/hickory-dns/hickory-dns
/// [`Hickory DNS docker image`]: https://hub.docker.com/r/hickorydns/hickory-dns
#[derive(Debug, Clone)]
pub struct HickoryDns {
    files: Vec<CopyToContainer>,
}

impl HickoryDns {
    /// Internal port for TCP and UDP connections.
    pub const INTERNAL_PORT: u16 = 53;

    /// # Arguments
    ///
    /// - `config`: Server config file to be placed in `/etc/named.toml`. Futher
    ///   example configurations are provided in the project's [`test_configs`]
    ///   directory.
    ///
    /// [`test_configs`]: https://github.com/hickory-dns/hickory-dns/tree/main/tests/test-data/test_configs
    pub fn new(config: impl Into<CopyDataSource>) -> Self {
        let config_file = CopyToContainer::new(config.into(), CONFIG_PATH);

        Self {
            files: vec![config_file],
        }
    }

    /// # Arguments
    ///
    /// - `filename`: Referenced from the config file for the corresponding zone.
    /// - `description`: Zone file description as described in [RFC 1034 (section 3.6.1)][RFC1034] and [RFC 1035 (section 5)][RFC1035]
    ///
    /// [RFC1034]: https://datatracker.ietf.org/doc/html/rfc1034#section-3.6.1
    /// [RFC1035]: https://datatracker.ietf.org/doc/html/rfc1035#section-5
    pub fn with_zone(
        mut self,
        filename: impl Into<Cow<'static, str>>,
        description: impl Into<CopyDataSource>,
    ) -> Self {
        let target = format!("{ZONE_DIR}/{}", filename.into());
        let zone_file = CopyToContainer::new(description, target);
        self.files.push(zone_file);

        self
    }
}

impl Image for HickoryDns {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout(
            "server starting up, awaiting connections...",
        )]
    }

    fn expose_ports(&self) -> &[testcontainers::core::ContainerPort] {
        &[
            ContainerPort::Tcp(Self::INTERNAL_PORT),
            ContainerPort::Udp(Self::INTERNAL_PORT),
        ]
    }

    fn copy_to_sources(&self) -> impl IntoIterator<Item = &CopyToContainer> {
        &self.files
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    use hickory_resolver::{
        config::{ConnectionConfig, NameServerConfig, ProtocolConfig, ResolverConfig},
        net::runtime::TokioRuntimeProvider,
    };
    use testcontainers::runners::AsyncRunner;

    use super::*;

    const CONFIG: &str = r#"
        [[zones]]
        file = "example.com.zone"
        zone = "example.com"
        zone_type = "Primary"
    "#;

    const ZONE: &str = r#"
@ 3600 IN SOA ns.example.com. admin.example.com. (
    1 ; SerialNA
    1h ; Refresh
    10m ; Retry
    10d ; Expire
    10h ; Negative Caching TTL
)

simple IN A 10.0.0.1
    "#;

    #[tokio::test]
    async fn a_record_query() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let container = HickoryDns::new(CONFIG.as_bytes().to_vec())
            .with_zone("example.com.zone", ZONE.as_bytes().to_vec())
            .start()
            .await?;

        let port = container
            .get_host_port_ipv4(HickoryDns::INTERNAL_PORT)
            .await?;

        let name_server_config = {
            let mut tcp_connection = ConnectionConfig::new(ProtocolConfig::Tcp);
            tcp_connection.port = port;

            let mut udp_connection = ConnectionConfig::new(ProtocolConfig::Udp);
            udp_connection.port = port;

            NameServerConfig::new(
                IpAddr::V4(Ipv4Addr::LOCALHOST),
                true,
                vec![udp_connection, tcp_connection],
            )
        };

        let resolver_config = ResolverConfig::from_parts(None, vec![], vec![name_server_config]);

        let resolver = hickory_resolver::Resolver::builder_with_config(
            resolver_config,
            TokioRuntimeProvider::new(),
        )
        .build()?;

        let response = resolver.ipv4_lookup("simple.example.com.").await?;

        let actual = response.answers().first().unwrap().data.ip_addr().unwrap();

        let expected = "10.0.0.1".parse::<IpAddr>().unwrap();

        assert_eq!(expected, actual);

        Ok(())
    }
}
