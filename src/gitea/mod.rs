/// Self-hosted git server with https/http/ssh access, uses [Gitea](https://docs.gitea.com/).
use std::result::Result;

use rcgen::{BasicConstraints, CertificateParams, IsCa, KeyPair};
use testcontainers::{
    core::{
        wait::HttpWaitStrategy, CmdWaitFor, ContainerPort, ContainerState, ExecCommand, WaitFor,
    },
    CopyDataSource, CopyToContainer, Image, TestcontainersError,
};

/// Container port for SSH listener.
pub const GITEA_SSH_PORT: ContainerPort = ContainerPort::Tcp(2222);
/// Container port for HTTPS/HTTP listener.
pub const GITEA_HTTP_PORT: ContainerPort = ContainerPort::Tcp(3000);
/// Container port for HTTP listener to redirect call to HTTPS port.
pub const GITEA_HTTP_REDIRECT_PORT: ContainerPort = ContainerPort::Tcp(3080);

/// Default admin username.
pub const GITEA_DEFAULT_ADMIN_USERNAME: &str = "git-admin";
/// Default admin password.
pub const GITEA_DEFAULT_ADMIN_PASSWORD: &str = "git-admin";

/// Container folder where configuration and SSL certificates are stored to.
pub const GITEA_CONFIG_FOLDER: &str = "/etc/gitea";
/// Container folder with git data: repos, DB, etc.
pub const GITEA_DATA_FOLDER: &str = "/var/lib/gitea";

/// Docker hub registry with gitea image.
const GITEA_IMAGE_NAME: &str = "gitea/gitea";
/// Image tag to use.
const GITEA_IMAGE_TAG: &str = "1.22.3-rootless";

/// File name with SSL certificate.
const TLS_CERT_FILE_NAME: &str = "cert.pem";
/// File name with a private key for SSL certificate.
const TLS_KEY_FILE_NAME: &str = "key.pem";
/// File name with a Gitea config.
const CONFIG_FILE_NAME: &str = "app.ini";

