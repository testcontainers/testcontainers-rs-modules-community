use std::{borrow::Cow, collections::HashMap};

use testcontainers::{
    core::{ContainerPort, ContainerState, ExecCommand, WaitFor},
    Image,
};

const NAME: &str = "confluentinc/cp-kafka";
const TAG: &str = "6.1.1";
/// Port that the [`Kafka`] part of the container has internally
/// Can be rebound externally via [`testcontainers::core::ImageExt::with_mapped_port`]
///
/// [`Kafka`]: https://kafka.apache.org/
pub const KAFKA_PORT: ContainerPort = ContainerPort::Tcp(9093);
/// Port that the [`Zookeeper`] part of the container has internally
/// Can be rebound externally via [`testcontainers::core::ImageExt::with_mapped_port`]
///
/// [`Zookeeper`]: https://zookeeper.apache.org/
pub const ZOOKEEPER_PORT: ContainerPort = ContainerPort::Tcp(2181);

#[derive(Debug, Clone)]
pub struct Kafka {
    env_vars: HashMap<String, String>,
}

impl Default for Kafka {
    fn default() -> Self {
        let mut env_vars = HashMap::new();

        env_vars.insert(
            "KAFKA_ZOOKEEPER_CONNECT".to_owned(),
            format!("localhost:{}", ZOOKEEPER_PORT.as_u16()),
        );
        env_vars.insert(
            "KAFKA_LISTENERS".to_owned(),
            format!(
                "PLAINTEXT://0.0.0.0:{port},BROKER://0.0.0.0:9092",
                port = KAFKA_PORT.as_u16(),
            ),
        );
        env_vars.insert(
            "KAFKA_LISTENER_SECURITY_PROTOCOL_MAP".to_owned(),
            "BROKER:PLAINTEXT,PLAINTEXT:PLAINTEXT".to_owned(),
        );
        env_vars.insert(
            "KAFKA_INTER_BROKER_LISTENER_NAME".to_owned(),
            "BROKER".to_owned(),
        );
        env_vars.insert(
            "KAFKA_ADVERTISED_LISTENERS".to_owned(),
            format!(
                "PLAINTEXT://localhost:{port},BROKER://localhost:9092",
                port = KAFKA_PORT.as_u16(),
            ),
        );
        env_vars.insert("KAFKA_BROKER_ID".to_owned(), "1".to_owned());
        env_vars.insert(
            "KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR".to_owned(),
            "1".to_owned(),
        );

        Self { env_vars }
    }
}

impl Image for Kafka {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Creating new log file")]
    }

    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        &self.env_vars
    }

    fn cmd(&self) -> impl IntoIterator<Item = impl Into<Cow<'_, str>>> {
        vec![
            "/bin/bash".to_owned(),
            "-c".to_owned(),
            format!(
                r#"
echo 'clientPort={ZOOKEEPER_PORT}' > zookeeper.properties;
echo 'dataDir=/var/lib/zookeeper/data' >> zookeeper.properties;
echo 'dataLogDir=/var/lib/zookeeper/log' >> zookeeper.properties;
zookeeper-server-start zookeeper.properties &
. /etc/confluent/docker/bash-config &&
/etc/confluent/docker/configure &&
/etc/confluent/docker/launch"#,
                ZOOKEEPER_PORT = ZOOKEEPER_PORT.as_u16()
            ),
        ]
    }

    fn expose_ports(&self) -> &[ContainerPort] {
        &[KAFKA_PORT]
    }

    fn exec_after_start(
        &self,
        cs: ContainerState,
    ) -> Result<Vec<ExecCommand>, testcontainers::TestcontainersError> {
        let mut commands = vec![];
        let cmd = vec![
            "kafka-configs".to_string(),
            "--alter".to_string(),
            "--bootstrap-server".to_string(),
            "0.0.0.0:9092".to_string(),
            "--entity-type".to_string(),
            "brokers".to_string(),
            "--entity-name".to_string(),
            "1".to_string(),
            "--add-config".to_string(),
            format!(
                "advertised.listeners=[PLAINTEXT://127.0.0.1:{},BROKER://localhost:9092]",
                cs.host_port_ipv4(KAFKA_PORT)?
            ),
        ];
        let ready_conditions = vec![WaitFor::message_on_stdout(
            "Checking need to trigger auto leader balancing",
        )];
        commands.push(ExecCommand::new(cmd).with_container_ready_conditions(ready_conditions));
        Ok(commands)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use futures::StreamExt;
    use rdkafka::{
        consumer::{Consumer, StreamConsumer},
        producer::{FutureProducer, FutureRecord},
        ClientConfig, Message,
    };
    use testcontainers::runners::AsyncRunner;

    use crate::kafka;

    #[tokio::test]
    async fn produce_and_consume_messages() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let _ = pretty_env_logger::try_init();
        let kafka_node = kafka::Kafka::default().start().await?;

        let bootstrap_servers = format!(
            "127.0.0.1:{}",
            kafka_node.get_host_port_ipv4(kafka::KAFKA_PORT).await?
        );

        let producer = ClientConfig::new()
            .set("bootstrap.servers", &bootstrap_servers)
            .set("message.timeout.ms", "5000")
            .create::<FutureProducer>()
            .expect("Failed to create Kafka FutureProducer");

        let consumer = ClientConfig::new()
            .set("group.id", "testcontainer-rs")
            .set("bootstrap.servers", &bootstrap_servers)
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "earliest")
            .create::<StreamConsumer>()
            .expect("Failed to create Kafka StreamConsumer");

        let topic = "test-topic";

        let number_of_messages_to_produce = 5_usize;
        let expected: Vec<String> = (0..number_of_messages_to_produce)
            .map(|i| format!("Message {i}"))
            .collect();

        for (i, message) in expected.iter().enumerate() {
            producer
                .send(
                    FutureRecord::to(topic)
                        .payload(message)
                        .key(&format!("Key {i}")),
                    Duration::from_secs(0),
                )
                .await
                .unwrap();
        }

        consumer
            .subscribe(&[topic])
            .expect("Failed to subscribe to a topic");

        let mut message_stream = consumer.stream();
        for produced in expected {
            let borrowed_message =
                tokio::time::timeout(Duration::from_secs(10), message_stream.next())
                    .await
                    .unwrap()
                    .unwrap();

            assert_eq!(
                produced,
                borrowed_message
                    .unwrap()
                    .payload_view::<str>()
                    .unwrap()
                    .unwrap()
            );
        }

        Ok(())
    }
}
