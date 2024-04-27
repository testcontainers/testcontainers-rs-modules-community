use std::borrow::Cow;

use testcontainers::{core::WaitFor, Image, RunnableImage};

/// Nats image for [testcontainers](https://crates.io/crates/testcontainers).
///
/// This image is based on the official [Nats](https://hub.docker.com/_/nats) image.
/// The default user is `` and the default password is ``.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Nats {
    version: Value,
    user: Option<Value>,
    pass: Option<Value>,
}

impl Nats {
    const DEFAULT_USER: &'static str = "";
    const DEFAULT_PASS: &'static str = "";
    const DEFAULT_VERSION_TAG: &'static str = "2.10.14";

    /// Create a new instance of a Nats image.
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: Cow::Borrowed(Self::DEFAULT_VERSION_TAG),
            user: Some(Cow::Borrowed(Self::DEFAULT_USER)),
            pass: Some(Cow::Borrowed(Self::DEFAULT_PASS)),
        }
    }

    /// Set the Nats version to use.
    /// The value must be an existing Nats version tag.
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
}

type Value = Cow<'static, str>;

impl Default for Nats {
    fn default() -> Self {
        Self::new()
    }
}

/// The actual Nats testcontainers image type which is returned by `container.image()`
pub struct NatsImage {
    version: String,
    auth: Option<(String, String)>,
}

impl NatsImage {
    /// Return the version of the Nats image.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Return the user/password authentication tuple of the Nats server.
    /// If no authentication is set, `None` is returned.
    #[must_use]
    pub fn auth(&self) -> Option<(&str, &str)> {
        self.auth
            .as_ref()
            .map(|(user, pass)| (user.as_str(), pass.as_str()))
    }

    /// Return the user of the Nats server.
    /// If no authentication is set, `None` is returned.
    #[must_use]
    pub fn user(&self) -> Option<&str> {
        self.auth().map(|(user, _)| user)
    }

    /// Return the password of the Nats server.
    /// If no authentication is set, `None` is returned.
    #[must_use]
    pub fn password(&self) -> Option<&str> {
        self.auth().map(|(_, pass)| pass)
    }
}

impl Image for NatsImage {
    type Args = ();

    fn name(&self) -> String {
        "nats".to_owned()
    }

    fn tag(&self) -> String {
        self.version.clone()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![
            WaitFor::message_on_stderr("Listening for client connections on 0.0.0.0:4222"),
            WaitFor::message_on_stderr("Server is ready"),
        ]
    }
}

impl Nats {
    pub fn build(self) -> NatsImage {

        let auth = self
            .user
            .and_then(|user| self.pass.map(|pass| (user.into_owned(), pass.into_owned())));

        let version = self.version.into_owned();


        NatsImage {
            version,
            auth,
        }
    }
}

impl From<Nats> for NatsImage {
    fn from(nats: Nats) -> Self {
        nats.build()
    }
}

impl From<Nats> for RunnableImage<NatsImage> {
    fn from(nats: Nats) -> Self {
        Self::from(nats.build())
    }
}

impl std::fmt::Debug for NatsImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NatsImage")
            .field("version", &self.version)
            .field("auth", &self.auth())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use testcontainers::clients::Cli;

    use super::*;

    #[test]
    fn set_valid_version() {
        let nats = Nats::new().with_version("2.10.14").build();
        assert_eq!(nats.version, "2.10.14");
    }

    #[test]
    fn set_partial_version() {
        let nats = Nats::new().with_version("2.10").build();
        assert_eq!(nats.version, "2.10");

        let nats = Nats::new().with_version("2").build();
        assert_eq!(nats.version, "2");
    }

    #[test]
    fn set_user() {
        let nats = Nats::new().with_user("shalbaal").build();
        assert_eq!(nats.user(), Some("shalbaal"));
        assert_eq!(nats.auth(), Some(("shalbaal", "")));
    }

    #[test]
    fn set_password() {
        let nats = Nats::new().with_password("Passwort").build();
        assert_eq!(nats.password(), Some("Passwort"));
        assert_eq!(nats.auth(), Some(("", "Passwort")));
    }

    #[test]
    fn set_short_password() {
        let nats = Nats::new().with_password("1337").build();
        assert_eq!(nats.password(), Some("1337"));
        assert_eq!(nats.auth(), Some(("", "1337")));
    }

    #[test]
    fn disable_auth() {
        let nats = Nats::new().without_authentication().build();
        assert_eq!(nats.password(), None);
        assert_eq!(nats.user(), None);
        assert_eq!(nats.auth(), None);
    }

    #[tokio::test]
    async fn it_works() {
        let cli = Cli::default();
        let nats = Nats::default().build();
        let container = cli.run(nats);

        let auth_user = container.image().user().expect("");
        let auth_pass = container.image().password().expect("");

        let url = format!("127.0.0.1:{}", container.get_host_port_ipv4(4222));

        let nats_client = async_nats::ConnectOptions::with_user_and_password(
            auth_user.to_string(),
            auth_pass.to_string(),
        )
        .connect(url)
        .await
        .expect("failed to connect to nats server");

        let mut subscriber = nats_client.subscribe("messages").await.expect("failed to subscribe to nats subject");
        nats_client.publish("messages", "data".into()).await.expect("failed to publish to nats subject");
        let message = subscriber.next().await.expect("failed to fetch nats message");
        assert_eq!(message.payload, "data");


    }
}
