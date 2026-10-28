use std::{borrow::Cow, fmt};

#[cfg(feature = "blocking")]
use testcontainers::Container;
use testcontainers::{
    core::{wait::HttpWaitStrategy, ContainerPort, WaitFor},
    ContainerAsync, Image, TestcontainersError,
};

const NAME: &str = "rustfs/rustfs";
const TAG: &str = "1.0.4";

/// Port used by the S3-compatible API inside the container.
pub const RUSTFS_API_PORT: ContainerPort = ContainerPort::Tcp(9000);
/// Port used by the web console inside the container.
pub const RUSTFS_CONSOLE_PORT: ContainerPort = ContainerPort::Tcp(9001);
const ALL_PORTS: [ContainerPort; 2] = [RUSTFS_API_PORT, RUSTFS_CONSOLE_PORT];
const DEFAULT_ACCESS_KEY: &str = "rustfsadmin";
const DEFAULT_SECRET_KEY: &str = "rustfsadmin";
const DEFAULT_REGION: &str = "us-east-1";

/// The access key and secret key used by an S3 client to authenticate with RustFS.
#[derive(Clone, PartialEq, Eq)]
pub struct RustFSCredentials {
    access_key: String,
    secret_key: String,
}

impl fmt::Debug for RustFSCredentials {
    /// Formats credentials with the secret key redacted.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RustFSCredentials")
            .field("access_key", &self.access_key)
            .field("secret_key", &"[REDACTED]")
            .finish()
    }
}

impl RustFSCredentials {
    /// Creates a credential pair for a throwaway RustFS test container.
    ///
    /// Avoid `/` in the access key. SigV4 uses `/` to separate the fields of the
    /// credential scope, so S3 clients may fail to sign requests with such a key.
    pub fn new(access_key: impl Into<String>, secret_key: impl Into<String>) -> Self {
        Self {
            access_key: access_key.into(),
            secret_key: secret_key.into(),
        }
    }

    /// Returns the S3 access key.
    pub fn access_key(&self) -> &str {
        &self.access_key
    }

    /// Returns the S3 secret key.
    pub fn secret_key(&self) -> &str {
        &self.secret_key
    }
}

impl Default for RustFSCredentials {
    /// Returns the RustFS well-known local-test credentials.
    fn default() -> Self {
        Self::new(DEFAULT_ACCESS_KEY, DEFAULT_SECRET_KEY)
    }
}

/// Module to work with [`RustFS`] inside of tests.
///
/// Starts an instance of RustFS based on the official [`RustFS docker image`].
///
/// RustFS is a high-performance object storage server compatible with Amazon S3 APIs.
/// The container exposes [`RUSTFS_API_PORT`] and, by default, [`RUSTFS_CONSOLE_PORT`].
/// It is ready when the S3 API returns HTTP 200 from `/health/ready`, which RustFS serves
/// once storage, IAM, and locking are initialized.
///
/// The readiness wait requires RustFS 1.0.0 or later, where `/health/ready` is served
/// without authentication. Setting `RUSTFS_HEALTH_ENDPOINT_ENABLE=false` disables the
/// endpoint, so the container never reports ready and startup fails at the startup timeout.
///
/// Configure S3 clients for path-style addressing unless [`RustFS::with_server_domains`] is set.
/// The default credentials are RustFS's well-known local-test values; set a unique pair
/// with [`RustFS::with_credentials`] when starting a container.
///
/// # Example
/// ```
/// use testcontainers_modules::{
///     rustfs::{RustFS, RustFSConnectionInfo},
///     testcontainers::runners::AsyncRunner,
/// };
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let rustfs_instance = RustFS::default()
///     .with_credentials("test-access-key", "test-secret-key")
///     .start()
///     .await?;
/// let connection = RustFSConnectionInfo::from_async_container(&rustfs_instance).await?;
///
/// // Configure an S3 client with connection.s3_endpoint(), connection.credentials(),
/// // connection.region(), and path-style addressing.
/// # Ok(())
/// # }
/// ```
///
/// [`RustFS`]: https://rustfs.com/
/// [`RustFS docker image`]: https://hub.docker.com/r/rustfs/rustfs
#[derive(Debug, Clone)]
pub struct RustFS {
    credentials: RustFSCredentials,
    region: String,
    console_enabled: bool,
    server_domains: Vec<String>,
}

