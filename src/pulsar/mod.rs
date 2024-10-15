use std::{borrow::Cow, collections::BTreeMap};

use testcontainers::{
    core::{CmdWaitFor, ContainerPort, ContainerState, ExecCommand, Mount, WaitFor},
    Image, TestcontainersError,
};

const NAME: &str = "apachepulsar/pulsar";
const TAG: &str = "2.10.6";

const PULSAR_PORT: ContainerPort = ContainerPort::Tcp(6650);
const ADMIN_PORT: ContainerPort = ContainerPort::Tcp(8080);

/// Module to work with [`Apache Pulsar`] inside of tests.
///
/// This module is based on the official [`Apache Pulsar docker image`].
///
/// # Example
/// ```
/// use testcontainers_modules::{pulsar, testcontainers::runners::SyncRunner};
///
/// let pulsar = pulsar::Pulsar::default().start().unwrap();
/// let http_port = pulsar.get_host_port_ipv4(6650).unwrap();
///
/// // do something with the running pulsar instance..
/// ```
///
/// [`Apache Pulsar`]: https://github.com/apache/pulsar
/// [`Apache Pulsar docker image`]: https://hub.docker.com/r/apachepulsar/pulsar/
#[derive(Debug, Clone)]
pub struct Pulsar {
    data_mount: Mount,
    env: BTreeMap<String, String>,
    admin_commands: Vec<Vec<String>>,
}

impl Default for Pulsar {
    /**
     * Starts an in-memory instance in dev mode, with horrible token values.
     * Obviously not to be emulated in production.
     */
    fn default() -> Self {
        Self {
            data_mount: Mount::tmpfs_mount("/pulsar/data"),
            env: BTreeMap::new(),
            admin_commands: vec![],
        }
    }
}

impl Pulsar {
    /// Add configuration parameter to Pulsar `conf/standalone.conf`
    pub fn with_config(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.env
            .insert(format!("PULSAR_PREFIX_{}", name.into()), value.into());
        self
    }

    /// Runs admin command after container start
    pub fn with_admin_command(mut self, command: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let mut vec: Vec<String> = command.into_iter().map(Into::into).collect();
        vec.insert(0, "bin/pulsar-admin".to_string());
        self.admin_commands.push(vec);
        self
    }

    /// Creates tenant after container start
    pub fn with_tenant(self, tenant: impl Into<String>) -> Self {
        let tenant = tenant.into();
        self.with_admin_command(["tenants", "create", &tenant])
    }

    /// Creates namespace after container start
    pub fn with_namespace(self, namespace: impl Into<String>) -> Self {
        let namespace = namespace.into();
        self.with_admin_command(["namespaces", "create", &namespace])
    }

    /// Creates topic after container start
    pub fn with_topic(self, topic: impl Into<String>) -> Self {
        let topic = topic.into();
        self.with_admin_command(["topics", "create", &topic])
    }
}

impl Image for Pulsar {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![
            WaitFor::message_on_stdout("HTTP Service started at"),
            WaitFor::message_on_stdout("messaging service is ready"),
        ]
    }

    fn mounts(&self) -> impl IntoIterator<Item = &Mount> {
        [&self.data_mount]
    }

    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        &self.env
    }

    fn cmd(&self) -> impl IntoIterator<Item = impl Into<Cow<'_, str>>> {
        [
            "sh",
            "-c",
            "bin/apply-config-from-env.py conf/standalone.conf && bin/pulsar standalone",
        ]
    }

    fn exec_after_start(
        &self,
        _cs: ContainerState,
    ) -> Result<Vec<ExecCommand>, TestcontainersError> {
        Ok(self
            .admin_commands
            .iter()
            .map(|cmd| ExecCommand::new(cmd).with_cmd_ready_condition(CmdWaitFor::exit_code(0)))
            .collect())
    }

    fn expose_ports(&self) -> &[ContainerPort] {
        &[PULSAR_PORT, ADMIN_PORT]
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use pulsar::{
        producer::Message, Consumer, DeserializeMessage, Error, Payload, SerializeMessage,
        TokioExecutor,
    };
    use serde::{Deserialize, Serialize};

    use super::*;
    use crate::testcontainers::runners::AsyncRunner;

    #[derive(Serialize, Deserialize)]
    struct TestData {
        data: String,
    }

    impl DeserializeMessage for TestData {
        type Output = Result<TestData, serde_json::Error>;

        fn deserialize_message(payload: &Payload) -> Self::Output {
            serde_json::from_slice(&payload.data)
        }
    }

    impl SerializeMessage for TestData {
        fn serialize_message(input: Self) -> Result<Message, Error> {
            Ok(Message {
                payload: serde_json::to_vec(&input).map_err(|e| Error::Custom(e.to_string()))?,
                ..Default::default()
            })
        }
    }

    #[tokio::test]
    async fn pulsar_subscribe_and_publish() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let topic = "persistent://test/test-ns/test-topic";

        let pulsar = Pulsar::default()
            .with_tenant("test")
            .with_namespace("test/test-ns")
            .with_topic(topic)
            .start()
            .await
            .unwrap();

        let endpoint = format!(
            "pulsar://0.0.0.0:{}",
            pulsar.get_host_port_ipv4(6650).await?
        );
        let client = pulsar::Pulsar::builder(endpoint, TokioExecutor)
            .build()
            .await?;

        let mut consumer: Consumer<TestData, _> =
            client.consumer().with_topic(topic).build().await?;

        let mut producer = client.producer().with_topic(topic).build().await?;

        producer
            .send_non_blocking(TestData {
                data: "test".to_string(),
            })
            .await?
            .await?;

        let data = consumer.next().await.unwrap()?.deserialize()?;
        assert_eq!("test", data.data);

        Ok(())
    }

    #[tokio::test]
    async fn pulsar_config() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let topic = "persistent://test/test-ns/test-topic";

        let pulsar = Pulsar::default()
            .with_tenant("test")
            .with_namespace("test/test-ns")
            .with_config("allowAutoTopicCreation", "false")
            .start()
            .await
            .unwrap();

        let endpoint = format!(
            "pulsar://0.0.0.0:{}",
            pulsar.get_host_port_ipv4(6650).await?
        );
        let client = pulsar::Pulsar::builder(endpoint, TokioExecutor)
            .build()
            .await?;

        let producer = client.producer().with_topic(topic).build().await;

        match producer {
            Ok(_) => panic!("Producer should return error"),
            Err(e) => assert_eq!("Connection error: Server error (Some(TopicNotFound)): Topic not found persistent://test/test-ns/test-topic", e.to_string()),
        }

        Ok(())
    }
}
