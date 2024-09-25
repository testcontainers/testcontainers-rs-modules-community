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
pub const SPANNER_PORT: u16 = 9010;

#[allow(missing_docs, reason = "not having docs here is currently allowed to adress the missing docs problem one place at a time. If you would like to help us, documenting one of these places helps other devs tremendously")]
#[derive(Debug, Clone)]
pub struct CloudSdkCmd {
    pub host: String,
    pub port: u16,
    pub emulator: Emulator,
}

#[allow(missing_docs, reason = "not having docs here is currently allowed to adress the missing docs problem one place at a time. If you would like to help us, documenting one of these places helps other devs tremendously")]
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

        args.into_iter()
    }
}

#[allow(missing_docs, reason = "not having docs here is currently allowed to adress the missing docs problem one place at a time. If you would like to help us, documenting one of these places helps other devs tremendously")]
#[derive(Debug, Clone)]
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
    fn new(port: u16, emulator: Emulator, ready_condition: WaitFor) -> Self {
        let cmd = CloudSdkCmd {
            host: HOST.to_owned(),
            port,
            emulator,
        };
        Self {
            exposed_ports: vec![ContainerPort::Tcp(port)],
            ready_condition,
            cmd,
        }
    }

    #[allow(missing_docs, reason = "not having docs here is currently allowed to adress the missing docs problem one place at a time. If you would like to help us, documenting one of these places helps other devs tremendously")]
    pub fn bigtable() -> Self {
        Self::new(
            BIGTABLE_PORT,
            Emulator::Bigtable,
            WaitFor::message_on_stderr("[bigtable] Cloud Bigtable emulator running on"),
        )
    }

    #[allow(missing_docs, reason = "not having docs here is currently allowed to adress the missing docs problem one place at a time. If you would like to help us, documenting one of these places helps other devs tremendously")]
    pub fn firestore() -> Self {
        Self::new(
            FIRESTORE_PORT,
            Emulator::Firestore,
            WaitFor::message_on_stderr("[firestore] Dev App Server is now running"),
        )
    }

    #[allow(missing_docs, reason = "not having docs here is currently allowed to adress the missing docs problem one place at a time. If you would like to help us, documenting one of these places helps other devs tremendously")]
    pub fn datastore(project: impl Into<String>) -> Self {
        let project = project.into();
        Self::new(
            DATASTORE_PORT,
            Emulator::Datastore { project },
            WaitFor::message_on_stderr("[datastore] Dev App Server is now running"),
        )
    }

    #[allow(missing_docs, reason = "not having docs here is currently allowed to adress the missing docs problem one place at a time. If you would like to help us, documenting one of these places helps other devs tremendously")]
    pub fn pubsub() -> Self {
        Self::new(
            PUBSUB_PORT,
            Emulator::PubSub,
            WaitFor::message_on_stderr("[pubsub] INFO: Server started, listening on"),
        )
    }

    #[allow(missing_docs, reason = "not having docs here is currently allowed to adress the missing docs problem one place at a time. If you would like to help us, documenting one of these places helps other devs tremendously")]
    pub fn spanner() -> Self {
        Self::new(
            SPANNER_PORT, // gRPC port
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
        let port = node.get_host_port_ipv4(google_cloud_sdk_emulators::SPANNER_PORT)?;
        assert!(RANDOM_PORTS.contains(&port), "Port {port} not found");
        Ok(())
    }
}
