//! Demonstrates driving the Toxiproxy control API with strongly-typed request bodies:
//! create a proxy, route traffic through it, then add a `latency` toxic and watch the same
//! request slow down.
//!
//! Run with: `cargo run --example toxiproxy --features toxiproxy`
use std::time::Instant;

use serde::Serialize;
use testcontainers_modules::{
    testcontainers::runners::AsyncRunner,
    toxiproxy::{Toxiproxy, CONTROL_PORT},
};

/// Listen port we let a proxy bind inside the container (published to the host).
const PROXY_PORT: u16 = 8666;

/// Request body for `POST /proxies`.
#[derive(Serialize)]
struct Proxy {
    name: String,
    listen: String,
    upstream: String,
    enabled: bool,
}

/// Direction a toxic is applied in.
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)] // `Upstream` is part of the model even though this example only uses `Downstream`.
enum Stream {
    /// Applies on the client -> upstream connection.
    Upstream,
    /// Applies on the upstream -> client connection.
    Downstream,
}

/// Attributes of a `latency` toxic, in milliseconds.
#[derive(Serialize)]
struct Latency {
    latency: u64,
    jitter: u64,
}

/// Request body for `POST /proxies/{proxy}/toxics`, generic over the toxic's attributes.
#[derive(Serialize)]
struct Toxic<A> {
    #[serde(rename = "type")]
    kind: &'static str,
    stream: Stream,
    toxicity: f32,
    attributes: A,
}

impl Toxic<Latency> {
    /// A `latency` toxic adding a fixed delay (± jitter) in milliseconds.
    fn latency(stream: Stream, latency_ms: u64, jitter_ms: u64) -> Self {
        Self {
            kind: "latency",
            stream,
            toxicity: 1.0,
            attributes: Latency {
                latency: latency_ms,
                jitter: jitter_ms,
            },
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    // Start Toxiproxy and publish a port for our proxy to listen on.
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

    // Create a proxy. For a self-contained example we proxy to Toxiproxy's own HTTP API, reachable
    // at localhost:8474 from inside the container. In real tests point `upstream` at another
    // container (sharing a user-defined network) or at `host.docker.internal`.
    let proxy = Proxy {
        name: "example".to_string(),
        listen: format!("0.0.0.0:{PROXY_PORT}"),
        upstream: format!("localhost:{}", CONTROL_PORT.as_u16()),
        enabled: true,
    };
    http.post(format!("{api}/proxies"))
        .json(&proxy)
        .send()
        .await?
        .error_for_status()?;
    println!("created proxy '{}' -> {}", proxy.name, proxy.upstream);

    // Without toxics, a request flows straight through.
    let started = Instant::now();
    http.get(format!("{through_proxy}/version"))
        .send()
        .await?
        .error_for_status()?;
    println!(
        "request through proxy (no toxics) took {:?}",
        started.elapsed()
    );

    // Add a downstream latency toxic of ~1s and watch the same request slow down.
    let toxic = Toxic::latency(Stream::Downstream, 1000, 0);
    http.post(format!("{api}/proxies/{}/toxics", proxy.name))
        .json(&toxic)
        .send()
        .await?
        .error_for_status()?;
    println!("added latency toxic of {}ms", toxic.attributes.latency);

    let started = Instant::now();
    http.get(format!("{through_proxy}/version"))
        .send()
        .await?
        .error_for_status()?;
    let elapsed = started.elapsed();
    println!("request through proxy (with latency toxic) took {elapsed:?}");
    assert!(
        elapsed.as_millis() >= 900,
        "latency toxic should have delayed the request"
    );

    Ok(())
}
