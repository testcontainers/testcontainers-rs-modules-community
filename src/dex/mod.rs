mod config;

use std::borrow::Cow;

pub use config::{PrivateClient, User};
use testcontainers::{
    core::{
        error::Result, wait::HttpWaitStrategy, ContainerPort, ContainerState, ExecCommand, WaitFor,
    },
    Image,
};

use crate::dex::config::OAuth2;

const NAME: &str = "dexidp/dex";
const TAG: &str = "v2.41.1";
const HTTP_PORT: ContainerPort = ContainerPort::Tcp(5556);

const CONFIG_FILE: &str = "/etc/dex/config.docker.json";

/// Module to work with [`Dex`] inside of tests.
///
/// Dex is a lightweight [`OpenID Connect`] provider.
/// Uses the official [`Dex docker image`].
///
/// Dex's HTTP endpoint exposed at the port 5556.
///
/// # Example
/// ```
/// use testcontainers::runners::SyncRunner;
/// use testcontainers_modules::dex;
///
/// let dex = dex::Dex::default()
///     .with_simple_user()
///     .with_simple_client()
///     .start()
///     .unwrap();
/// let port = dex.get_host_port_ipv4(5556).unwrap();
/// ```
///
/// [`Dex`]: https://dexidp.io/
/// [`Dex docker image`]: https://hub.docker.com/r/dexidp/dex
/// [`OpenID Connect`]: https://openid.net/developers/how-connect-works/
pub struct Dex {
    tag: String,
    clients: Vec<PrivateClient>,
    users: Vec<User>,
    allow_password_grants: bool,
}

impl Default for Dex {
    fn default() -> Self {
        Self {
            tag: TAG.to_string(),
            clients: vec![],
            users: vec![],
            allow_password_grants: false,
        }
    }
}

impl Dex {
    /// Overrides the image tag.
    /// Check https://hub.docker.com/r/dexidp/dex/tags to see available tags.
    pub fn with_tag(self, tag: String) -> Self {
        Self { tag, ..self }
    }

    /// Appends a user with
    /// - E-Mail: `user@example.org`
    /// - Username: `user`
    /// - Password: `user`
    /// - User ID: `user`
    ///
    /// Users can only be added before the container starts.
    pub fn with_simple_user(self) -> Self {
        self.with_user(User::simple_user())
    }

    /// Appends the specified user.
    ///
    /// Users can only be added before the container starts.
    pub fn with_user(self, user: User) -> Self {
        Self {
            users: self.users.into_iter().chain(vec![user]).collect(),
            ..self
        }
    }

    /// Appends a client with
    /// - Id: `client`
    /// - Redirect URI: `http://localhost/oidc-callback`
    /// - Secret: `secret`
    ///
    /// Clients can only be added before the container starts.
    pub fn with_simple_client(self) -> Self {
        self.with_client(PrivateClient::simple_client())
    }

    /// Appends the specified client.
    /// Clients can only be added before the container starts.
    pub fn with_client(self, client: PrivateClient) -> Self {
        Self {
            clients: self.clients.into_iter().chain(vec![client]).collect(),
            ..self
        }
    }

    /// Enables grant_type 'password' (usually for testing purposes)
    pub fn with_allow_password_grants(self) -> Self {
        Self {
            allow_password_grants: true,
            ..self
        }
    }
}

impl Dex {
    fn generate_config(&self, host: &str, host_port: u16) -> ExecCommand {
        let config = config::Config {
            issuer: String::from(format!("http://{}:{}", host, host_port)),
            enable_password_db: true,
            storage: config::Storage::sqlite(),
            web: config::Web::http(),
            static_clients: self.clients.clone(),
            static_passwords: self.users.clone(),
            oauth2: if !self.allow_password_grants {
                None
            } else {
                Some(OAuth2::allow_password_grant())
            },
        };

        let config = serde_json::to_string(&config)
            .expect("Parsing should only fail if structs were defined incorrectly.");

        ExecCommand::new(vec![
            "/bin/sh",
            "-c",
            &format!("echo '{}' > {}", config, CONFIG_FILE),
        ])
    }
}

impl Image for Dex {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        &self.tag
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::Http(
            HttpWaitStrategy::new("/.well-known/openid-configuration")
                .with_port(HTTP_PORT)
                .with_expected_status_code(200u16),
        )]
    }

    fn cmd(&self) -> impl IntoIterator<Item = impl Into<Cow<'_, str>>> {
        // Stolen from the Java implementation:
        // https://github.com/Kehrlann/testcontainers-dex/tree/main/testcontainers-dex/src/main/java/wf/garnier/testcontainers/dexidp/DexContainer.java#L116
        let command = format!(
            r#"while [[ ! -f {CONFIG_FILE} ]]; do sleep 1; echo "Waiting for configuration file..."; done;
            dex serve {CONFIG_FILE}"#,
        );
        vec![String::from("/bin/sh"), String::from("-c"), command]
    }

    fn expose_ports(&self) -> &[ContainerPort] {
        &[HTTP_PORT]
    }

    fn exec_before_ready(&self, cs: ContainerState) -> Result<Vec<ExecCommand>> {
        let host = cs.host();
        let port = cs.host_port_ipv4(HTTP_PORT)?;
        Ok(vec![self.generate_config(&host.to_string(), port)])
    }
}

#[cfg(test)]
mod tests {
    use super::Dex;

    #[tokio::test]
    async fn starts_with_async_runner() {
        use testcontainers::runners::AsyncRunner;
        Dex::default().with_simple_user().start().await.unwrap();
    }

    #[test]
    fn starts_with_sync_runner() {
        use testcontainers::runners::SyncRunner;
        Dex::default().with_simple_user().start().unwrap();
    }

    #[tokio::test]
    async fn starts_without_users_and_client() {
        use testcontainers::runners::AsyncRunner;
        Dex::default().start().await.unwrap();
    }

    #[tokio::test]
    async fn can_authenticate() {
        use testcontainers::runners::AsyncRunner;
        let dex = Dex::default()
            .with_simple_user()
            .with_simple_client()
            .with_allow_password_grants()
            .start()
            .await
            .unwrap();
        let request = reqwest::Client::new();
        let url = format!(
            "http://{}:{}/token",
            dex.get_host().await.unwrap(),
            dex.get_host_port_ipv4(5556).await.unwrap()
        );
        let token = request
            .post(url)
            .header("Authorization", "Basic Y2xpZW50OnNlY3JldA==")
            .form(&[
                ("grant_type", "password"),
                ("scope", "openid"),
                ("username", "user@example.org"),
                ("password", "user"),
            ])
            .send()
            .await
            .unwrap();
        assert!(token.status().is_success());
        assert!(token
            .text()
            .await
            .unwrap()
            .starts_with("{\"access_token\":"));
    }
}
