use std::borrow::Cow;

use testcontainers::{core::WaitFor, Image};

const NAME: &str = "nats";
const TAG: &str = "2.10.14";

/// Nats image for [testcontainers](https://crates.io/crates/testcontainers).
///
/// This image is based on the official [Nats](https://hub.docker.com/_/nats) image.
#[derive(Debug, Default, Clone)]
pub struct Nats {
    cmd: NatsServerCmd,
}

/// Configuration for the NATS server command-line arguments.
///
/// This struct allows you to customize the NATS server startup configuration
/// by setting various options like authentication credentials and enabling features
/// like JetStream.
///
/// # Example
/// ```
/// use testcontainers_modules::nats::NatsServerCmd;
///
/// let cmd = NatsServerCmd::default()
///     .with_user("myuser")
///     .with_password("mypass")
///     .with_jetstream();
/// ```
#[derive(Default, Debug, Clone)]
pub struct NatsServerCmd {
    user: Option<String>,
    pass: Option<String>,

    jetstream: Option<bool>,
}

impl NatsServerCmd {
    /// Sets the username for NATS server authentication.
    ///
    /// This configures the NATS server to require authentication with the specified username.
    /// Should be used together with [`with_password`](Self::with_password) for complete authentication setup.
    ///
    /// # Example
    /// ```
    /// use testcontainers_modules::nats::NatsServerCmd;
    ///
    /// let cmd = NatsServerCmd::default().with_user("myuser");
    /// ```
    pub fn with_user(mut self, user: &str) -> Self {
        self.user = Some(user.to_owned());
        self
    }

    /// Sets the password for NATS server authentication.
    ///
    /// This configures the NATS server to require authentication with the specified password.
    /// Should be used together with [`with_user`](Self::with_user) for complete authentication setup.
    ///
    /// # Example
    /// ```
    /// use testcontainers_modules::nats::NatsServerCmd;
    ///
    /// let cmd = NatsServerCmd::default()
    ///     .with_user("myuser")
    ///     .with_password("mypass");
    /// ```
    pub fn with_password(mut self, password: &str) -> Self {
        self.pass = Some(password.to_owned());
        self
    }

    /// Enable JetStream in the Nats server to use the built-in persistence
    /// features of NATS.
    ///
    /// See: https://docs.nats.io/nats-concepts/jetstream
    ///
    /// Example:
    /// ```rust,ignore
    /// # use testcontainers_modules::nats::{Nats, NatsServerCmd};
    /// let nats_cmd = NatsServerCmd::default().with_jetstream();
    /// let node = Nats::default().with_cmd(&nats_cmd).start().await?;
    /// ```
    pub fn with_jetstream(mut self) -> Self {
        self.jetstream = Some(true);
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

        if let Some(ref jetstream) = self.jetstream {
            if *jetstream {
                args.push("--jetstream".to_owned());
            }
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
    use std::time::Duration;

    use async_nats::jetstream::{self, consumer::PushConsumer};
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

    #[test]
    fn enable_jetstream() {
        let nats_cmd_args = NatsServerCmd::default().with_jetstream();
        assert_eq!(nats_cmd_args.jetstream, Some(true));
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

    #[tokio::test]
    /// Show how to use the Nats module with the Jetstream feature.
    /// See: https://github.com/nats-io/nats.rs/blob/main/async-nats/examples/jetstream_push.rs
    async fn it_works_with_jetstream() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let nats_cmd = NatsServerCmd::default().with_jetstream();
        let container = Nats::default().with_cmd(&nats_cmd).start().await?;

        let host = container.get_host().await?;
        let host_port = container.get_host_port_ipv4(4222).await?;
        let url = format!("{host}:{host_port}");

        let nats_client = async_nats::ConnectOptions::default()
            .connect(url)
            .await
            .expect("failed to connect to nats server");

        let inbox = nats_client.new_inbox();

        let jetstream = jetstream::new(nats_client);

        let stream_name = String::from("EVENTS");

        let consumer: PushConsumer = jetstream
            .create_stream(jetstream::stream::Config {
                name: stream_name,
                subjects: vec!["events.>".to_string()],
                ..Default::default()
            })
            .await?
            .create_consumer(jetstream::consumer::push::Config {
                deliver_subject: inbox.clone(),
                inactive_threshold: Duration::from_secs(60),
                ..Default::default()
            })
            .await?;

        // Publish a few messages for the example.
        for i in 0..10 {
            jetstream
                .publish(format!("events.{i}"), "data".into())
                .await?
                .await?;
        }

        let mut messages_processed = 0;

        let mut messages = consumer.messages().await?.take(10);

        // Iterate over messages.
        while let Some(message) = messages.next().await {
            let message = message?;

            assert_eq!(
                message.subject.to_string(),
                format!("events.{messages_processed}")
            );

            // acknowledge the message
            message.ack().await.unwrap();

            messages_processed += 1;
        }

        assert_eq!(messages_processed, 10);

        Ok(())
    }
}
