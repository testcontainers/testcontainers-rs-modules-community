use std::{borrow::Cow, collections::HashMap};

use testcontainers::{
    core::{wait::HttpWaitStrategy, ContainerPort, WaitFor},
    Image,
};

const NAME: &str = "ghcr.io/zitadel/zitadel";
const TAG: &str = "v3.0.0-rc.2";
const DEFAULT_MASTER_KEY: &str = "MasterkeyNeedsToHave32Characters";

/// Port that the [`Zitadel`] container has internally
/// Can be rebound externally via [`testcontainers::core::ImageExt::with_mapped_port`]
///
/// [`Zitadel`]: https://zitadel.com/
pub const ZITADEL_PORT: ContainerPort = ContainerPort::Tcp(8080);

// Zitadel testcontainer module
//
// This module provides a [Zitadel] container that can be used for testing purposes.
// It uses the official Zitadel Docker image and configures it with a default master key
// and in-memory database for quick testing.
//
// # Example
// ```rust,no_run
// use testcontainers_modules::zitadel::Zitadel;
//
// #[tokio::main]
// async fn main() {
//     let zitadel = Zitadel::default().start().await?;
//     let host_port = zitadel_node.get_host_port_ipv4(zitadel::ZITADEL_PORT).await?;
//     println!("Zitadel is running on port: {}", host_port);
// }
// ```

/// Configuration for the Zitadel container.
#[derive(Debug, Clone, Default)]
pub struct Zitadel {
    env_vars: HashMap<String, String>,
}

impl Zitadel {
    // Helper function to convert bool to "true" or "false" string
    fn bool_to_string(value: bool) -> String {
        (if value { "true" } else { "false" }).to_owned()
    }

    // https://zitadel.com/docs/self-hosting/manage/configure

    /// Configures external secure for the [`Zitadel`] instance.
    /// ExternalSecure specifies if ZITADEL is exposed externally using HTTPS or HTTP.
    /// Read more about external access: https://zitadel.com/docs/self-hosting/manage/custom-domain
    /// Default is `true` if not overridden by this function
    ///
    /// See the [official docs for this option](https://zitadel.com/docs/self-hosting/manage/configure)
    pub fn with_external_secure(mut self, external_secure: bool) -> Self {
        self.env_vars.insert(
            "ZITADEL_EXTERNALSECURE".to_owned(),
            Self::bool_to_string(external_secure),
        );
        self
    }

    /// Sets the Postgres database for the Zitadel instance.
    pub fn with_postgres_database(
        mut self,
        host: Option<String>,
        port: Option<u16>,
        database: Option<String>,
    ) -> Self {
        match host {
            Some(host) => self
                .env_vars
                .insert("ZITADEL_DATABASE_POSTGRES_HOST".to_owned(), host.to_owned()),
            None => self.env_vars.remove("ZITADEL_DATABASE_POSTGRES_HOST"),
        };
        match port {
            Some(port) => self.env_vars.insert(
                "ZITADEL_DATABASE_POSTGRES_PORT".to_owned(),
                port.to_string(),
            ),
            None => self.env_vars.remove("ZITADEL_DATABASE_POSTGRES_PORT"),
        };
        match database {
            Some(database) => self.env_vars.insert(
                "ZITADEL_DATABASE_POSTGRES_DATABASE".to_owned(),
                database.to_owned(),
            ),
            None => self.env_vars.remove("ZITADEL_DATABASE_POSTGRES_DATABASE"),
        };
        self
    }

    /// Sets the Postgres database user for the Zitadel instance.
    pub fn with_postgres_database_user(
        mut self,
        username: Option<String>,
        password: Option<String>,
        ssl_mode: Option<String>,
    ) -> Self {
        match username {
            Some(username) => self.env_vars.insert(
                "ZITADEL_DATABASE_POSTGRES_USER_USERNAME".to_owned(),
                username.to_owned(),
            ),
            None => self
                .env_vars
                .remove("ZITADEL_DATABASE_POSTGRES_USER_USERNAME"),
        };
        match password {
            Some(password) => self.env_vars.insert(
                "ZITADEL_DATABASE_POSTGRES_USER_PASSWORD".to_owned(),
                password.to_owned(),
            ),
            None => self
                .env_vars
                .remove("ZITADEL_DATABASE_POSTGRES_USER_PASSWORD"),
        };
        match ssl_mode {
            Some(ssl_mode) => self.env_vars.insert(
                "ZITADEL_DATABASE_POSTGRES_USER_SSL_MODE".to_owned(),
                ssl_mode.to_owned(),
            ),
            None => self
                .env_vars
                .remove("ZITADEL_DATABASE_POSTGRES_USER_SSL_MODE"),
        };
        self
    }

    /// Sets the Postgres database admin for the Zitadel instance.
    pub fn with_postgres_database_admin(
        mut self,
        username: Option<String>,
        password: Option<String>,
        ssl_mode: Option<String>,
    ) -> Self {
        match username {
            Some(username) => self.env_vars.insert(
                "ZITADEL_DATABASE_POSTGRES_ADMIN_USERNAME".to_owned(),
                username.to_owned(),
            ),
            None => self
                .env_vars
                .remove("ZITADEL_DATABASE_POSTGRES_ADMIN_USERNAME"),
        };
        match password {
            Some(password) => self.env_vars.insert(
                "ZITADEL_DATABASE_POSTGRES_ADMIN_PASSWORD".to_owned(),
                password.to_owned(),
            ),
            None => self
                .env_vars
                .remove("ZITADEL_DATABASE_POSTGRES_ADMIN_PASSWORD"),
        };
        match ssl_mode {
            Some(ssl_mode) => self.env_vars.insert(
                "ZITADEL_DATABASE_POSTGRES_ADMIN_SSL_MODE".to_owned(),
                ssl_mode.to_owned(),
            ),
            None => self
                .env_vars
                .remove("ZITADEL_DATABASE_POSTGRES_ADMIN_SSL_MODE"),
        };
        self
    }
}

impl Image for Zitadel {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![
            WaitFor::message_on_stderr("server is listening on"),
            WaitFor::http(
                HttpWaitStrategy::new("/debug/healthz")
                    .with_port(ZITADEL_PORT)
                    .with_expected_status_code(200_u16)
                    .with_body("ok".as_bytes()),
            ),
        ]
    }

    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        &self.env_vars
    }

    fn cmd(&self) -> impl IntoIterator<Item = impl Into<Cow<'_, str>>> {
        [
            "start-from-init",
            "--masterkey",
            DEFAULT_MASTER_KEY,
            "--tlsMode",
            "disabled",
        ]
    }

    fn expose_ports(&self) -> &[ContainerPort] {
        &[ZITADEL_PORT]
    }
}
