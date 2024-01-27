use std::collections::HashMap;

use testcontainers::{core::WaitFor, Image, ImageArgs};

const NAME: &str = "surrealdb/surrealdb";
const TAG: &str = "v1.1.1";

pub const SURREALDB_PORT: u16 = 8000;

#[derive(Debug, Default, Clone)]
pub struct SurrealDbArgs;

impl ImageArgs for SurrealDbArgs {
    fn into_iterator(self) -> Box<dyn Iterator<Item = String>> {
        Box::new(vec!["start".to_owned()].into_iter())
    }
}

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
/// use testcontainers::clients;
/// use testcontainers_modules::surrealdb;
///
/// let docker = clients::Cli::default();
/// let surrealdb_instance = docker.run(surrealdb::SurrealDb::default());
///
/// let connection_string = format!(
///    "127.0.0.1:{}",
///    surrealdb_instance.get_host_port_ipv4(surrealdb::SURREALDB_PORT)
/// );
///
/// # let runtime = tokio::runtime::Runtime::new().unwrap();
/// # runtime.block_on(async {
/// let db: Surreal<Client> = Surreal::init();
/// db.connect::<Ws>(connection_string).await.expect("Failed to connect to SurrealDB");
/// # });
///
/// ```
/// [`SurrealDB`]: https://surrealdb.com/
/// [`SurrealDB docker image`]: https://hub.docker.com/r/surrealdb/surrealdb
///
#[derive(Debug)]
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

    /// Sets authentication for the SurrealDB instance.
    pub fn with_authentication(mut self, authentication: bool) -> Self {
        self.env_vars
            .insert("SURREAL_AUTH".to_owned(), authentication.to_string());
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
    type Args = SurrealDbArgs;

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stderr("Started web server on ")]
    }

    fn env_vars(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(self.env_vars.iter())
    }

    fn expose_ports(&self) -> Vec<u16> {
        vec![SURREALDB_PORT]
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
    use testcontainers::clients;

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
    async fn surrealdb_select() {
        let _ = pretty_env_logger::try_init();
        let docker = clients::Cli::default();
        let node = docker.run(SurrealDb::default());
        let host_port = node.get_host_port_ipv4(SURREALDB_PORT);
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
        assert_eq!(result.marketing, true)
    }

    #[tokio::test]
    async fn surrealdb_no_auth() {
        let _ = pretty_env_logger::try_init();
        let docker = clients::Cli::default();
        let node = docker.run(SurrealDb::default().with_authentication(false));
        let host_port = node.get_host_port_ipv4(SURREALDB_PORT);
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
        assert_eq!(result.marketing, true)
    }
}
