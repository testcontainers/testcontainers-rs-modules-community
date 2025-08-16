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

/// Configuration for K3s server command-line arguments.
///
/// This struct allows you to customize the K3s server startup configuration
/// by setting various options like the container snapshotter.
#[derive(Debug, Clone)]
pub struct K3sCmd {
    snapshotter: String,
}

impl K3sCmd {
    /// Sets the container snapshotter for the K3s server.
    ///
    /// The snapshotter is responsible for managing container filesystem snapshots.
    /// Common values include "overlayfs", "fuse-overlayfs", or "native".
    ///
    /// # Example
    /// ```
    /// use testcontainers_modules::k3s::K3sCmd;
    ///
    /// let cmd = K3sCmd::default().with_snapshotter("overlayfs");
    /// ```
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
    /// Mounts a host directory to the K3s configuration directory.
    ///
    /// This allows you to access the K3s configuration files (like kubeconfig)
    /// from the host filesystem. The kubeconfig file will be created at
    /// `{conf_mount_path}/k3s.yaml` and can be read using [`read_kube_config`](Self::read_kube_config).
    ///
    /// # Example
    /// ```
    /// use testcontainers_modules::k3s::K3s;
    /// use std::path::Path;
    ///
    /// let k3s = K3s::default().with_conf_mount(Path::new("/tmp/k3s-config"));
    /// ```
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

    /// Reads the kubeconfig file from the mounted configuration directory.
    ///
    /// This method reads the `k3s.yaml` file from the mounted configuration directory
    /// that was set up using [`with_conf_mount`](Self::with_conf_mount).
    /// The kubeconfig can be used to connect kubectl or other Kubernetes clients to the K3s cluster.
    ///
    /// # Example
    /// ```no_run
    /// use testcontainers_modules::k3s::K3s;
    /// use std::path::Path;
    ///
    /// let k3s = K3s::default().with_conf_mount(Path::new("/tmp/k3s-config"));
    /// // After starting the container...
    /// let kubeconfig = k3s.read_kube_config().expect("Failed to read kubeconfig");
    /// ```
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
                *server = format!("https://127.0.0.1:{port}")
            }
        });

        let client_config =
            Config::from_custom_kubeconfig(config, &KubeConfigOptions::default()).await?;

        Ok(kube::Client::try_from(client_config)?)
    }
}
