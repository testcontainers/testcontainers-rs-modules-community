use std::{
    borrow::Cow,
    collections::HashMap,
    io,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use testcontainers::{
    core::{ContainerPort, Mount, WaitFor},
    Image,
};

const NAME: &str = "rancher/k3s";
const TAG: &str = "v1.28.8-k3s1";
/// Port that the [`traefik`] part of the container has internally
/// Can be rebound externally via [`testcontainers::core::ImageExt::with_mapped_port`]
///
/// [`traefik`]: https://doc.traefik.io/traefik/
pub const TRAEFIK_HTTP: ContainerPort = ContainerPort::Tcp(80);
/// Port that the [`Kubernetes`] part of the container has internally
/// Can be rebound externally via [`testcontainers::core::ImageExt::with_mapped_port`]
///
/// [`Kubernetes`]: https://kubernetes.io/
pub const KUBE_SECURE_PORT: ContainerPort = ContainerPort::Tcp(6443);
/// Port that the [`Rancher`] part of the container has internally
/// Can be rebound externally via [`testcontainers::core::ImageExt::with_mapped_port`]
///
/// [`Rancher`]: https://rancher.io/
pub const RANCHER_WEBHOOK_PORT: ContainerPort = ContainerPort::Tcp(8443);

/// Module to work with [`K3s`] inside of tests.
///
/// Starts an instance of K3s, a single-node server fully-functional Kubernetes cluster
/// so you are able to interact with the cluster using standard [`Kubernetes API`] exposed at [`KUBE_SECURE_PORT`] port
///
/// This module is based on the official [`K3s docker image`].
///
/// # Example
/// ```
/// use std::env::temp_dir;
///
/// use testcontainers_modules::{
///     k3s::{K3s, KUBE_SECURE_PORT},
///     testcontainers::{runners::SyncRunner, ImageExt},
/// };
///
/// let k3s_instance = K3s::default()
///     .with_conf_mount(&temp_dir())
///     .with_privileged(true)
///     .with_userns_mode("host")
///     .start()
///     .unwrap();
///
/// let kube_port = k3s_instance.get_host_port_ipv4(KUBE_SECURE_PORT);
/// let kube_conf = k3s_instance
///     .image()
///     .read_kube_config()
///     .expect("Cannot read kube conf");
/// // use kube_port and kube_conf to connect and control k3s cluster
/// ```
///
/// [`K3s`]: https://k3s.io/
/// [`Kubernetes API`]: https://kubernetes.io/docs/concepts/overview/kubernetes-api/
/// [`K3s docker image`]: https://hub.docker.com/r/rancher/k3s
#[derive(Debug, Default, Clone)]
pub struct K3s {
    env_vars: HashMap<String, String>,
    conf_mount: Option<Mount>,
    cmd: K3sCmd,
}

#[allow(missing_docs, reason = "not having docs here is currently allowed to adress the missing docs problem one place at a time. If you would like to help us, documenting one of these places helps other devs tremendously")]
#[derive(Debug, Clone)]
pub struct K3sCmd {
    snapshotter: String,
}

impl K3sCmd {
    #[allow(missing_docs, reason = "not having docs here is currently allowed to adress the missing docs problem one place at a time. If you would like to help us, documenting one of these places helps other devs tremendously")]
    pub fn with_snapshotter(self, snapshotter: impl Into<String>) -> Self {
        Self {
            snapshotter: snapshotter.into(),
        }
    }
}

impl Default for K3sCmd {
    fn default() -> Self {
        Self {
            snapshotter: String::from("native"),
        }
    }
}

impl Image for K3s {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stderr(
            "Node controller sync successful",
        )]
    }

    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        &self.env_vars
    }

    fn mounts(&self) -> impl IntoIterator<Item = &Mount> {
        let mut mounts = Vec::new();
        if let Some(conf_mount) = &self.conf_mount {
            mounts.push(conf_mount);
        }
        mounts
    }

    fn cmd(&self) -> impl IntoIterator<Item = impl Into<Cow<'_, str>>> {
        &self.cmd
    }

    fn expose_ports(&self) -> &[ContainerPort] {
        &[KUBE_SECURE_PORT, RANCHER_WEBHOOK_PORT, TRAEFIK_HTTP]
    }
}

