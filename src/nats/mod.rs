use testcontainers::{core::WaitFor, Image, ImageArgs};

const NAME: &str = "nats";
const TAG: &str = "2.10.14";

/// Nats image for [testcontainers](https://crates.io/crates/testcontainers).
///
/// This image is based on the official [Nats](https://hub.docker.com/_/nats) image.
#[derive(Debug, Default)]
pub struct Nats {
    _private: (),
}

#[derive(Default, Debug, Clone)]
pub struct NatsServerArgs {
    user: Option<String>,
    pass: Option<String>,
}

impl NatsServerArgs {
    pub fn with_user(mut self, user: &str) -> Self {
        self.user = Some(user.to_owned());
        self
    }

    pub fn with_password(mut self, password: &str) -> Self {
        self.pass = Some(password.to_owned());
        self
    }
}

impl ImageArgs for NatsServerArgs {
    fn into_iterator(self) -> Box<dyn Iterator<Item = String>> {
        let mut args = Vec::new();

        if let Some(ref user) = self.user {
            args.push("--user".to_owned());
            args.push(user.to_owned())
        }
        if let Some(ref pass) = self.pass {
            args.push("--pass".to_owned());
            args.push(pass.to_owned())
        }

        Box::new(args.into_iter())
    }
}

impl Image for Nats {
    type Args = NatsServerArgs;

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![
            WaitFor::message_on_stderr("Listening for client connections on 0.0.0.0:4222"),
            WaitFor::message_on_stderr("Server is ready"),
        ]
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use testcontainers::runners::AsyncRunner;

    use crate::nats::{Nats, NatsServerArgs};

    #[test]
    fn set_user() {
        let nats_cmd_args = NatsServerArgs::default().with_user("custom_user");
        assert_eq!(nats_cmd_args.user, Some("custom_user".into()));
    }

    #[test]
    fn set_password() {
        let nats_cmd_args = NatsServerArgs::default().with_password("custom_password");
        assert_eq!(nats_cmd_args.pass, Some("custom_password".into()));
    }

    #[tokio::test]
    async fn it_works() {
        let container = Nats::default().start().await;
        let host = container.get_host().await;
        let host_port = container.get_host_port_ipv4(4222).await;
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
    }
}
