use std::{
    borrow::Cow,
    cell::RefCell,
    collections::{BTreeSet, HashMap},
};
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
    user: Option<Value>,
    pass: Option<Value>,
    plugins: BTreeSet<Neo4jLabsPlugin>,
}

impl Neo4j {
    const DEFAULT_USER: &'static str = "neo4j";
    const DEFAULT_PASS: &'static str = "password";
    const DEFAULT_VERSION_TAG: &'static str = "5";

    /// Create a new instance of a Neo4j image.
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: Cow::Borrowed(Self::DEFAULT_VERSION_TAG),
            user: Some(Cow::Borrowed(Self::DEFAULT_USER)),
            pass: Some(Cow::Borrowed(Self::DEFAULT_PASS)),
            plugins: BTreeSet::new(),
        }
    }

    /// Set the Neo4j version to use.
    /// The value must be an existing Neo4j version tag.
    pub fn with_version(mut self, version: impl Into<Value>) -> Self {
        self.version = version.into();
        self
    }

    /// Set the username to use.
    #[must_use]
    pub fn with_user(mut self, user: impl Into<Value>) -> Self {
        self.user = Some(user.into());
        self
    }

    /// Set the password to use.
    #[must_use]
    pub fn with_password(mut self, pass: impl Into<Value>) -> Self {
        self.pass = Some(pass.into());
        self
    }

    /// Do not use any authentication on the testcontainer.
    ///
    /// Setting this will override any prior usages of [`Self::with_user`] and
    /// [`Self::with_password`].
    pub fn without_authentication(mut self) -> Self {
        self.user = None;
        self.pass = None;
        self
    }

    /// Add Neo4j lab plugins to get started with the database.
    #[must_use]
    pub fn with_neo4j_labs_plugin(mut self, plugins: &[Neo4jLabsPlugin]) -> Self {
        self.plugins.extend(plugins.iter().cloned());
        self
    }
}

type Value = Cow<'static, str>;

impl Default for Neo4j {
    fn default() -> Self {
        Self::new()
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
    fn auth_env(&self) -> impl IntoIterator<Item = (String, String)> {
        let auth = self
            .user
            .as_ref()
            .and_then(|user| self.pass.as_ref().map(|pass| format!("{}/{}", user, pass)))
            .unwrap_or_else(|| "none".to_owned());
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
        let pass = self.pass.as_ref()?;

        if pass.len() < 8 {
            Some((
                "NEO4J_dbms_security_auth__minimum__password__length".to_owned(),
                pass.len().to_string(),
            ))
        } else {
            None
        }
    }

    fn build(self) -> Neo4jImage {
        let mut env_vars = HashMap::new();

        for (key, value) in self.auth_env() {
            env_vars.insert(key, value);
        }

        for (key, value) in self.plugins_env() {
            env_vars.insert(key, value);
        }

        for (key, value) in self.conf_env() {
            env_vars.insert(key, value);
        }

        let auth = self
            .user
            .and_then(|user| self.pass.map(|pass| (user.into_owned(), pass.into_owned())));

        let version = self.version.into_owned();

        Neo4jImage {
            version,
            auth,
            env_vars,
            state: RefCell::new(None),
        }
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
        assert_eq!(neo4j.auth(), Some(("Benutzer", "password")));
        assert_eq!(
            neo4j.env_vars.get("NEO4J_AUTH").unwrap(),
            "Benutzer/password"
        );
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