impl Default for RustFS {
    /// Returns a RustFS image with default credentials, region `us-east-1`,
    /// the console enabled, and no server domains.
    fn default() -> Self {
        Self {
            credentials: RustFSCredentials::default(),
            region: DEFAULT_REGION.to_owned(),
            console_enabled: true,
            server_domains: Vec::new(),
        }
    }
}

impl RustFS {
    /// Sets the access key and secret key together for S3 authentication.
    ///
    /// See [`RustFSCredentials::new`] for key format notes.
    pub fn with_credentials(
        mut self,
        access_key: impl Into<String>,
        secret_key: impl Into<String>,
    ) -> Self {
        self.credentials = RustFSCredentials::new(access_key, secret_key);
        self
    }

    /// Returns the credentials configured for the image.
    pub fn credentials(&self) -> &RustFSCredentials {
        &self.credentials
    }

    /// Sets the region reported to S3 clients. The default is `us-east-1`.
    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = region.into();
        self
    }

    /// Returns the region configured for the image.
    pub fn region(&self) -> &str {
        &self.region
    }

    /// Enables or disables the web console, enabled by default.
    /// The image may still map port 9001 when the console listener is disabled.
    pub fn with_console_enabled(mut self, enabled: bool) -> Self {
        self.console_enabled = enabled;
        self
    }

    /// Returns whether the web console is enabled.
    pub fn console_enabled(&self) -> bool {
        self.console_enabled
    }

    /// Sets domains used for virtual-hosted-style S3 requests.
    ///
    /// Clients must resolve bucket subdomains to the container host. Path-style requests
    /// work without this setting.
    pub fn with_server_domains(
        mut self,
        domains: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.server_domains = domains.into_iter().map(Into::into).collect();
        self
    }

    /// Returns the configured S3 server domains.
    pub fn server_domains(&self) -> &[String] {
        &self.server_domains
    }
}

/// Connection settings for a running RustFS container.
///
/// Use the endpoint and credentials with an S3 client configured for path-style addressing.
/// These settings reflect the typed [`RustFS`] configuration. Raw `ImageExt::with_env_var`
/// overrides are not reflected here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustFSConnectionInfo {
    s3_endpoint: String,
    console_endpoint: Option<String>,
    credentials: RustFSCredentials,
    region: String,
}

impl RustFSConnectionInfo {
    /// Reads connection settings from an asynchronously started RustFS container.
    pub async fn from_async_container(
        container: &ContainerAsync<RustFS>,
    ) -> Result<Self, TestcontainersError> {
        let image = container.image();
        let host = container.get_host().await?;
        let api_port = container.get_host_port_ipv4(RUSTFS_API_PORT).await?;
        let console_port = if image.console_enabled() {
            Some(container.get_host_port_ipv4(RUSTFS_CONSOLE_PORT).await?)
        } else {
            None
        };

        Ok(Self::new(image, host, api_port, console_port))
    }

    /// Reads connection settings from a synchronously started RustFS container.
    #[cfg(feature = "blocking")]
    pub fn from_container(container: &Container<RustFS>) -> Result<Self, TestcontainersError> {
        let image = container.image();
        let host = container.get_host()?;
        let api_port = container.get_host_port_ipv4(RUSTFS_API_PORT)?;
        let console_port = if image.console_enabled() {
            Some(container.get_host_port_ipv4(RUSTFS_CONSOLE_PORT)?)
        } else {
            None
        };

        Ok(Self::new(image, host, api_port, console_port))
    }