/// Module to work with [Gitea](https://docs.gitea.com/) container.
///
/// Starts an instance of [`Gitea`](https://docs.gitea.com/), fully functional git server, with reasonable defaults
/// and possibility to tune some configuration options.
///
/// From the `Gitea` documentation:
/// _Gitea is a painless, self-hosted, all-in-one software development service.
/// It includes Git hosting, code review, team collaboration, package registry,
/// and CI/CD. It is similar to GitHub, Bitbucket and GitLab._
///
/// By default, `Gitea` server container starts with the following config:
/// - accepts SSH (Git) protocol requests on port [GITEA_SSH_PORT];
/// - accepts HTTP requests on port [GITEA_HTTP_PORT];
/// - has a single configured user with admin privileges,
///   with pre-defined [username](GITEA_DEFAULT_ADMIN_USERNAME) and [password](GITEA_DEFAULT_ADMIN_PASSWORD);
/// - configured git server hostname is `localhost`; this is a name which `Gitea` uses in the links to repositories;
/// - no repositories are created.
///
/// Additionally to defaults, it's possible to:
/// - use HTTPS instead of HTTP with auto-generated self-signed certificate or provide your own certificate;
/// - redirect HTTP calls to HTTPS listener, if HTTPS is enabled;
/// - change git server hostname, which is used in various links to repos or web-server;
/// - provide your own admin user credentials as well as its SSH public key to authorize git calls;
/// - create any number of public or private repositories with provided names during server startup;
/// - execute set of `gitea admin ...` commands during server startup to customize configuration;
/// - add environment variables
///
/// # Examples
///
/// 1. Minimalistic server
/// ```rust
/// use testcontainers::{runners::AsyncRunner, ImageExt};
/// use testcontainers_modules::gitea::{self, Gitea, GiteaRepo};
///
/// #[tokio::test]
/// async fn default_gitea_server() {
///     // Run default container
///     let gitea = Gitea::default().start().await.unwrap();
///     let port = gitea
///         .get_host_port_ipv4(gitea::GITEA_HTTP_PORT)
///         .await
///         .unwrap();
///     let url = format!(
///         "http://localhost:{port}/api/v1/users/{}",
///         gitea::GITEA_DEFAULT_ADMIN_USERNAME
///     );
///
///     // Anonymous query Gitea API for user info
///     let response = reqwest::Client::new().get(url).send().await.unwrap();
///     assert_eq!(response.status(), 200);
/// }
/// ```
///
/// 2. Customized server
/// ```rust
/// use testcontainers::{runners::AsyncRunner, ImageExt};
/// use testcontainers_modules::gitea::{self, Gitea, GiteaRepo};
///
/// #[tokio::test]
/// async fn gitea_server_with_custom_config() {
///     // Start server container with:
///     // - custom admin credentials
///     // - two repos: public and private
///     // - TLS enabled
///     // - port mapping for HTTP and SSH
///     // - custom git hostname
///     let gitea = Gitea::default()
///         .with_git_hostname("gitea.example.com")
///         .with_admin_account("custom-admin", "password", None)
///         .with_repo(GiteaRepo::Public("public-test-repo".to_string()))
///         .with_repo(GiteaRepo::Private("private-test-repo".to_string()))
///         .with_tls(true)
///         .with_mapped_port(443, gitea::GITEA_HTTP_PORT)
///         .with_mapped_port(22, gitea::GITEA_SSH_PORT)
///         .start()
///         .await
///         .unwrap();
///
///     // Obtain auto-created root CA certificate
///     let ca = gitea.image().tls_ca().unwrap();
///     let ca = reqwest::Certificate::from_pem(ca.as_bytes()).unwrap();
///     // Attach custom CA to the client
///     let client = reqwest::ClientBuilder::new()
///         .add_root_certificate(ca)
///         .build()
///         .unwrap();
///
///     // Get list of repos of particular user.
///     // This query should be authorized.
///     let response = client
///         .get("https://localhost/api/v1/user/repos")
///         .basic_auth("custom-admin", Some("password"))
///         .header("Host", "gitea.example.com")
///         .send()
///         .await
///         .unwrap();
///     assert_eq!(response.status(), 200);
///
///     let repos = response.json::<serde_json::Value>().await.unwrap();
///     assert_eq!(repos.as_array().unwrap().len(), 2);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Gitea {
    git_hostname: String,
    admin_username: String,
    admin_password: String,
    admin_key: Option<String>,
    admin_commands: Vec<Vec<String>>,
    tls: Option<GiteaTlsCert>,
    repos: Vec<GiteaRepo>,
    copy_to_sources: Vec<CopyToContainer>,
}

impl Default for Gitea {
    /// Returns default Gitea server setup with the following defaults:
    /// - hostname is `localhost`;
    /// - admin account username from [GITEA_DEFAULT_ADMIN_USERNAME];
    /// - admin account password from [GITEA_DEFAULT_ADMIN_PASSWORD];
    /// - without admins' account SSH public key;
    /// - without additional startup admin commands;
    /// - without TLS (SSH and HTTP protocols only);
    /// - without repositories.
    fn default() -> Self {
        Self {
            git_hostname: "localhost".to_string(),
            admin_username: GITEA_DEFAULT_ADMIN_USERNAME.to_string(),
            admin_password: GITEA_DEFAULT_ADMIN_PASSWORD.to_string(),
            admin_key: None,
            admin_commands: vec![],
            tls: None,
            repos: vec![],
            copy_to_sources: vec![Self::render_app_ini("http", "localhost", false)],
        }
    }
}

impl Image for Gitea {
    fn name(&self) -> &str {
        GITEA_IMAGE_NAME
    }

    fn tag(&self) -> &str {
        GITEA_IMAGE_TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        let http_check = match self.tls {
            Some(_) => WaitFor::seconds(5), // it's expensive to add reqwest dependency for the single health check only
            None => WaitFor::http(
                HttpWaitStrategy::new("/api/swagger")
                    .with_port(GITEA_HTTP_PORT)
                    .with_expected_status_code(200_u16),
            ),
        };

        vec![
            WaitFor::message_on_stdout(format!(
                "Starting new Web server: tcp:0.0.0.0:{}",
                GITEA_HTTP_PORT.as_u16()
            )),
            http_check,
        ]
    }

    fn copy_to_sources(&self) -> impl IntoIterator<Item = &CopyToContainer> {
        &self.copy_to_sources
    }

