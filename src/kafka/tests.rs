use std::time::Duration;

use futures::StreamExt;
use rdkafka::{
    consumer::{Consumer, StreamConsumer},
    producer::{FutureProducer, FutureRecord},
    ClientConfig, Message,
};
use testcontainers::{
    core::{ContainerPort, ExecCommand},
    runners::AsyncRunner,
    ContainerRequest, GenericImage, Image, ImageExt,
};

pub(super) async fn produce_and_consume_messages_with_peer<I: Image>(
    case: &str,
    host_port: ContainerPort,
    peer_port: u16,
    configure: impl FnOnce(&str) -> ContainerRequest<I>,
) -> Result<(), Box<dyn std::error::Error + 'static>> {
    let _ = pretty_env_logger::try_init();
    let broker_hostname = format!("kafka-{case}-{}", std::process::id());
    let network = format!("testcontainers-kafka-{case}-{}", std::process::id());
    let kafka_node = configure(&broker_hostname)
        .with_network(&network)
        .with_container_name(&broker_hostname)
        .start()
        .await?;
    let peer_node = GenericImage::new("apache/kafka", "3.8.0")
        .with_entrypoint("bash")
        .with_cmd(["-c", "sleep infinity"])
        .with_network(&network)
        .with_container_name(format!("kafka-peer-{case}-{}", std::process::id()))
        .start()
        .await?;

    let bootstrap_servers = format!(
        "127.0.0.1:{}",
        kafka_node.get_host_port_ipv4(host_port).await?
    );
    let producer = ClientConfig::new()
        .set("bootstrap.servers", &bootstrap_servers)
        .set("message.timeout.ms", "5000")
        .create::<FutureProducer>()
        .expect("Failed to create Kafka FutureProducer");
    let consumer = ClientConfig::new()
        .set("group.id", "testcontainer-rs-peer")
        .set("bootstrap.servers", &bootstrap_servers)
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "earliest")
        .create::<StreamConsumer>()
        .expect("Failed to create Kafka StreamConsumer");

    producer
        .send(
            FutureRecord::to("host-to-peer")
                .payload("host-message")
                .key("host-key"),
            Duration::from_secs(0),
        )
        .await
        .unwrap_or_else(|error| panic!("{case}: host produce failed: {error:?}"));

    let broker_address = format!("{broker_hostname}:{peer_port}");
    let (stdout, stderr, exit_code) = tokio::time::timeout(Duration::from_secs(30), async {
        let mut result = peer_node
            .exec(ExecCommand::new([
                "/opt/kafka/bin/kafka-console-consumer.sh",
                "--bootstrap-server",
                &broker_address,
                "--topic",
                "host-to-peer",
                "--from-beginning",
                "--max-messages",
                "1",
                "--timeout-ms",
                "10000",
            ]))
            .await?;
        let stdout = result.stdout_to_vec().await?;
        let stderr = result.stderr_to_vec().await?;
        let exit_code = result.exit_code().await?;
        Ok::<_, testcontainers::TestcontainersError>((stdout, stderr, exit_code))
    })
    .await
    .unwrap_or_else(|_| panic!("{case}: peer consumer timed out"))?;
    assert_eq!(
        stdout,
        b"host-message\n",
        "{case}: peer consumer stderr: {}",
        String::from_utf8_lossy(&stderr)
    );
    assert_eq!(
        exit_code,
        Some(0),
        "{case}: peer consumer stderr: {}",
        String::from_utf8_lossy(&stderr)
    );

    let (stderr, exit_code) = tokio::time::timeout(Duration::from_secs(30), async {
        let mut result = peer_node
            .exec(ExecCommand::new([
                "sh",
                "-c",
                r#"printf '%s\n' 'peer-message' | /opt/kafka/bin/kafka-console-producer.sh --bootstrap-server "$1" --topic peer-to-host --sync --producer-property max.block.ms=10000 --producer-property request.timeout.ms=5000 --producer-property delivery.timeout.ms=10000"#,
                "kafka-peer-producer",
                &broker_address,
            ]))
            .await?;
        let _ = result.stdout_to_vec().await?;
        let stderr = result.stderr_to_vec().await?;
        let exit_code = result.exit_code().await?;
        Ok::<_, testcontainers::TestcontainersError>((stderr, exit_code))
    })
    .await
    .unwrap_or_else(|_| panic!("{case}: peer producer timed out"))?;
    assert_eq!(
        exit_code,
        Some(0),
        "{case}: peer producer stderr: {}",
        String::from_utf8_lossy(&stderr)
    );

    consumer
        .subscribe(&["peer-to-host"])
        .expect("Failed to subscribe to a topic");
    let mut message_stream = consumer.stream();
    let message = tokio::time::timeout(Duration::from_secs(10), message_stream.next())
        .await
        .unwrap_or_else(|_| panic!("{case}: host consumer timed out"))
        .expect("Kafka message stream ended")?;
    assert_eq!(
        message.payload(),
        Some(b"peer-message".as_slice()),
        "{case}"
    );

    Ok(())
}
