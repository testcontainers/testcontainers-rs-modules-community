use std::borrow::Cow;

use testcontainers::{
    core::{ContainerPort, WaitFor},
    Image,
};

const NAME: &str = "google/cloud-sdk";
const TAG: &str = "362.0.0-emulators";

const HOST: &str = "0.0.0.0";
/// Port that the [`Bigtable`] emulator container has internally
/// Can be rebound externally via [`testcontainers::core::ImageExt::with_mapped_port`]
///
/// [`Bigtable`]: https://cloud.google.com/bigtable
pub const BIGTABLE_PORT: u16 = 8086;
/// Port that the [`Datastore`] emulator container has internally
/// Can be rebound externally via [`testcontainers::core::ImageExt::with_mapped_port`]
///
/// [`Datastore`]: https://cloud.google.com/datastore
pub const DATASTORE_PORT: u16 = 8081;
/// Port that the [`Firestore`] emulator container has internally
/// Can be rebound externally via [`testcontainers::core::ImageExt::with_mapped_port`]
///
/// [`Firestore`]: https://cloud.google.com/firestore
pub const FIRESTORE_PORT: u16 = 8080;
/// Port that the [`Pub/Sub`] emulator container has internally
/// Can be rebound externally via [`testcontainers::core::ImageExt::with_mapped_port`]
///
/// [`Pub/Sub`]: https://cloud.google.com/pubsub
pub const PUBSUB_PORT: u16 = 8085;
/// Port that the [`Spanner`] emulator container has internally
/// Can be rebound externally via [`testcontainers::core::ImageExt::with_mapped_port`]
///
/// [`Spanner`]: https://cloud.google.com/spanner
#[deprecated(since = "0.3.8", note = "please use `SPANNER_GRPC_PORT` instead")]
pub const SPANNER_PORT: u16 = SPANNER_GRPC_PORT;
/// Port that the [`Spanner`] emulator container has internally (gRPC)
/// Can be rebound externally via [`testcontainers::core::ImageExt::with_mapped_port`]
///
/// [`Spanner`]: https://cloud.google.com/spanner
pub const SPANNER_GRPC_PORT: u16 = 9010;
/// Port that the [`Spanner`] emulator container has internally (REST)
/// Can be rebound externally via [`testcontainers::core::ImageExt::with_mapped_port`]
///
/// [`Spanner`]: https://cloud.google.com/spanner
pub const SPANNER_REST_PORT: u16 = 9020;

#[allow(missing_docs)]
// not having docs here is currently allowed to address the missing docs problem one place at a time. Helping us by documenting just one of these places helps other devs tremendously
#[derive(Debug, Clone)]
pub struct CloudSdkCmd {
    pub host: String,
    pub port: u16,
    pub rest_port: Option<u16>,
    pub emulator: Emulator,
}

#[allow(missing_docs)]
// not having docs here is currently allowed to address the missing docs problem one place at a time. Helping us by documenting just one of these places helps other devs tremendously
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Emulator {
    Bigtable,
    Datastore { project: String },
    Firestore,
    PubSub,
    Spanner,
}

impl IntoIterator for &CloudSdkCmd {
    type Item = String;
    type IntoIter = <Vec<String> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        let (emulator, project) = match &self.emulator {
            Emulator::Bigtable => ("bigtable", None),
            Emulator::Datastore { project } => ("datastore", Some(project)),
            Emulator::Firestore => ("firestore", None),
            Emulator::PubSub => ("pubsub", None),
            Emulator::Spanner => ("spanner", None),
        };
        let mut args = vec![
            "gcloud".to_owned(),
            "beta".to_owned(),
            "emulators".to_owned(),
            emulator.to_owned(),
            "start".to_owned(),
        ];
        if let Some(project) = project {
            args.push("--project".to_owned());
            args.push(project.to_owned());
        }
        args.push("--host-port".to_owned());
        args.push(format!("{}:{}", self.host, self.port));

        if let Some(rest_port) = self.rest_port {
            args.push("--rest-port".to_owned());
            args.push(rest_port.to_string());
        }

        args.into_iter()
    }
}

/// Module to work with [`Google Cloud Emulators`] inside of tests.
///
/// The same image can be used to run multiple emulators, using the `emulator` argument allows
/// selecting the one to run.
///
/// This module is based on the official [`GCloud SDK image`].
///
/// # Example
/// ```
/// use testcontainers::runners::SyncRunner;
/// use testcontainers_modules::google_cloud_sdk_emulators;
///
/// let container = google_cloud_sdk_emulators::CloudSdk::spanner().start().unwrap();
/// let port = container.get_host_port_ipv4(google_cloud_sdk_emulators::SPANNER_REST_PORT).unwrap();
///
/// let spanner_host = format!("localhost:{port}");
///
/// // do something with the started spanner instance.
/// ```
///
/// [`Google Cloud Emulators`]: https://cloud.google.com/sdk/gcloud/reference/beta/emulators
/// [`GCloud SDK image`]: https://cloud.google.com/sdk/docs/downloads-docker#[derive(Debug, Clone)]
pub struct CloudSdk {
    exposed_ports: Vec<ContainerPort>,
    ready_condition: WaitFor,
    cmd: CloudSdkCmd,
}

