use testcontainers::{core::WaitFor, Image};

const NAME: &str = "nats";
const TAG: &str = "2.10.14";

/// Nats image for [testcontainers](https://crates.io/crates/testcontainers).
///
/// This image is based on the official [Nats](https://hub.docker.com/_/nats) image.
#[derive(Debug, Default)]
pub struct Nats;

impl Image for Nats {
    type Args = ();

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
    use crate::nats::Nats;
    use futures::StreamExt;
    use testcontainers::runners::AsyncRunner;

    #[tokio::test]
    async fn it_works() {
        let container = Nats.start().await;

        let host_port = container.get_host_port_ipv4(4222).await;
        let url = format!("127.0.0.1:{host_port}");

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