    /// Builds connection info from typed image settings and mapped host ports.
    fn new(
        image: &RustFS,
        host: impl fmt::Display,
        api_port: u16,
        console_port: Option<u16>,
    ) -> Self {
        Self {
            s3_endpoint: format!("http://{host}:{api_port}"),
            console_endpoint: console_port.map(|port| format!("http://{host}:{port}")),
            credentials: image.credentials().clone(),
            region: image.region().to_owned(),
        }
    }

    /// Returns the HTTP URL for the S3-compatible API.
    pub fn s3_endpoint(&self) -> &str {
        &self.s3_endpoint
    }

    /// Returns the HTTP URL for the web console, if enabled.
    pub fn console_endpoint(&self) -> Option<&str> {
        self.console_endpoint.as_deref()
    }

    /// Returns the configured S3 credentials.
    pub fn credentials(&self) -> &RustFSCredentials {
        &self.credentials
    }

    /// Returns the configured S3 region.
    pub fn region(&self) -> &str {
        &self.region
    }
}

impl Image for RustFS {
    /// Returns the Docker Hub image name `rustfs/rustfs`.
    fn name(&self) -> &str {
        NAME
    }

    /// Returns the pinned image tag.
    fn tag(&self) -> &str {
        TAG
    }

    /// Waits until `GET /health/ready` on the API port returns HTTP 200.
    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::http(
            HttpWaitStrategy::new("/health/ready")
                .with_port(RUSTFS_API_PORT)
                .with_expected_status_code(200_u16),
        )]
    }

    /// Exposes the S3 API and console ports.
    fn expose_ports(&self) -> &[ContainerPort] {
        &ALL_PORTS
    }

    /// Returns environment variables derived from the typed configuration.
    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        let mut vars = vec![
            (
                Cow::Borrowed("RUSTFS_ADDRESS"),
                Cow::Owned(format!(":{}", RUSTFS_API_PORT.as_u16())),
            ),
            (
                Cow::Borrowed("RUSTFS_CONSOLE_ADDRESS"),
                Cow::Owned(format!(":{}", RUSTFS_CONSOLE_PORT.as_u16())),
            ),
            (
                Cow::Borrowed("RUSTFS_ACCESS_KEY"),
                Cow::Borrowed(self.credentials.access_key()),
            ),
            (
                Cow::Borrowed("RUSTFS_SECRET_KEY"),
                Cow::Borrowed(self.credentials.secret_key()),
            ),
            (Cow::Borrowed("RUSTFS_REGION"), Cow::Borrowed(self.region())),
            (
                Cow::Borrowed("RUSTFS_CONSOLE_ENABLE"),
                Cow::Borrowed(if self.console_enabled {
                    "true"
                } else {
                    "false"
                }),
            ),
        ];
        if !self.server_domains.is_empty() {
            vars.push((
                Cow::Borrowed("RUSTFS_SERVER_DOMAINS"),
                Cow::Owned(self.server_domains.join(",")),
            ));
        }
        vars
    }

    /// Passes `/data` as the volume argument to the RustFS process.
    fn cmd(&self) -> impl IntoIterator<Item = impl Into<Cow<'_, str>>> {
        ["/data"]
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, net::ToSocketAddrs, time::Duration};

    use aws_config::BehaviorVersion;
    use aws_sdk_s3::{
        config::{Credentials, Region},
        presigning::PresigningConfig,
        primitives::ByteStream,
        Client,
    };
    use testcontainers::runners::AsyncRunner;

    use super::*;

    /// Checks that typed builders map to env vars, exposed ports, and redacted secrets.
    #[test]
    fn typed_configuration_sets_environment_and_ports() {
        let rustfs = RustFS::default()
            .with_credentials("access-key", "secret-key")
            .with_region("eu-west-1")
            .with_console_enabled(false)
            .with_server_domains(["s3.example.com", "s3.example.org"]);
        let env: HashMap<String, String> = rustfs
            .env_vars()
            .into_iter()
            .map(|(key, value)| (key.into().into_owned(), value.into().into_owned()))
            .collect();

        assert_eq!(env["RUSTFS_ADDRESS"], ":9000");
        assert_eq!(env["RUSTFS_CONSOLE_ADDRESS"], ":9001");
        assert_eq!(env["RUSTFS_ACCESS_KEY"], "access-key");
        assert_eq!(env["RUSTFS_SECRET_KEY"], "secret-key");
        assert_eq!(env["RUSTFS_REGION"], "eu-west-1");
        assert_eq!(env["RUSTFS_CONSOLE_ENABLE"], "false");
        assert_eq!(
            env["RUSTFS_SERVER_DOMAINS"],
            "s3.example.com,s3.example.org"
        );
        assert_eq!(rustfs.expose_ports(), &ALL_PORTS);
        assert_eq!(RustFS::default().expose_ports(), &ALL_PORTS);
        assert!(!format!("{:?}", rustfs).contains("secret-key"));
    }

    /// Starts RustFS with the console disabled and lists buckets over the S3 API.
    #[tokio::test]
    async fn rustfs_without_console() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let node = RustFS::default()
            .with_credentials("console-off-access", "console-off-secret")
            .with_region("eu-west-1")
            .with_console_enabled(false)
            .start()
            .await?;

        let connection = RustFSConnectionInfo::from_async_container(&node).await?;
        assert_eq!(connection.region(), "eu-west-1");
        assert!(connection.console_endpoint().is_none());
        let client = build_s3_client(&connection).await;
        client.list_buckets().send().await?;
        Ok(())
    }

    /// Creates a bucket and round-trips a put/get object through the S3 API.
    #[tokio::test]
    async fn rustfs_buckets() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let rustfs = RustFS::default().with_credentials("test-access-key", "test-secret-key");
        let node = rustfs.start().await?;

        let connection = RustFSConnectionInfo::from_async_container(&node).await?;
        assert_eq!(connection.credentials().access_key(), "test-access-key");
        assert_eq!(connection.region(), DEFAULT_REGION);
        assert!(connection.console_endpoint().is_some());
        let client = build_s3_client(&connection).await;

        let bucket_name = "test-bucket";

        client
            .create_bucket()
            .bucket(bucket_name)
            .send()
            .await
            .expect("Failed to create test bucket");

        let buckets = client
            .list_buckets()
            .send()
            .await
            .expect("Failed to get list of buckets")
            .buckets
            .unwrap();

        let bucket_exists = buckets
            .iter()
            .any(|b| b.name.as_deref() == Some(bucket_name));
        assert!(bucket_exists, "Bucket {} not found", bucket_name);

        client
            .put_object()
            .bucket(bucket_name)
            .key("hello.txt")
            .body(ByteStream::from_static(b"Hello RustFS"))
            .send()
            .await?;
        let object = client
            .get_object()
            .bucket(bucket_name)
            .key("hello.txt")
            .send()
            .await?;
        assert_eq!(object.body.collect().await?.into_bytes(), "Hello RustFS");
        Ok(())
    }

    /// Serves a virtual-hosted-style GET after configuring `with_server_domains`.
    #[tokio::test]
    async fn rustfs_virtual_hosted_style_with_server_domains(
    ) -> Result<(), Box<dyn std::error::Error + 'static>> {
        let domain = "s3.rustfs.test";
        let bucket_name = "virtual-host-bucket";
        let node = RustFS::default()
            .with_credentials("vhost-access-key", "vhost-secret-key")
            .with_server_domains([domain])
            .start()
            .await?;

        let connection = RustFSConnectionInfo::from_async_container(&node).await?;
        let path_style_client = build_s3_client(&connection).await;
        path_style_client
            .create_bucket()
            .bucket(bucket_name)
            .send()
            .await?;
        path_style_client
            .put_object()
            .bucket(bucket_name)
            .key("hello.txt")
            .body(ByteStream::from_static(b"Hello virtual host"))
            .send()
            .await?;

        let host = node.get_host().await?.to_string();
        let api_port = node.get_host_port_ipv4(RUSTFS_API_PORT).await?;
        let api_addr = (host.as_str(), api_port)
            .to_socket_addrs()?
            .find(|addr| addr.is_ipv4())
            .ok_or("container host has no IPv4 address")?;

        let virtual_host_client =
            build_s3_client_with(&connection, &format!("http://{domain}:{api_port}"), false).await;
        let request = virtual_host_client
            .get_object()
            .bucket(bucket_name)
            .key("hello.txt")
            .presigned(PresigningConfig::expires_in(Duration::from_secs(300))?)
            .await?;
        let bucket_host = format!("{bucket_name}.{domain}");
        assert!(
            request
                .uri()
                .starts_with(&format!("http://{bucket_host}:{api_port}/hello.txt?")),
            "unexpected presigned URL {}",
            request.uri()
        );

        let http = reqwest::Client::builder()
            .no_proxy()
            .resolve(&bucket_host, api_addr)
            .build()?;
        let response = http.get(request.uri()).send().await?;
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        assert_eq!(response.text().await?, "Hello virtual host");
        Ok(())
    }

    /// Checks `from_container` endpoints, credentials, region, and `/health/ready`.
    #[cfg(feature = "blocking")]
    #[test]
    fn rustfs_connection_info_from_sync_container(
    ) -> Result<(), Box<dyn std::error::Error + 'static>> {
        use testcontainers::runners::SyncRunner;

        let image = RustFS::default()
            .with_credentials("sync-access-key", "sync-secret-key")
            .with_region("eu-central-1");
        let node = SyncRunner::start(image)?;

        let connection = RustFSConnectionInfo::from_container(&node)?;
        let host = node.get_host()?;
        let api_port = node.get_host_port_ipv4(RUSTFS_API_PORT)?;
        let console_port = node.get_host_port_ipv4(RUSTFS_CONSOLE_PORT)?;
        assert_eq!(
            connection.s3_endpoint(),
            format!("http://{host}:{api_port}")
        );
        assert_eq!(
            connection.console_endpoint(),
            Some(format!("http://{host}:{console_port}").as_str())
        );
        assert_eq!(connection.credentials().access_key(), "sync-access-key");
        assert_eq!(connection.credentials().secret_key(), "sync-secret-key");
        assert_eq!(connection.region(), "eu-central-1");

        let response = reqwest::blocking::Client::builder()
            .no_proxy()
            .build()?
            .get(format!("{}/health/ready", connection.s3_endpoint()))
            .send()?;
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        Ok(())
    }

    /// Builds a path-style S3 client from connection info.
    async fn build_s3_client(connection: &RustFSConnectionInfo) -> Client {
        build_s3_client_with(connection, connection.s3_endpoint(), true).await
    }

    /// Builds an S3 client for the given endpoint and path-style setting.
    async fn build_s3_client_with(
        connection: &RustFSConnectionInfo,
        endpoint: &str,
        force_path_style: bool,
    ) -> Client {
        let region = Region::new(connection.region().to_owned());
        let creds = Credentials::new(
            connection.credentials().access_key(),
            connection.credentials().secret_key(),
            None,
            None,
            "test",
        );

        let shared_config = aws_config::defaults(BehaviorVersion::latest())
            .region(region)
            .endpoint_url(endpoint)
            .credentials_provider(creds)
            .load()
            .await;

        let s3_config = aws_sdk_s3::config::Builder::from(&shared_config)
            .force_path_style(force_path_style)
            .build();
        Client::from_conf(s3_config)
    }
}