    fn expose_ports(&self) -> &[ContainerPort] {
        if self.tls.is_some() {
            // additional port for HTTP with redirect to HTTPS
            &[GITEA_SSH_PORT, GITEA_HTTP_PORT, GITEA_HTTP_REDIRECT_PORT]
        } else {
            &[GITEA_SSH_PORT, GITEA_HTTP_PORT]
        }
    }

    fn exec_after_start(
        &self,
        _cs: ContainerState,
    ) -> Result<Vec<ExecCommand>, TestcontainersError> {
        // Create admin user
        let mut start_commands = vec![self.create_admin_user_cmd()];
        // Add admins' public key if needed
        if let Some(key) = &self.admin_key {
            start_commands.push(self.create_admin_key_cmd(key));
        }
        // create repos if they're defined
        self.repos.iter().for_each(|r| {
            start_commands.push(self.create_repo_cmd(r));
        });

        // and finally, add `gitea admin` commands, if defined
        let admin_commands: Vec<Vec<String>> = self
            .admin_commands
            .clone()
            .into_iter()
            .map(|v| {
                vec!["gitea".to_string(), "admin".to_string()]
                    .into_iter()
                    .chain(v)
                    .collect::<Vec<String>>()
            })
            .collect();

        // glue everything togather
        start_commands.extend(admin_commands);

        // and convert to `ExecCommand`s
        let commands: Vec<ExecCommand> = start_commands
            .iter()
            .map(|v| ExecCommand::new(v).with_cmd_ready_condition(CmdWaitFor::exit_code(0)))
            .collect();

        Ok(commands)
    }
}

impl Gitea {
    /// Change admin user credential to the custom provided `username` and `password` instead of using defaults.
    ///
    /// If `public_key` value is provided, it will be added to the admin account.
    ///
    /// # Example
    /// ```rust,ignore
    /// #[tokio::test]
    /// async fn test() {
    ///     const PUB_KEY: &str = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJRE5a67/cTbR6DpWqzBl6BTY0LE0Hg715ZI/FMK7iCH";
    ///     let gitea = Gitea::default()
    ///             .with_admin_account("git-admin", "nKz4SC7bkz4KSXbQ", Some(PUB_KEY))
    ///             .start()
    ///             .await
    ///             .unwrap();
    /// // ...
    /// }
    /// ```
    pub fn with_admin_account(
        self,
        username: impl Into<String>,
        password: impl Into<String>,
        public_key: Option<String>,
    ) -> Self {
        Self {
            admin_username: username.into(),
            admin_password: password.into(),
            admin_key: public_key,
            ..self
        }
    }

    /// Set git server hostname instead of the default `localhost`.
    ///
    /// This is not a containers' hostname, but the name which git server uses in various links like repo URLs.
    pub fn with_git_hostname(self, hostname: impl Into<String>) -> Self {
        let new = Self {
            git_hostname: hostname.into(),
            ..self
        };
        Self {
            // to update app.ini
            copy_to_sources: new.generate_copy_to_sources(),
            ..new
        }
    }

    /// Create a repository during startup.
    ///
    /// It's possible to call this method more than once to create several repositories.
    ///
    /// # Example
    /// ```rust,ignore
    /// #[tokio::test]
    /// async fn test() {
    ///     let gitea = Gitea::default()
    ///             .with_repo(GiteaRepo::Public("example-public-repo"))
    ///             .with_repo(GiteaRepo::Private("example-private-repo"))
    ///             .start()
    ///             .await
    ///             .unwrap();
    /// // ...
    /// }
    /// ```
    pub fn with_repo(self, repo: GiteaRepo) -> Self {
        let mut repos = self.repos;
        repos.push(repo);
        Self { repos, ..self }
    }

    /// Add `gitea admin ...` command with parameters to execute after server startup.
    ///
    /// This method is useful, for example, to create additional users or to do other admin stuff.
    ///
    /// It's possible to call this method more than once to add several consecutive commands.
    ///
    /// # Example
    /// ```rust,ignore
    /// #[tokio::test]
    /// async fn test() {
    ///     let cmd = vec![
    ///          "user",
    ///          "create",
    ///          "--username",
    ///          "test-user",
    ///          "--password",
    ///          "test-password",
    ///          "--email",
    ///          "test@localhost",
    ///          "--must-change-password=true",
    ///          ]
    ///          .into_iter()
    ///          .map(String::from)
    ///          .collect::<Vec<String>>();
    ///
    ///     let gitea = Gitea::default()
    ///         .with_admin_command(command)
    ///         .start()
    ///         .await
    ///         .unwrap();
    /// // ...
    /// }
    /// ```
    pub fn with_admin_command(self, command: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let command = command
            .into_iter()
            .map(|s| s.into())
            .collect::<Vec<String>>();
        let mut admin_commands = self.admin_commands;

        admin_commands.push(command);
        Self {
            admin_commands,
            ..self
        }
    }

