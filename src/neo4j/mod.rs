use std::{borrow::Cow, cell::RefCell, collections::HashMap, io::BufRead};
use testcontainers::{
    core::{ContainerState, WaitFor},
    Image, RunnableImage,
};

/// Available Neo4j plugins.
/// See [Neo4j operations manual](https://neo4j.com/docs/operations-manual/current/docker/operations/#docker-neo4j-plugins) for more information.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Neo4jLabsPlugin {
    Apoc,
    ApocCore,
    Bloom,
    Streams,
    GraphDataScience,
    NeoSemantics,
    Custom(String),
}

impl std::fmt::Display for Neo4jLabsPlugin {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Apoc => formatter.pad("apoc"),
            Self::ApocCore => formatter.pad("apoc-core"),
            Self::Bloom => formatter.pad("bloom"),
            Self::Streams => formatter.pad("streams"),
            Self::GraphDataScience => formatter.pad("graph-data-science"),
            Self::NeoSemantics => formatter.pad("n10s"),
            Self::Custom(plugin_name) => formatter.pad(plugin_name),
        }
    }
}

/// Neo4j image for [testcontainers](https://crates.io/crates/testcontainers).
///
/// This image is based on the official [Neo4j](https://hub.docker.com/_/neo4j) image.
/// The default user is `neo4j` and the default password is `neo`.
/// The default version is `5`.
///
/// # Example
///
/// ```rust,no_run
/// use testcontainers::clients::Cli;
/// use testcontainers_modules::neo4j::Neo4j;
///
/// let cli = Cli::default();
/// let container = cli.run(Neo4j::default());
/// let uri = format!("bolt://localhost:{}", container.image().bolt_port_ipv4());
/// let auth_user = container.image().user();
/// let auth_pass = container.image().password();
/// // connect to Neo4j with the uri, user and pass
/// ```
///
/// # Neo4j Version
///
/// The version of the image can be set with the `NEO4J_VERSION_TAG` environment variable.
/// The default version is `5`.
/// The available versions can be found on [Docker Hub](https://hub.docker.com/_/neo4j/tags).
///
/// The used version can be retrieved with the `version` method.
///
/// # Auth
///
/// The default user is `neo4j` and the default password is `neo`.
///
/// The used user can be retrieved with the `user` method.
/// The used password can be retrieved with the `pass` method.
///
/// # Environment variables
///
/// The following environment variables are supported:
///   * `NEO4J_VERSION_TAG`: The default version of the image to use.
///   * `NEO4J_TEST_USER`: The default user to use for authentication.
///   * `NEO4J_TEST_PASS`: The default password to use for authentication.
///
/// # Neo4j Labs Plugins
///
/// Neo4j offers built-in support for Neo4j Labs plugins.
/// The method `with_neo4j_labs_plugin` can be used to define them.
///
/// Supported plugins are APOC, APOC Core, Bloom, Streams, Graph Data Science, and Neo Semantics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Neo4j {
    version: Value,
    user: Value,
    pass: Value,
    enterprise: bool,
    plugins: Vec<Neo4jLabsPlugin>,
}

impl Neo4j {
    const DEFAULT_USER: &'static str = "neo4j";
    const DEFAULT_PASS: &'static str = "neo";
    const DEFAULT_VERSION_TAG: &'static str = "5";