impl K3s {
    #[allow(missing_docs, reason = "not having docs here is currently allowed to adress the missing docs problem one place at a time. If you would like to help us, documenting one of these places helps other devs tremendously")]
    pub fn with_conf_mount(mut self, conf_mount_path: impl AsRef<Path>) -> Self {
        self.env_vars
            .insert(String::from("K3S_KUBECONFIG_MODE"), String::from("644"));
        Self {
            conf_mount: Some(Mount::bind_mount(
                conf_mount_path.as_ref().to_str().unwrap_or_default(),
                "/etc/rancher/k3s/",
            )),
            ..self
        }
    }

    #[allow(missing_docs, reason = "not having docs here is currently allowed to adress the missing docs problem one place at a time. If you would like to help us, documenting one of these places helps other devs tremendously")]
    pub fn read_kube_config(&self) -> io::Result<String> {
        let k3s_conf_file_path = self
            .conf_mount
            .as_ref()
            .and_then(|mount| mount.source())
            .map(PathBuf::from)
            .map(|conf_dir| conf_dir.join("k3s.yaml"))
            .ok_or_else(|| io::Error::new(ErrorKind::InvalidData, "K3s conf dir is not mounted"))?;

        std::fs::read_to_string(k3s_conf_file_path)
    }
}

impl IntoIterator for &K3sCmd {
    type Item = String;
    type IntoIter = <Vec<String> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        let mut cmd = vec![String::from("server")];
        cmd.push(format!("--snapshotter={}", self.snapshotter));
        cmd.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use std::env::temp_dir;

    use k8s_openapi::api::core::v1::Pod;
    use kube::{
        api::ListParams,
        config::{KubeConfigOptions, Kubeconfig},
        Api, Config, ResourceExt,
    };
    use rustls::crypto::CryptoProvider;
    use testcontainers::{runners::AsyncRunner, ContainerAsync, ImageExt};

    use super::*;

    #[tokio::test]
    async fn k3s_pods() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let conf_dir = temp_dir();
        let k3s = K3s::default()
            .with_conf_mount(&conf_dir)
            .with_privileged(true)
            .with_userns_mode("host");

        let k3s_container = k3s.start().await?;

        let client = get_kube_client(&k3s_container).await?;

        let pods = Api::<Pod>::all(client)
            .list(&ListParams::default())
            .await
            .expect("Cannot read pods");

        let pod_names = pods
            .into_iter()
            .map(|pod| pod.name_any())
            .collect::<Vec<_>>();

        assert!(
            pod_names
                .iter()
                .any(|pod_name| pod_name.starts_with("coredns")),
            "coredns pod not found - found pods {pod_names:?}"
        );
        assert!(
            pod_names
                .iter()
                .any(|pod_name| pod_name.starts_with("metrics-server")),
            "metrics-server pod not found - found pods {pod_names:?}"
        );
        assert!(
            pod_names
                .iter()
                .any(|pod_name| pod_name.starts_with("local-path-provisioner")),
            "local-path-provisioner pod not found - found pods {pod_names:?}"
        );
        Ok(())
    }

    pub async fn get_kube_client(
        container: &ContainerAsync<K3s>,
    ) -> Result<kube::Client, Box<dyn std::error::Error + 'static>> {
        if CryptoProvider::get_default().is_none() {
            rustls::crypto::ring::default_provider()
                .install_default()
                .expect("Error initializing rustls provider");
        }

        let conf_yaml = container.image().read_kube_config()?;

        let mut config = Kubeconfig::from_yaml(&conf_yaml).expect("Error loading kube config");

        let port = container.get_host_port_ipv4(KUBE_SECURE_PORT).await?;
        config.clusters.iter_mut().for_each(|cluster| {
            if let Some(server) = cluster.cluster.as_mut().and_then(|c| c.server.as_mut()) {
                *server = format!("https://127.0.0.1:{}", port)
            }
        });

        let client_config =
            Config::from_custom_kubeconfig(config, &KubeConfigOptions::default()).await?;

        Ok(kube::Client::try_from(client_config)?)
    }
}