    /// `Gitea` web server will start with HTTPS listener (with auto-generated certificate),
    /// instead of the default HTTP.
    ///
    /// If `enabled` is `true,` web server will be started with TLS listener with auto-generated self-signed certificate.
    /// If Root CA certificate is needed to ensure fully protected communications,
    /// it can be obtained by [Gitea::tls_ca()] method call.
    ///
    /// Note: _If TLS is enabled, additional HTTP listener will be started on port [GITEA_HTTP_REDIRECT_PORT]
    /// to redirect all HTTP calls to the HTTPS listener._
    pub fn with_tls(self, enabled: bool) -> Self {
        let new = Self {
            tls: if enabled {
                Some(GiteaTlsCert::default())
            } else {
                None
            },
            ..self
        };

        Self {
            // to update app.ini and certificates
            copy_to_sources: new.generate_copy_to_sources(),
            ..new
        }
    }

    /// `Gitea` web server will start with HTTPS listener (with provided certificate), instead of the default HTTP.
    ///
    /// `cert` and `key` are strings with PEM encoded certificate and its key.
    /// This method is similar to [Gitea::with_tls()] but use provided certificate instead of generating self-signed one.
    ///
    /// Note: _If TLS is enabled, additional HTTP listener will be started on port [GITEA_HTTP_REDIRECT_PORT]
    /// to redirect all HTTP calls to the HTTPS listener._
    pub fn with_tls_certs(self, cert: impl Into<String>, key: impl Into<String>) -> Self {
        let new = Self {
            tls: Some(GiteaTlsCert::from_pem(cert.into(), key.into())),
            ..self
        };

        Self {
            // to update app.ini and certificates
            copy_to_sources: new.generate_copy_to_sources(),
            ..new
        }
    }

    /// Return PEM encoded Root CA certificate of the Gitea servers' certificate issuer.
    ///
    /// If TLS has been enabled using [Gitea::with_tls_certs()] method (with auto-generated self-signed certificate),
    /// then this method returns `Some` option with issuer root CA certificate to verify servers' certificate
    /// and ensure fully protected communications.
    ///
    /// If TLS isn't enabled or TLS is enabled with external certificate,
    /// provided using [Gitea::with_tls_certs] method,
    /// this method returns `None` since there is no known CA certificate.
    pub fn tls_ca(&self) -> Option<&str> {
        self.tls.as_ref().and_then(|t| t.ca())
    }

    /// Gather app.ini and certificates (if needed) into one vector to store into modules' structure.
    fn generate_copy_to_sources(&self) -> Vec<CopyToContainer> {
        let mut to_copy = vec![];

        // Prepare app.ini from template
        let app_ini = Self::render_app_ini(
            self.protocol(),
            self.git_hostname.as_str(),
            self.tls.is_some(),
        );
        to_copy.push(app_ini);

        // Add certificates if TLS is enabled
        if let Some(tls_config) = &self.tls {
            let cert = CopyToContainer::new(
                CopyDataSource::Data(tls_config.cert.clone().into_bytes()),
                format!("{GITEA_CONFIG_FOLDER}/{TLS_CERT_FILE_NAME}",),
            );
            let key = CopyToContainer::new(
                CopyDataSource::Data(tls_config.key.clone().into_bytes()),
                format!("{GITEA_CONFIG_FOLDER}/{TLS_KEY_FILE_NAME}",),
            );
            to_copy.push(cert);
            to_copy.push(key);
        }

        to_copy
    }

