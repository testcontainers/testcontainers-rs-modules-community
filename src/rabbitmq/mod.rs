use testcontainers::{core::WaitFor, Image};

const NAME: &str = "rabbitmq";
const TAG: &str = "3.8.22-management";

/// Module to work with [`RabbitMQ`] inside of tests.
///
/// Starts an instance of RabbitMQ with the [`management-plugin`] started by default,
/// so you are able to use the [`RabbitMQ Management HTTP API`] to manage the configuration if the started [`RabbitMQ`] instance at test runtime.
///
/// This module is based on the officlal [`RabbitMQ docker image`].
///
/// # Example
/// ```
/// use testcontainers::clients;
/// use testcontainers_modules::rabbitmq;
///
/// let docker = clients::Cli::default();
/// let rabbitmq_instance = docker.run(rabbitmq::RabbitMq::default());
///
/// let amqp_url = format!("amqp://127.0.0.1:{}", rabbitmq_instance.get_host_port_ipv4(5672));
///
/// // do something with the started rabbitmq instance..
/// ```
///
/// [`RabbitMQ`]: https://www.rabbitmq.com/
/// [`management-plugin`]: https://www.rabbitmq.com/management.html
/// [`RabbitMQ Management HTTP API`]: https://www.rabbitmq.com/management.html#http-api
/// [`RabbitMQ docker image`]: https://hub.docker.com/_/rabbitmq
#[derive(Debug, Default, Clone)]
pub struct RabbitMq;

impl Image for RabbitMq {
    type Args = ();

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout(
            "Server startup complete; 4 plugins started.",
        )]
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use futures::StreamExt;
    use lapin::{
        options::{
            BasicConsumeOptions, BasicPublishOptions, ExchangeDeclareOptions, QueueBindOptions,
            QueueDeclareOptions,
        },
        types::FieldTable,
        BasicProperties, Connection, ConnectionProperties, ExchangeKind,
    };
    use testcontainers::clients;

    use crate::rabbitmq;

    #[tokio::test]
    async fn rabbitmq_produce_and_consume_messages() {
        let _ = pretty_env_logger::try_init();
        let docker = clients::Cli::default();
        let rabbit_node = docker.run(rabbitmq::RabbitMq::default());

        let amqp_url = format!("amqp://127.0.0.1:{}", rabbit_node.get_host_port_ipv4(5672));

        let options = ConnectionProperties::default();
        let connection = Connection::connect(amqp_url.as_str(), options)
            .await
            .unwrap();

        let channel = connection.create_channel().await.unwrap();

        assert!(channel.status().connected());

        channel
            .exchange_declare(
                "test_exchange",
                ExchangeKind::Topic,
                ExchangeDeclareOptions::default(),
                FieldTable::default(),
            )
            .await
            .unwrap();

        let queue = channel
            .queue_declare(
                "test_queue",
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await
            .unwrap();

        channel
            .queue_bind(
                queue.name().as_str(),
                "test_exchange",
                "#",
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .await
            .unwrap();

        let mut consumer = channel
            .basic_consume(
                queue.name().as_str(),
                "test_consumer_tag",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await
            .unwrap();

        channel
            .basic_publish(
                "test_exchange",
                "routing-key",
                BasicPublishOptions::default(),
                b"Test Payload",
                BasicProperties::default(),
            )
            .await
            .unwrap();

        let consumed = tokio::time::timeout(Duration::from_secs(10), consumer.next())
            .await
            .unwrap()
            .unwrap();

        let delivery = consumed.expect("Failed to consume delivery!");
        assert_eq!(
            String::from_utf8(delivery.data.clone()).unwrap(),
            "Test Payload"
        );
        assert_eq!(delivery.exchange.as_str(), "test_exchange");
        assert_eq!(delivery.routing_key.as_str(), "routing-key");
    }
}