    /// Create a new instance of a Neo4j image.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            version: Value::Default(Self::DEFAULT_VERSION_TAG),
            user: Value::Default(Self::DEFAULT_USER),
            pass: Value::Default(Self::DEFAULT_PASS),
            enterprise: false,
            plugins: Vec::new(),
        }
    }

    /// Create a new instance of a Neo4j 5 image with the default user and password.
    #[must_use]
    pub const fn from_env() -> Self {
        Self {
            version: Value::Env {
                var: "NEO4J_VERSION_TAG",
                fallback: Self::DEFAULT_VERSION_TAG,
            },
            user: Value::Env {
                var: "NEO4J_TEST_USER",
                fallback: Self::DEFAULT_USER,
            },
            pass: Value::Env {
                var: "NEO4J_TEST_PASS",
                fallback: Self::DEFAULT_PASS,
            },
            enterprise: false,
            plugins: Vec::new(),
        }
    }

    /// Set the Neo4j version to use.
    /// The value must be an existing Neo4j version tag.
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Value::Value(version.into());
        self
    }

    /// Set the username to use.
    #[must_use]
    pub fn with_user(mut self, user: impl Into<String>) -> Self {
        self.user = Value::Value(user.into());
        self
    }

    /// Set the password to use.
    #[must_use]
    pub fn with_password(mut self, pass: impl Into<String>) -> Self {
        self.pass = Value::Value(pass.into());
        self
    }

    /// Do not use any authentication on the testcontainer.
    ///
    /// Setting this will override any prior usages of [`Self::with_user`] and
    /// [`Self::with_password`].
    pub fn without_authentication(mut self) -> Self {
        self.user = Value::Unset;
        self.pass = Value::Unset;
        self
    }

    /// Use the enterprise edition of Neo4j.
    ///
    /// # Note
    /// Please have a look at the [Neo4j Licensing page](https://neo4j.com/licensing/).
    /// While the Neo4j Community Edition can be used for free in your projects under the GPL v3 license,
    /// Neo4j Enterprise edition needs either a commercial, education or evaluation license.
    pub fn with_enterprise_edition(
        mut self,
    ) -> Result<Self, Box<dyn std::error::Error + Sync + Send + 'static>> {
        const ACCEPTANCE_FILE_NAME: &str = "container-license-acceptance.txt";

        let version = Self::value(&self.version).expect("Version is always set");
        let image = format!("neo4j:{}-enterprise", version);

        let acceptance_file = std::env::current_dir()
            .ok()
            .map(|o| o.join(ACCEPTANCE_FILE_NAME));

        let has_license_acceptance = acceptance_file
            .as_deref()
            .and_then(|o| std::fs::File::open(o).ok())
            .into_iter()
            .flat_map(|o| std::io::BufReader::new(o).lines())
            .any(|o| o.map_or(false, |line| line.trim() == image));

        if !has_license_acceptance {
            return Err(format!(
                concat!(
                    "You need to accept the Neo4j Enterprise Edition license by ",
                    "creating the file `{}` with the following content:\n\n\t{}",
                ),
                acceptance_file.map_or_else(
                    || ACCEPTANCE_FILE_NAME.to_owned(),
                    |o| { o.display().to_string() }
                ),
                image
            )
            .into());
        }

        self.enterprise = true;
        Ok(self)
    }

    /// Add Neo4j lab plugins to get started with the database.
    #[must_use]
    pub fn with_neo4j_labs_plugin(mut self, plugins: &[Neo4jLabsPlugin]) -> Self {
        self.plugins.extend_from_slice(plugins);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Value {
    Env {
        var: &'static str,
        fallback: &'static str,
    },
    Default(&'static str),
    Value(String),
    Unset,
}

impl Default for Neo4j {
    fn default() -> Self {
        Self::from_env()
    }
}

/// The actual Neo4j testcontainers image type which is returned by `container.image()`
pub struct Neo4jImage {
    version: String,
    auth: Option<(String, String)>,
    env_vars: HashMap<String, String>,
    state: RefCell<Option<ContainerState>>,
}

impl Neo4jImage {
    /// Return the version of the Neo4j image.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Return the user/password authentication tuple of the Neo4j server.
    /// If no authentication is set, `None` is returned.
    #[must_use]
    pub fn auth(&self) -> Option<(&str, &str)> {
        self.auth
            .as_ref()
            .map(|(user, pass)| (user.as_str(), pass.as_str()))
    }

    /// Return the user of the Neo4j server.
    /// If no authentication is set, `None` is returned.
    #[must_use]
    pub fn user(&self) -> Option<&str> {
        self.auth().map(|(user, _)| user)
    }

    /// Return the password of the Neo4j server.
    /// If no authentication is set, `None` is returned.
    #[must_use]
    pub fn password(&self) -> Option<&str> {
        self.auth().map(|(_, pass)| pass)
    }

    /// Return the port to connect to the Neo4j server via Bolt over IPv4.
    pub fn bolt_port_ipv4(&self) -> u16 {
        self.state
            .borrow()
            .as_ref()
            .expect("Container must be started before port can be retrieved")
            .host_port_ipv4(7687)
    }

    /// Return the port to connect to the Neo4j server via Bolt over IPv6.
    pub fn bolt_uri_ipv6(&self) -> u16 {
        self.state
            .borrow()
            .as_ref()
            .expect("Container must be started before port can be retrieved")
            .host_port_ipv6(7687)
    }

    /// Return the port to connect to the Neo4j server via HTTP over IPv4.
    pub fn http_port_ipv4(&self) -> u16 {
        self.state
            .borrow()
            .as_ref()
            .expect("Container must be started before port can be retrieved")
            .host_port_ipv4(7474)
    }

    /// Return the port to connect to the Neo4j server via HTTP over IPv6.
    pub fn http_uri_ipv6(&self) -> u16 {
        self.state
            .borrow()
            .as_ref()
            .expect("Container must be started before port can be retrieved")
            .host_port_ipv6(7474)
    }
}

impl Image for Neo4jImage {
    type Args = ();

    fn name(&self) -> String {
        "neo4j".to_owned()
    }

    fn tag(&self) -> String {
        self.version.clone()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![
            WaitFor::message_on_stdout("Bolt enabled on"),
            WaitFor::message_on_stdout("Started."),
        ]
    }

    fn env_vars(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(self.env_vars.iter())
    }

    fn exec_after_start(&self, cs: ContainerState) -> Vec<testcontainers::core::ExecCommand> {
        *self.state.borrow_mut() = Some(cs);
        Vec::new()
    }
}

impl Neo4j {
    fn enterprise_env(&self) -> impl IntoIterator<Item = (String, String)> {
        self.enterprise.then(|| {
            (
                "NEO4J_ACCEPT_LICENSE_AGREEMENT".to_owned(),
                "yes".to_owned(),
            )
        })
    }

    fn auth_env(&self) -> impl IntoIterator<Item = (String, String)> {
        fn auth(image: &Neo4j) -> Option<String> {
            let user = Neo4j::value(&image.user)?;
            let pass = Neo4j::value(&image.pass)?;
            Some(format!("{}/{}", user, pass))
        }

        let auth = auth(self).unwrap_or_else(|| "none".to_owned());
        Some(("NEO4J_AUTH".to_owned(), auth))
    }

    fn plugins_env(&self) -> impl IntoIterator<Item = (String, String)> {
        if self.plugins.is_empty() {
            return None;
        }

        let plugin_names = self
            .plugins
            .iter()
            .map(|p| format!("\"{}\"", p))
            .collect::<Vec<String>>()
            .join(",");

        let plugin_definition = format!("[{}]", plugin_names);

        Some(("NEO4JLABS_PLUGINS".to_owned(), plugin_definition))
    }

    fn conf_env(&self) -> impl IntoIterator<Item = (String, String)> {
        let pass = Self::value(&self.pass)?;

        if pass.len() < 8 {
            Some((
                "NEO4J_dbms_security_auth__minimum__password__length".to_owned(),
                pass.len().to_string(),
            ))
        } else {
            None
        }
    }

    fn build(mut self) -> Neo4jImage {
        self.plugins.sort();
        self.plugins.dedup();

        let mut env_vars = HashMap::new();

        for (key, value) in self.enterprise_env() {
            env_vars.insert(key, value);
        }

        for (key, value) in self.auth_env() {
            env_vars.insert(key, value);
        }

        for (key, value) in self.plugins_env() {
            env_vars.insert(key, value);
        }

        for (key, value) in self.conf_env() {
            env_vars.insert(key, value);
        }

        let auth = Self::value(&self.user).and_then(|user| {
            Self::value(&self.pass).map(|pass| (user.into_owned(), pass.into_owned()))
        });

        let version = Self::value(&self.version).expect("Version must be set");
        let version = format!(
            "{}{}",
            version,
            if self.enterprise { "-enterprise" } else { "" }
        );

        Neo4jImage {
            version,
            auth,
            env_vars,
            state: RefCell::new(None),
        }
    }

    fn value(value: &Value) -> Option<Cow<'_, str>> {
        Some(match value {
            &Value::Env { var, fallback } => {
                std::env::var(var).map_or_else(|_| fallback.into(), Into::into)
            }
            &Value::Default(value) => value.into(),
            Value::Value(value) => value.as_str().into(),
            Value::Unset => return None,
        })
    }
}

impl From<Neo4j> for Neo4jImage {
    fn from(neo4j: Neo4j) -> Self {
        neo4j.build()
    }
}

impl From<Neo4j> for RunnableImage<Neo4jImage> {
    fn from(neo4j: Neo4j) -> Self {
        Self::from(neo4j.build())
    }
}

impl std::fmt::Debug for Neo4jImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Neo4jImage")
            .field("version", &self.version)
            .field("auth", &self.auth())
            .field("env_vars", &self.env_vars)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use neo4rs::Graph;
    use testcontainers::clients::Cli;

    use super::*;

    #[test]
    fn set_valid_version() {
        let neo4j = Neo4j::new().with_version("4.2.0").build();
        assert_eq!(neo4j.version, "4.2.0");
    }

    #[test]
    fn set_partial_version() {
        let neo4j = Neo4j::new().with_version("4.2").build();
        assert_eq!(neo4j.version, "4.2");

        let neo4j = Neo4j::new().with_version("4").build();
        assert_eq!(neo4j.version, "4");
    }

    #[test]
    fn set_user() {
        let neo4j = Neo4j::new().with_user("Benutzer").build();
        assert_eq!(neo4j.user(), Some("Benutzer"));
        assert_eq!(neo4j.auth(), Some(("Benutzer", "neo")));
        assert_eq!(neo4j.env_vars.get("NEO4J_AUTH").unwrap(), "Benutzer/neo");
    }

    #[test]
    fn set_password() {
        let neo4j = Neo4j::new().with_password("Passwort").build();
        assert_eq!(neo4j.password(), Some("Passwort"));
        assert_eq!(neo4j.auth(), Some(("neo4j", "Passwort")));
        assert_eq!(neo4j.env_vars.get("NEO4J_AUTH").unwrap(), "neo4j/Passwort");
    }

    #[test]
    fn set_short_password() {
        let neo4j = Neo4j::new().with_password("1337").build();
        assert_eq!(neo4j.password(), Some("1337"));
        assert_eq!(neo4j.auth(), Some(("neo4j", "1337")));
        assert_eq!(
            neo4j
                .env_vars
                .get("NEO4J_dbms_security_auth__minimum__password__length")
                .unwrap(),
            "4"
        );
    }

    #[test]
    fn disable_auth() {
        let neo4j = Neo4j::new().without_authentication().build();
        assert_eq!(neo4j.password(), None);
        assert_eq!(neo4j.user(), None);
        assert_eq!(neo4j.auth(), None);
        assert_eq!(neo4j.env_vars.get("NEO4J_AUTH").unwrap(), "none");
    }

    #[test]
    fn single_plugin_definition() {
        let neo4j = Neo4j::new()
            .with_neo4j_labs_plugin(&[Neo4jLabsPlugin::Apoc])
            .build();
        assert_eq!(
            neo4j.env_vars.get("NEO4JLABS_PLUGINS").unwrap(),
            "[\"apoc\"]"
        );
    }

    #[test]
    fn multiple_plugin_definition() {
        let neo4j = Neo4j::new()
            .with_neo4j_labs_plugin(&[Neo4jLabsPlugin::Apoc, Neo4jLabsPlugin::Bloom])
            .build();
        assert_eq!(
            neo4j.env_vars.get("NEO4JLABS_PLUGINS").unwrap(),
            "[\"apoc\",\"bloom\"]"
        );
    }

    #[test]
    fn multiple_wiht_plugin_calls() {
        let neo4j = Neo4j::new()
            .with_neo4j_labs_plugin(&[Neo4jLabsPlugin::Apoc])
            .with_neo4j_labs_plugin(&[Neo4jLabsPlugin::Bloom])
            .with_neo4j_labs_plugin(&[Neo4jLabsPlugin::Apoc])
            .build();
        assert_eq!(
            neo4j.env_vars.get("NEO4JLABS_PLUGINS").unwrap(),
            "[\"apoc\",\"bloom\"]"
        );
    }

    #[tokio::test]
    async fn it_works() {
        let cli = Cli::default();
        let container = cli.run(Neo4j::default());

        let uri = format!("bolt://localhost:{}", container.image().bolt_port_ipv4());

        let auth_user = container.image().user().expect("default user");
        let auth_pass = container.image().password().expect("default password");

        let graph = Graph::new(uri, auth_user, auth_pass).await.unwrap();
        let mut result = graph.execute(neo4rs::query("RETURN 1")).await.unwrap();
        let row = result.next().await.unwrap().unwrap();
        let value: i64 = row.get("1").unwrap();
        assert_eq!(1, value);
    }
}