impl Image for CloudSdk {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![self.ready_condition.clone()]
    }

    fn cmd(&self) -> impl IntoIterator<Item = impl Into<Cow<'_, str>>> {
        &self.cmd
    }

    fn expose_ports(&self) -> &[ContainerPort] {
        &self.exposed_ports
    }
}

impl CloudSdk {
    fn new(
        port: u16,
        rest_port: Option<u16>,
        emulator: Emulator,
        ready_condition: WaitFor,
    ) -> Self {
        let cmd = CloudSdkCmd {
            host: HOST.to_owned(),
            port,
            rest_port,
            emulator,
        };
        let mut exposed_ports = vec![ContainerPort::Tcp(port)];
        exposed_ports.extend(rest_port.map(ContainerPort::Tcp));
        Self {
            exposed_ports,
            ready_condition,
            cmd,
        }
    }

    // not having docs here is currently allowed to address the missing docs problem one place at a time. Helping us by documenting just one of these places helps other devs tremendously
    #[allow(missing_docs)]
    pub fn bigtable() -> Self {
        Self::new(
            BIGTABLE_PORT,
            None,
            Emulator::Bigtable,
            WaitFor::message_on_stderr("[bigtable] Cloud Bigtable emulator running on"),
        )
    }

    // not having docs here is currently allowed to address the missing docs problem one place at a time. Helping us by documenting just one of these places helps other devs tremendously
    #[allow(missing_docs)]
    pub fn firestore() -> Self {
        Self::new(
            FIRESTORE_PORT,
            None,
            Emulator::Firestore,
            WaitFor::message_on_stderr("[firestore] Dev App Server is now running"),
        )
    }

    // not having docs here is currently allowed to address the missing docs problem one place at a time. Helping us by documenting just one of these places helps other devs tremendously
    #[allow(missing_docs)]
    pub fn datastore(project: impl Into<String>) -> Self {
        let project = project.into();
        Self::new(
            DATASTORE_PORT,
            None,
            Emulator::Datastore { project },
            WaitFor::message_on_stderr("[datastore] Dev App Server is now running"),
        )
    }

    // not having docs here is currently allowed to address the missing docs problem one place at a time. Helping us by documenting just one of these places helps other devs tremendously
    #[allow(missing_docs)]
    pub fn pubsub() -> Self {
        Self::new(
            PUBSUB_PORT,
            None,
            Emulator::PubSub,
            WaitFor::message_on_stderr("[pubsub] INFO: Server started, listening on"),
        )
    }

    // not having docs here is currently allowed to address the missing docs problem one place at a time. Helping us by documenting just one of these places helps other devs tremendously
    #[allow(missing_docs)]
    pub fn spanner() -> Self {
        Self::new(
            SPANNER_GRPC_PORT,
            Some(SPANNER_REST_PORT),
            Emulator::Spanner,
            WaitFor::message_on_stderr("Cloud Spanner emulator running"),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Range;

    use crate::{google_cloud_sdk_emulators, testcontainers::runners::SyncRunner};

    const RANDOM_PORTS: Range<u16> = 32768..65535;

    #[test]
    fn bigtable_emulator_expose_port() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let _ = pretty_env_logger::try_init();
        let node = (google_cloud_sdk_emulators::CloudSdk::bigtable()).start()?;
        let port = node.get_host_port_ipv4(google_cloud_sdk_emulators::BIGTABLE_PORT)?;
        assert!(RANDOM_PORTS.contains(&port), "Port {port} not found");
        Ok(())
    }

    #[test]
    fn datastore_emulator_expose_port() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let _ = pretty_env_logger::try_init();
        let node = google_cloud_sdk_emulators::CloudSdk::datastore("test").start()?;
        let port = node.get_host_port_ipv4(google_cloud_sdk_emulators::DATASTORE_PORT)?;
        assert!(RANDOM_PORTS.contains(&port), "Port {port} not found");
        Ok(())
    }

    #[test]
    fn firestore_emulator_expose_port() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let _ = pretty_env_logger::try_init();
        let node = google_cloud_sdk_emulators::CloudSdk::firestore().start()?;
        let port = node.get_host_port_ipv4(google_cloud_sdk_emulators::FIRESTORE_PORT)?;
        assert!(RANDOM_PORTS.contains(&port), "Port {port} not found");
        Ok(())
    }

    #[test]
    fn pubsub_emulator_expose_port() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let _ = pretty_env_logger::try_init();
        let node = google_cloud_sdk_emulators::CloudSdk::pubsub().start()?;
        let port = node.get_host_port_ipv4(google_cloud_sdk_emulators::PUBSUB_PORT)?;
        assert!(RANDOM_PORTS.contains(&port), "Port {port} not found");
        Ok(())
    }

    #[test]
    fn spanner_emulator_expose_port() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let _ = pretty_env_logger::try_init();
        let node = google_cloud_sdk_emulators::CloudSdk::spanner().start()?;
        let port = node.get_host_port_ipv4(google_cloud_sdk_emulators::SPANNER_GRPC_PORT)?;
        assert!(RANDOM_PORTS.contains(&port), "Port {port} not found");
        let port = node.get_host_port_ipv4(google_cloud_sdk_emulators::SPANNER_REST_PORT)?;
        assert!(RANDOM_PORTS.contains(&port), "Port {port} not found");
        Ok(())
    }
}
