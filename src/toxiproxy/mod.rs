use testcontainers::{
    core::{wait::HttpWaitStrategy, ContainerPort, WaitFor},
    Image,
};

const NAME: &str = "ghcr.io/shopify/toxiproxy";
const TAG: &str = "2.12.0";

/// Port of the Toxiproxy control (HTTP) API.
///
/// Send requests here to create/update/delete proxies and toxics at runtime. Read the mapped host
/// port with `get_host_port_ipv4` on the started container.
pub const CONTROL_PORT: ContainerPort = ContainerPort::Tcp(8474);

/// Module to work with [`Toxiproxy`] inside of tests.
///
/// Toxiproxy is a TCP proxy that deterministically simulates adverse network conditions (latency,
/// bandwidth limits, timeouts, connection resets, ...) for resiliency and chaos testing. Proxies and
/// their "toxics" are configured at runtime through an HTTP API exposed on [`CONTROL_PORT`] (`8474`).
///
/// This module is based on the official [`Toxiproxy docker image`].
///
/// # Exposing proxy ports
///
/// A proxy *listens* on a port chosen at runtime, but a port is only reachable from the host if it is
/// published when the container starts. Declare each proxy port up front with
/// [`Toxiproxy::with_proxy_port`], then create a matching proxy at runtime whose `listen` address is
/// `0.0.0.0:<port>` and read the mapped host port with `get_host_port_ipv4(<port>)`.
///
/// # Example
/// ```
/// use testcontainers_modules::{testcontainers::runners::SyncRunner, toxiproxy::Toxiproxy};
///
/// let toxiproxy = Toxiproxy::default().start().unwrap();
/// let control_api_port = toxiproxy.get_host_port_ipv4(8474).unwrap();
///
/// // Talk to the Toxiproxy HTTP API at 127.0.0.1:{control_api_port} to create proxies and toxics.
/// ```
///
/// # Controlling proxies and toxics
///
/// This module intentionally stays a thin container wrapper: it does not ship an HTTP client. Drive
/// the control API with any HTTP client (e.g. [`reqwest`](https://docs.rs/reqwest)) using your own
/// strongly-typed request bodies. See `examples/toxiproxy.rs` for a complete, strongly-typed example
/// that creates a proxy, adds a latency toxic and routes traffic through the proxy.
///
/// [`Toxiproxy`]: https://github.com/Shopify/toxiproxy
/// [`Toxiproxy docker image`]: https://github.com/Shopify/toxiproxy/pkgs/container/toxiproxy
#[derive(Debug, Clone)]
pub struct Toxiproxy {
    exposed_ports: Vec<ContainerPort>,
}

impl Default for Toxiproxy {
    fn default() -> Self {
        Self {
            exposed_ports: vec![CONTROL_PORT],
        }
    }
}

impl Toxiproxy {
    /// Publish an additional container port that a proxy will `listen` on.
    ///
    /// The port is exposed so Docker maps it to a host port, making proxies created on it reachable
    /// from the test. Create the matching proxy at runtime via the control API with
    /// `listen = "0.0.0.0:<port>"`, then read the mapped host port with `get_host_port_ipv4(<port>)`.
    pub fn with_proxy_port(mut self, port: u16) -> Self {
        self.exposed_ports.push(ContainerPort::Tcp(port));
        self
    }
}

impl Image for Toxiproxy {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        let strategy = HttpWaitStrategy::new("/version")
            .with_port(CONTROL_PORT)
            .with_response_matcher(|response| response.status().is_success());
        vec![WaitFor::http(strategy)]
    }

    fn expose_ports(&self) -> &[ContainerPort] {
        &self.exposed_ports
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use serde::Serialize;

    use super::*;
    use crate::testcontainers::runners::AsyncRunner;

    const PROXY_PORT: u16 = 8666;

    /// Request body for `POST /proxies`.
    #[derive(Serialize)]
    struct CreateProxy {
        name: &'static str,
        listen: String,
        upstream: String,
        enabled: bool,
    }

    /// Attributes of a `latency` toxic (milliseconds).
    #[derive(Serialize)]
    struct LatencyAttributes {
        latency: u64,
        jitter: u64,
    }

    /// Request body for `POST /proxies/{proxy}/toxics`.
    #[derive(Serialize)]
    struct CreateToxic {
        #[serde(rename = "type")]
        kind: &'static str,
        stream: &'static str,
        toxicity: f64,
        attributes: LatencyAttributes,
    }

    #[tokio::test]
    async fn proxies_traffic_and_applies_latency_toxic(
    ) -> Result<(), Box<dyn std::error::Error + 'static>> {
        let _ = pretty_env_logger::try_init();

        let container = Toxiproxy::default()
            .with_proxy_port(PROXY_PORT)
            .start()
            .await?;
        let host = container.get_host().await?;
        let api_port = container.get_host_port_ipv4(CONTROL_PORT).await?;
        let proxy_port = container.get_host_port_ipv4(PROXY_PORT).await?;

        let api = format!("http://{host}:{api_port}");
        let through_proxy = format!("http://{host}:{proxy_port}");
        let http = reqwest::Client::new();

        // Create a proxy whose upstream is Toxiproxy's own control API, which is reachable at
        // localhost:8474 from inside the container. This keeps the test self-contained (no second
        // container or user-defined network needed).
        let create_proxy = CreateProxy {
            name: "self_api",
            listen: format!("0.0.0.0:{PROXY_PORT}"),
            upstream: format!("localhost:{}", CONTROL_PORT.as_u16()),
            enabled: true,
        };
        let response = http
            .post(format!("{api}/proxies"))
            .json(&create_proxy)
            .send()
            .await?;
        assert!(
            response.status().is_success(),
            "creating proxy failed: {}",
            response.status()
        );

        // Sanity check: a request through the proxy reaches the upstream API.
        let response = http.get(format!("{through_proxy}/version")).send().await?;
        assert!(
            response.status().is_success(),
            "request through proxy failed: {}",
            response.status()
        );

        // Add a downstream latency toxic and assert the proxied response is now delayed.
        let toxic = CreateToxic {
            kind: "latency",
            stream: "downstream",
            toxicity: 1.0,
            attributes: LatencyAttributes {
                latency: 800,
                jitter: 0,
            },
        };
        let response = http
            .post(format!("{api}/proxies/self_api/toxics"))
            .json(&toxic)
            .send()
            .await?;
        assert!(
            response.status().is_success(),
            "adding toxic failed: {}",
            response.status()
        );

        let started = Instant::now();
        let response = http.get(format!("{through_proxy}/version")).send().await?;
        let elapsed = started.elapsed();
        assert!(response.status().is_success());
        assert!(
            elapsed >= Duration::from_millis(700),
            "latency toxic should have delayed the response, but it took {elapsed:?}"
        );

        Ok(())
    }
}
