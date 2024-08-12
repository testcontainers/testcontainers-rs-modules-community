use std::{borrow::Cow, collections::HashMap};
use testcontainers::{
    core::{ContainerPort, ContainerState, ExecCommand, WaitFor},
    Image,
};

const KAFKA_NATIVE_IMAGE_NAME: &str = "apache/kafka-native";
const KAFKA_IMAGE_NAME: &str = "apache/kafka";
const TAG: &str = "latest";

/// Port that [`Apache Kafka`] uses internally.
/// Can be rebound externally via [`testcontainers::core::ImageExt::with_mapped_port`]
///
/// [`Apache Kafka`]: https://kafka.apache.org/
pub const KAFKA_PORT: ContainerPort = ContainerPort::Tcp(9092);

const START_SCRIPT: &str = "/opt/kafka/testcontainers_start.sh";
const DEFAULT_INTERNAL_TOPIC_RF: usize = 1;
const DEFAULT_CLUSTER_ID: &str = "5L6g3nShT-eMCtK--X86sw";
const DEFAULT_BROKER_ID: usize = 1;

/// Module to work with [`Apache Kafka`] broker
///
/// Starts an instance of Apache Kafka broker, with Apache Kafka Raft (KRaft) is the consensus protocol
/// enabled.
///
/// This module is based on the official [`Apache Kafka docker image`](https://hub.docker.com/r/apache/kafka)
///
/// Module comes in two flavours:
///
/// - [`Apache Kafka GraalVM docker image`](https://hub.docker.com/r/apache/kafka-native), which is default as it provides faster startup and lower memory consumption.
/// - [`Apache Kafka JVM docker image`](https://hub.docker.com/r/apache/kafka)
///
/// # Example
/// ```
/// use testcontainers_modules::{kafka::apache, testcontainers::runners::SyncRunner};
/// let kafka_node = apache::Kafka::default().start().unwrap();
/// // connect to kafka server to send/receive messages
/// ```
///
/// [`Apache Kafka`]: https://kafka.apache.org/
#[derive(Debug, Clone)]
pub struct Kafka {
    env_vars: HashMap<String, String>,
    image_name: String,
}

impl Default for Kafka {
    fn default() -> Self {
        let mut env_vars = HashMap::new();
        env_vars.insert(
            "KAFKA_LISTENERS".to_owned(),
            format!(
                "PLAINTEXT://0.0.0.0:{},BROKER://0.0.0.0:9093,CONTROLLER://0.0.0.0:9094",
                KAFKA_PORT.as_u16()
            ),
        );
        env_vars.insert("CLUSTER_ID".to_owned(), DEFAULT_CLUSTER_ID.to_owned());
        env_vars.insert(
            "KAFKA_PROCESS_ROLES".to_owned(),
            "broker,controller".to_owned(),
        );

        env_vars.insert(
            "KAFKA_CONTROLLER_LISTENER_NAMES".to_owned(),
            "CONTROLLER".to_owned(),
        );
        env_vars.insert(
            "KAFKA_LISTENER_SECURITY_PROTOCOL_MAP".to_owned(),
            "BROKER:PLAINTEXT,PLAINTEXT:PLAINTEXT,CONTROLLER:PLAINTEXT".to_owned(),
        );
        env_vars.insert(
            "KAFKA_INTER_BROKER_LISTENER_NAME".to_owned(),
            "BROKER".to_owned(),
        );
        env_vars.insert(
            "KAFKA_ADVERTISED_LISTENERS".to_owned(),
            format!(
                "PLAINTEXT://localhost:{},BROKER://localhost:9092",
                KAFKA_PORT.as_u16()
            ),
        );
        env_vars.insert("KAFKA_BROKER_ID".to_owned(), DEFAULT_BROKER_ID.to_string());
        env_vars.insert(
            "KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR".to_owned(),
            DEFAULT_INTERNAL_TOPIC_RF.to_string(),
        );
        env_vars.insert(
            "KAFKA_CONTROLLER_QUORUM_VOTERS".to_owned(),
            format!("{DEFAULT_BROKER_ID}@localhost:9094").to_owned(),
        );

        Self {
            env_vars,
            image_name: KAFKA_NATIVE_IMAGE_NAME.to_string(),
        }
    }
}

impl Kafka {
    /// Switches default image to `apache/kafka` instead of `apache/kafka-native`
    pub fn with_jvm_image(mut self) -> Self {
        self.image_name = KAFKA_IMAGE_NAME.to_string();

        self
    }
}

impl Image for Kafka {
    fn name(&self) -> &str {
        self.image_name.as_str()
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        // container will be started with custom command which will wait
        // for a start script to be created in `exec_after_start`,
        // thus container needs to progress to `exec_after_start`
        //
        // actual wait for `ready_conditions` is be done in `exec_after_start`
        vec![]
    }

    fn entrypoint(&self) -> Option<&str> {
        Some("bash")
    }

    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        &self.env_vars
    }

    fn cmd(&self) -> impl IntoIterator<Item = impl Into<Cow<'_, str>>> {
        // command starts a while (wait) loop until start script is created.
        // start script configures kafka with exposed port as is not
        // available at container creation,
        //
        // start script creation is performed in `exec_after_start`
        vec![
            "-c".to_string(),
            format!("while [ ! -f {START_SCRIPT}  ]; do sleep 0.1; done; chmod 755 {START_SCRIPT} && {START_SCRIPT}"),
        ]
        .into_iter()
    }

    fn expose_ports(&self) -> &[ContainerPort] {
        &[KAFKA_PORT]
    }

    fn exec_after_start(
        &self,
        cs: ContainerState,
    ) -> Result<Vec<ExecCommand>, testcontainers::TestcontainersError> {
        let mut commands = vec![];
        // with container running, port which will accept kafka connections is known
        // so we can proceed with creating a script which starts kafka broker
        // with correct port configuration.
        //
        // note: scrip will actually be executed by wait process started in `cmd`
        let cmd = vec![
            "sh".to_string(),
            "-c".to_string(),
            format!(
                "echo '#!/usr/bin/env bash\nexport KAFKA_ADVERTISED_LISTENERS=PLAINTEXT://127.0.0.1:{},BROKER://localhost:9093\n/etc/kafka/docker/run \n' > {}",
                cs.host_port_ipv4(KAFKA_PORT)?,
                START_SCRIPT
            ),
        ];
        let ready_conditions = vec![WaitFor::message_on_stdout("Kafka Server started")];
        // as start script will be executed by `cmd` process we need to look
        // for the message in container log, not script output.
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

    use crate::kafka::apache;

    #[tokio::test]
    async fn produce_and_consume_messages_graalvm(
    ) -> Result<(), Box<dyn std::error::Error + 'static>> {
        let _ = pretty_env_logger::try_init();
        let kafka_node = apache::Kafka::default().start().await?;

        let bootstrap_servers = format!(
            "127.0.0.1:{}",
            kafka_node.get_host_port_ipv4(apache::KAFKA_PORT).await?
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

    #[tokio::test]
    async fn produce_and_consume_messages_jvm() -> Result<(), Box<dyn std::error::Error + 'static>>
    {
        let _ = pretty_env_logger::try_init();
        let kafka_node = apache::Kafka::default().with_jvm_image().start().await?;

        let bootstrap_servers = format!(
            "127.0.0.1:{}",
            kafka_node.get_host_port_ipv4(apache::KAFKA_PORT).await?
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