    /// Render app.ini content from the template using current config values.
    fn render_app_ini(protocol: &str, hostname: &str, is_tls: bool) -> CopyToContainer {
        let redirect_port = GITEA_HTTP_REDIRECT_PORT.as_u16();
        // load template of the app.ini,
        // `[server]` section should be at the bottom to add variable part
        // and TLS-related variables is needed
        let mut app_ini_template = include_str!("app.ini").to_string();
        let host_template_part = format!(
            r#"
DOMAIN = {hostname}
SSH_DOMAIN = {hostname}
ROOT_URL = {protocol}://{hostname}/
PROTOCOL = {protocol}
"#,
        );
        app_ini_template.push_str(&host_template_part);

        // If TLS is enabled, add TLS-related config to app.ini
        if is_tls {
            let tls_config = format!(
                r#"
CERT_FILE = {GITEA_CONFIG_FOLDER}/{TLS_CERT_FILE_NAME}
KEY_FILE = {GITEA_CONFIG_FOLDER}/{TLS_KEY_FILE_NAME}
REDIRECT_OTHER_PORT = true
PORT_TO_REDIRECT = {redirect_port}
"#
            );
            app_ini_template.push_str(&tls_config);
        }

        CopyToContainer::new(
            CopyDataSource::Data(app_ini_template.into_bytes()),
            format!("{GITEA_CONFIG_FOLDER}/{CONFIG_FILE_NAME}",),
        )
    }

    /// Generate command to create admin user with actual parameters.
    fn create_admin_user_cmd(&self) -> Vec<String> {
        vec![
            "gitea",
            "admin",
            "user",
            "create",
            "--username",
            self.admin_username.as_str(),
            "--password",
            self.admin_password.as_str(),
            "--email",
            format!("{}@localhost", self.admin_username).as_str(),
            "--admin",
        ]
        .into_iter()
        .map(String::from)
        .collect::<Vec<String>>()
    }

    /// Generate curl command with API call to add public key for admin user.
    fn create_admin_key_cmd(&self, key: &String) -> Vec<String> {
        let body = format!(r#"{{"title":"default","key":"{}","read_only":false}}"#, key);
        self.create_gitea_api_curl_cmd("POST", "/user/keys", Some(body))
    }

    /// Generate curl command with API call to create repository with minimal parameters.
    fn create_repo_cmd(&self, repo: &GiteaRepo) -> Vec<String> {
        let (repo, private) = match repo {
            GiteaRepo::Private(name) => (name, "true"),
            GiteaRepo::Public(name) => (name, "false"),
        };

        let body = format!(
            r#"{{"name":"{}","readme":"Default","auto_init":true,"private":{}}}"#,
            repo, private
        );

        self.create_gitea_api_curl_cmd("POST", "/user/repos", Some(body))
    }

    /// Helper to generate curl commands with API call.
    fn create_gitea_api_curl_cmd(
        &self,
        method: &str,
        api_path: &str,
        body: Option<String>,
    ) -> Vec<String> {
        let mut curl = vec![
            "curl",
            "-sk",
            "-X",
            method,
            "-H",
            "accept: application/json",
            "-H",
            "Content-Type: application/json",
            "-u",
            format!("{}:{}", self.admin_username, self.admin_password).as_str(),
        ]
        .into_iter()
        .map(String::from)
        .collect::<Vec<String>>();

        // add body if present
        if let Some(body) = body {
            curl.push("-d".to_string());
            curl.push(body);
        }

        // and finally, add url to API with a requested path
        curl.push(self.api_url(api_path));

        curl
    }

    /// Return configured protocol string.
    fn protocol(&self) -> &str {
        if self.tls.is_some() {
            "https"
        } else {
            "http"
        }
    }

    /// Return container-internal base URL to the API.
    fn api_url(&self, api: &str) -> String {
        let api = api.strip_prefix('/').unwrap_or(api);
        format!(
            "{}://localhost:{}/api/v1/{api}",
            self.protocol(),
            GITEA_HTTP_PORT.as_u16()
        )
    }
}

/// Defines repository to create during container startup.
///
/// Each option includes repository name in the enum value.
///
/// [`Gitea::with_repo`] documentation provides more details and usage examples.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GiteaRepo {
    /// Create a private repository which is accessible with authorization only.
    Private(String),
    /// Create a public repository accessible without authorization.
    Public(String),
}

/// Helper struct to store TLS certificates.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct GiteaTlsCert {
    cert: String,
    key: String,
    ca: Option<String>,
}

