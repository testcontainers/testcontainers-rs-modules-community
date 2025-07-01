use std::{borrow::Cow, collections::HashMap};

use testcontainers::{
    core::{ContainerPort, WaitFor},
    Image,
};

const NAME: &str = "surrealdb/surrealdb";
const TAG: &str = "v2.2";

/// Port that the [`SurrealDB`] container has internally
/// Can be rebound externally via [`testcontainers::core::ImageExt::with_mapped_port`]
///
/// [`SurrealDB`]: https://surrealdb.com/
pub const SURREALDB_PORT: ContainerPort = ContainerPort::Tcp(8000);

/// Module to work with [`SurrealDB`] inside of tests.
/// Starts an instance of SurrealDB.
/// This module is based on the official [`SurrealDB docker image`].
/// Default user and password is `root`, and exposed port is `8000` ([`SURREALDB_PORT`]).
/// # Example
/// ```
/// # use ::surrealdb::{
/// #    engine::remote::ws::{Client, Ws},
/// #    Surreal,
/// # };
/// use testcontainers_modules::{surrealdb, testcontainers::runners::SyncRunner};
///
/// let surrealdb_instance = surrealdb::SurrealDb::default().start().unwrap();
///
/// let connection_string = format!(
///     "127.0.0.1:{}",
///     surrealdb_instance
///         .get_host_port_ipv4(surrealdb::SURREALDB_PORT)
///         .unwrap(),
/// );
///
/// # let runtime = tokio::runtime::Runtime::new().unwrap();
/// # runtime.block_on(async {
/// let db: Surreal<Client> = Surreal::init();
/// db.connect::<Ws>(connection_string)
///     .await
///     .expect("Failed to connect to SurrealDB");
/// # });
/// ```
/// [`SurrealDB`]: https://surrealdb.com/
/// [`SurrealDB docker image`]: https://hub.docker.com/r/surrealdb/surrealdb
#[derive(Debug, Clone)]
pub struct SurrealDb {
    env_vars: HashMap<String, String>,
}

impl SurrealDb {
    /// Sets the user for the SurrealDB instance.
    pub fn with_user(mut self, user: &str) -> Self {
        self.env_vars
            .insert("SURREAL_USER".to_owned(), user.to_owned());
        self
    }

    /// Sets the password for the SurrealDB instance.
    pub fn with_password(mut self, password: &str) -> Self {
        self.env_vars
            .insert("SURREAL_PASS".to_owned(), password.to_owned());
        self
    }

    /// Sets unauthenticated flag for the SurrealDB instance.
    pub fn with_unauthenticated(mut self) -> Self {
        self.env_vars
            .insert("SURREAL_UNAUTHENTICATED".to_owned(), "true".to_string());
        self
    }

    /// Sets strict mode for the SurrealDB instance.
    pub fn with_strict(mut self, strict: bool) -> Self {
        self.env_vars
            .insert("SURREAL_STRICT".to_owned(), strict.to_string());
        self
    }

    /// Sets all capabilities for the SurrealDB instance.
    pub fn with_all_capabilities(mut self, allow_all: bool) -> Self {
        self.env_vars
            .insert("SURREAL_CAPS_ALLOW_ALL".to_owned(), allow_all.to_string());
        self
    }
}

impl Default for SurrealDb {
    fn default() -> Self {
        let mut env_vars = HashMap::new();
        env_vars.insert("SURREAL_USER".to_owned(), "root".to_owned());
        env_vars.insert("SURREAL_PASS".to_owned(), "root".to_owned());
        env_vars.insert("SURREAL_AUTH".to_owned(), "true".to_owned());
        env_vars.insert("SURREAL_CAPS_ALLOW_ALL".to_owned(), "true".to_owned());
        env_vars.insert("SURREAL_PATH".to_owned(), "memory".to_owned());

        Self { env_vars }
    }
}

impl Image for SurrealDb {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Started web server on ")]
    }

    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        &self.env_vars
    }

    fn cmd(&self) -> impl IntoIterator<Item = impl Into<Cow<'_, str>>> {
        ["start"]
    }

    fn expose_ports(&self) -> &[ContainerPort] {
        &[SURREALDB_PORT]
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use surrealdb::{
        engine::remote::ws::{Client, Ws},
        opt::auth::Root,
        Surreal,
    };
    use testcontainers::runners::AsyncRunner;

    use super::*;

    #[derive(Debug, Serialize, Deserialize)]
    struct Name {
        first: String,
        last: String,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct Person {
        title: String,
        name: Name,
        marketing: bool,
    }

    #[tokio::test]
    async fn surrealdb_select() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let _ = pretty_env_logger::try_init();
        let node = SurrealDb::default().start().await?;
        let host_port = node.get_host_port_ipv4(SURREALDB_PORT).await?;
        let url = format!("127.0.0.1:{host_port}");

        let db: Surreal<Client> = Surreal::init();
        db.connect::<Ws>(url).await.unwrap();
        db.signin(Root {
            username: "root",
            password: "root",
        })
        .await
        .unwrap();

        db.use_ns("test").use_db("test").await.unwrap();

        db.create::<Option<Person>>(("person", "tobie"))
            .content(Person {
                title: "Founder & CEO".to_string(),
                name: Name {
                    first: "Tobie".to_string(),
                    last: "Morgan Hitchcock".to_string(),
                },
                marketing: true,
            })
            .await
            .unwrap();

        let result = db
            .select::<Option<Person>>(("person", "tobie"))
            .await
            .unwrap();

        assert!(result.is_some());
        let result = result.unwrap();

        assert_eq!(result.title, "Founder & CEO");
        assert_eq!(result.name.first, "Tobie");
        assert_eq!(result.name.last, "Morgan Hitchcock");
        assert!(result.marketing);
        Ok(())
    }

    #[tokio::test]
    async fn surrealdb_no_auth() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let _ = pretty_env_logger::try_init();
        let node = SurrealDb::default().with_unauthenticated().start().await?;
        let host_port = node.get_host_port_ipv4(SURREALDB_PORT).await?;
        let url = format!("127.0.0.1:{host_port}");

        let db: Surreal<Client> = Surreal::init();
        db.connect::<Ws>(url).await.unwrap();
        db.use_ns("test").use_db("test").await.unwrap();

        db.create::<Option<Person>>(("person", "tobie"))
            .content(Person {
                title: "Founder & CEO".to_string(),
                name: Name {
                    first: "Tobie".to_string(),
                    last: "Morgan Hitchcock".to_string(),
                },
                marketing: true,
            })
            .await
            .unwrap();

        let result = db
            .select::<Option<Person>>(("person", "tobie"))
            .await
            .unwrap();

        assert!(result.is_some());
        let result = result.unwrap();

        assert_eq!(result.title, "Founder & CEO");
        assert_eq!(result.name.first, "Tobie");
        assert_eq!(result.name.last, "Morgan Hitchcock");
        assert!(result.marketing);
        Ok(())
    }
}
