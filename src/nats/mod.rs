use std::borrow::Cow;

use testcontainers::{core::WaitFor, Image};

const NAME: &str = "nats";
const TAG: &str = "2.10.14";

/// Nats image for [testcontainers](https://crates.io/crates/testcontainers).
///
/// This image is based on the official [Nats](https://hub.docker.com/_/nats) image.
#[derive(Debug, Default)]
pub struct Nats {
    cmd: NatsServerCmd,
}

#[derive(Default, Debug, Clone)]
pub struct NatsServerCmd {
    user: Option<String>,
    pass: Option<String>,
}

impl NatsServerCmd {
    pub fn with_user(mut self, user: &str) -> Self {
        self.user = Some(user.to_owned());
        self
    }

    pub fn with_password(mut self, password: &str) -> Self {
        self.pass = Some(password.to_owned());
        self
    }
}

impl IntoIterator for &NatsServerCmd {
    type Item = String;
    type IntoIter = <Vec<String> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        let mut args = Vec::new();

        if let Some(ref user) = self.user {
            args.push("--user".to_owned());
            args.push(user.to_owned())
        }
        if let Some(ref pass) = self.pass {
            args.push("--pass".to_owned());
            args.push(pass.to_owned())
        }

        args.into_iter()
    }
}

impl Image for Nats {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![
            WaitFor::message_on_stderr("Listening for client connections on 0.0.0.0:4222"),
            WaitFor::message_on_stderr("Server is ready"),
        ]
    }

    fn cmd(&self) -> impl IntoIterator<Item = impl Into<Cow<'_, str>>> {
        &self.cmd
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use testcontainers::{runners::AsyncRunner, ImageExt};

    use crate::nats::{Nats, NatsServerCmd};

    #[test]
    fn set_user() {
        let nats_cmd_args = NatsServerCmd::default().with_user("custom_user");
        assert_eq!(nats_cmd_args.user, Some("custom_user".into()));
        let _image_with_cmd = Nats::default().with_cmd(&nats_cmd_args);
    }

    #[test]
    fn set_password() {
        let nats_cmd_args = NatsServerCmd::default().with_password("custom_password");
        assert_eq!(nats_cmd_args.pass, Some("custom_password".into()));
        let _image_with_cmd = Nats::default().with_cmd(&nats_cmd_args);
    }

    #[tokio::test]
    async fn it_works() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let container = Nats::default().start().await?;
        let host = container.get_host().await?;
        let host_port = container.get_host_port_ipv4(4222).await?;
        let url = format!("{host}:{host_port}");

        let nats_client = async_nats::ConnectOptions::default()
            .connect(url)
            .await
            .expect("failed to connect to nats server");

        let mut subscriber = nats_client
            .subscribe("messages")
            .await
            .expect("failed to subscribe to nats subject");
        nats_client
            .publish("messages", "data".into())
            .await
            .expect("failed to publish to nats subject");
        let message = subscriber
            .next()
            .await
            .expect("failed to fetch nats message");
        assert_eq!(message.payload, "data");
        Ok(())
    }
}