impl Default for GiteaTlsCert {
    fn default() -> Self {
        Self::new("localhost")
    }
}

impl GiteaTlsCert {
    /// Generate new self-signed Root CA certificate,
    /// and generate new server certificate signed by CA.
    ///
    /// SAN list includes "localhost", "127.0.0.1", "::1"
    /// and provided hostname (if it's different form localhost).
    fn new(hostname: impl Into<String>) -> Self {
        // generate root CA key and cert
        let ca_key = KeyPair::generate().unwrap();
        let mut ca_cert = CertificateParams::new(vec!["Gitea root CA".to_string()]).unwrap();
        ca_cert.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        let ca_cert = ca_cert.self_signed(&ca_key).unwrap();

        // prepare SANs
        let mut hostnames = vec![
            "localhost".to_string(),
            "127.0.0.1".to_string(),
            "::1".to_string(),
        ];
        let hostname = hostname.into();
        if hostname != "localhost" {
            hostnames.insert(0, hostname);
        }

        // and generate server key and cert
        let key = KeyPair::generate().unwrap();
        let cert = CertificateParams::new(hostnames)
            .unwrap()
            .signed_by(&key, &ca_cert, &ca_key)
            .unwrap();

        Self {
            cert: cert.pem(),
            key: key.serialize_pem(),
            ca: Some(ca_cert.pem()),
        }
    }

    /// Construct from externally provided certificate and key, without CA.
    fn from_pem(cert: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            cert: cert.into(),
            key: key.into(),
            ca: None,
        }
    }

    /// Return self-signed Root CA is it was generated.
    fn ca(&self) -> Option<&str> {
        self.ca.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use reqwest::Certificate;
    use serde_json::Value;
    use testcontainers::{runners::AsyncRunner, ContainerAsync};

    use super::*;

    const TEST_PUBLIC_KEY: &str =
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJRE5a67/cTbR6DpWqzBl6BTY0LE0Hg715ZI/FMK7iCH";
    const TEST_ADMIN_USERNAME: &str = "non-default-user";
    const TEST_ADMIN_PASSWORD: &str = "some-dummy-password";
    const TEST_PUBLIC_REPO: &str = "test-public-repo";
    const TEST_PRIVATE_REPO: &str = "test-private-repo";

    async fn api_url(container: &ContainerAsync<Gitea>, api: &str) -> String {
        let api = api.strip_prefix('/').unwrap_or(api);
        let host = container.get_host().await.unwrap();
        let port = container.get_host_port_ipv4(GITEA_HTTP_PORT).await.unwrap();

        format!(
            "{}://{host}:{port}/api/v1/{api}",
            container.image().protocol(),
        )
    }

    #[tokio::test]
    async fn gitea_defaults() {
        let gitea = Gitea::default().start().await.unwrap();

        // Check for admin user
        let response = reqwest::Client::new()
            .get(api_url(&gitea, &format!("/users/{GITEA_DEFAULT_ADMIN_USERNAME}")).await)
            .basic_auth(
                GITEA_DEFAULT_ADMIN_USERNAME,
                Some(GITEA_DEFAULT_ADMIN_PASSWORD),
            )
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);

        // Check for an admin user public key
        let keys_list = reqwest::Client::new()
            .get(api_url(&gitea, "/user/keys").await)
            .basic_auth(
                GITEA_DEFAULT_ADMIN_USERNAME,
                Some(GITEA_DEFAULT_ADMIN_PASSWORD),
            )
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap();

        let keys_list = keys_list.as_array().unwrap();
        assert!(keys_list.is_empty());
    }

    #[tokio::test]
    async fn gitea_with_tls() {
        let gitea = Gitea::default().with_tls(true).start().await.unwrap();

        // Check w/o CA, should fail
        let response = reqwest::Client::new()
            .get(api_url(&gitea, &format!("/users/{GITEA_DEFAULT_ADMIN_USERNAME}")).await)
            .basic_auth(
                GITEA_DEFAULT_ADMIN_USERNAME,
                Some(GITEA_DEFAULT_ADMIN_PASSWORD),
            )
            .send()
            .await;
        assert!(response.is_err());

        // Check with CA, should pass
        let ca = gitea.image().tls_ca().unwrap();
        let ca = Certificate::from_pem(ca.as_bytes()).unwrap();
        let client = reqwest::ClientBuilder::new()
            .add_root_certificate(ca)
            .build()
            .unwrap();

        let response = client
            .get(api_url(&gitea, &format!("/users/{GITEA_DEFAULT_ADMIN_USERNAME}")).await)
            .basic_auth(
                GITEA_DEFAULT_ADMIN_USERNAME,
                Some(GITEA_DEFAULT_ADMIN_PASSWORD),
            )
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
    }

    #[tokio::test]
    async fn gitea_custom_admin_credentials() {
        let gitea = Gitea::default()
            .with_admin_account(
                TEST_ADMIN_USERNAME,
                TEST_ADMIN_PASSWORD,
                Some(TEST_PUBLIC_KEY.to_string()),
            )
            .start()
            .await
            .unwrap();

        // Check for an admin user public key with default credentials,
        // fails since user doesn't exist
        let response = reqwest::Client::new()
            .get(api_url(&gitea, "/user/keys").await)
            .basic_auth(
                GITEA_DEFAULT_ADMIN_USERNAME,
                Some(GITEA_DEFAULT_ADMIN_PASSWORD),
            )
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 401);

        // The same check with custom credentials should pass
        let keys_list = reqwest::Client::new()
            .get(api_url(&gitea, "/user/keys").await)
            .basic_auth(TEST_ADMIN_USERNAME, Some(TEST_ADMIN_PASSWORD))
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap();

        let keys_list = keys_list.as_array().unwrap();
        assert_eq!(keys_list.len(), 1);
    }

    #[tokio::test]
    async fn gitea_create_repos() {
        let gitea = Gitea::default()
            .with_repo(GiteaRepo::Public(TEST_PUBLIC_REPO.to_string()))
            .with_repo(GiteaRepo::Private(TEST_PRIVATE_REPO.to_string()))
            .start()
            .await
            .unwrap();

        // Check access to the public repo w/o auth
        let response = reqwest::Client::new()
            .get(
                api_url(
                    &gitea,
                    &format!("/repos/{GITEA_DEFAULT_ADMIN_USERNAME}/{TEST_PUBLIC_REPO}"),
                )
                .await,
            )
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);

        // Check access to the private repo w/o auth,
        // should be 404
        let response = reqwest::Client::new()
            .get(
                api_url(
                    &gitea,
                    &format!("/repos/{GITEA_DEFAULT_ADMIN_USERNAME}/{TEST_PRIVATE_REPO}"),
                )
                .await,
            )
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 404);

        // Check access to the private repo with auth,
        // should be 200
        let response = reqwest::Client::new()
            .get(
                api_url(
                    &gitea,
                    &format!("/repos/{GITEA_DEFAULT_ADMIN_USERNAME}/{TEST_PRIVATE_REPO}"),
                )
                .await,
            )
            .basic_auth(
                GITEA_DEFAULT_ADMIN_USERNAME,
                Some(GITEA_DEFAULT_ADMIN_PASSWORD),
            )
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
    }

    #[tokio::test]
    async fn gitea_admin_commands() {
        let command = vec![
            "user",
            "create",
            "--username",
            TEST_ADMIN_USERNAME,
            "--password",
            TEST_ADMIN_PASSWORD,
            "--email",
            format!("{}@localhost", TEST_ADMIN_USERNAME).as_str(),
            "--must-change-password=false",
        ]
        .into_iter()
        .map(String::from)
        .collect::<Vec<String>>();

        let gitea = Gitea::default()
            .with_admin_command(command)
            .start()
            .await
            .unwrap();

        // Check for new custom user
        let response = reqwest::Client::new()
            .get(api_url(&gitea, &format!("/users/{TEST_ADMIN_USERNAME}")).await)
            .basic_auth(
                GITEA_DEFAULT_ADMIN_USERNAME,
                Some(GITEA_DEFAULT_ADMIN_PASSWORD),
            )
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200);

        // Check with users' credentials
        let response = reqwest::Client::new()
            .get(api_url(&gitea, "/user/emails").await)
            .basic_auth(TEST_ADMIN_USERNAME, Some(TEST_ADMIN_PASSWORD))
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap();

        let response = response.as_array().unwrap();
        assert_eq!(response.len(), 1);
    }
}
