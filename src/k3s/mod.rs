use std::collections::HashMap;
use std::io;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use testcontainers::core::{Mount, WaitFor};
use testcontainers::{Image, ImageArgs};

const NAME: &str = "rancher/k3s";
const TAG: &str = "v1.28.8-k3s1";
pub const TRAEFIK_HTTP: u16 = 80;
pub const KUBE_SECURE_PORT: u16 = 6443;
pub const RANCHER_WEBHOOK_PORT: u16 = 8443;

/// Module to work with [`K3s`] inside of tests.
///
/// Starts an instance of K3s, a single-node server fully-functional Kubernetes cluster
/// so you are able interact with the cluster using standard [`Kubernetes API`] exposed at [`KUBE_SECURE_PORT`] port
///
/// This module is based on the official [`K3s docker image`].
///
/// # Example
/// ```
/// use std::env::temp_dir;
/// use testcontainers::RunnableImage;
/// use testcontainers::runners::SyncRunner;
/// use testcontainers_modules::k3s::{K3s, KUBE_SECURE_PORT};
///
/// let k3s_instance = RunnableImage::from(K3s::default().with_conf_mount(&temp_dir()))
///            .with_privileged(true)
///            .with_userns_mode("host")
///            .start();
///
/// let kube_port = k3s_instance.get_host_port_ipv4(KUBE_SECURE_PORT);
/// let kube_conf = k3s_instance.image().read_kube_config().expect("Cannot read kube conf");
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
}

#[derive(Default, Debug, Clone)]
pub struct K3sArgs;

impl ImageArgs for K3sArgs {
    fn into_iterator(self) -> Box<dyn Iterator<Item = String>> {
        Box::new([String::from("server"), String::from("--snapshotter=native")].into_iter())
    }
}

impl Image for K3s {
    type Args = K3sArgs;

    fn name(&self) -> String {
        NAME.to_string()
    }

    fn tag(&self) -> String {
        TAG.to_string()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::StdErrMessage {
            message: String::from("Node controller sync successful"),
        }]
    }

    fn env_vars(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(self.env_vars.iter())
    }

    fn mounts(&self) -> Box<dyn Iterator<Item = &Mount> + '_> {
        let mut mounts = Vec::new();
        if let Some(conf_mount) = &self.conf_mount {
            mounts.push(conf_mount);
        }
        Box::new(mounts.into_iter())
    }

    fn expose_ports(&self) -> Vec<u16> {
        vec![KUBE_SECURE_PORT, RANCHER_WEBHOOK_PORT, TRAEFIK_HTTP]
    }
}

impl K3s {
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

    pub fn read_kube_config(&self) -> io::Result<String> {
        let k3s_conf_file_path = self
            .conf_mount
            .as_ref()
            .and_then(|mount| mount.source())
            .map(PathBuf::from)
            .map(|conf_dir| conf_dir.join("k3s.yaml"))
            .ok_or_else(|| io::Error::new(ErrorKind::InvalidData, "K3s conf dir is not mounted"))?;

        std::fs::read_to_string(&k3s_conf_file_path)
    }
}

#[cfg(test)]
mod tests {
    use std::env::temp_dir;

    use k8s_openapi::api::core::v1::Pod;
    use kube::api::ListParams;
    use kube::config::{KubeConfigOptions, Kubeconfig};
    use kube::{Api, Config, ResourceExt};
    use rustls::crypto::CryptoProvider;
    use testcontainers::runners::AsyncRunner;
    use testcontainers::{ContainerAsync, RunnableImage};

    use super::*;

    #[tokio::test]
    async fn k3s_pods() {
        let conf_dir = temp_dir();
        let k3s = RunnableImage::from(K3s::default().with_conf_mount(&conf_dir))
            .with_privileged(true)
            .with_userns_mode("host");

        let k3s_container = k3s.start().await;

        let client = get_kube_client(&k3s_container).await;

        let pods = Api::<Pod>::all(client)
            .list(&ListParams::default())
            .await
            .expect("Cannot read pods");

        assert!(
            pods.iter().any(|pod| pod.name_any().starts_with("coredns")),
            "coredns pod not found"
        );
        assert!(
            pods.iter()
                .any(|pod| pod.name_any().starts_with("metrics-server")),
            "metrics-server pod not found"
        );
        assert!(
            pods.iter()
                .any(|pod| pod.name_any().starts_with("local-path-provisioner")),
            "local-path-provisioner pod not found"
        );
    }

    pub async fn get_kube_client(container: &ContainerAsync<K3s>) -> kube::Client {
        if CryptoProvider::get_default().is_none() {
            rustls::crypto::ring::default_provider()
                .install_default()
                .expect("Error initializing rustls provider");
        }

        let conf_yaml = container
            .image()
            .read_kube_config()
            .expect("Error reading k3s.yaml");

        let mut config = Kubeconfig::from_yaml(&conf_yaml).expect("Error loading kube config");

        let port = container.get_host_port_ipv4(KUBE_SECURE_PORT).await;
        config.clusters.iter_mut().for_each(|cluster| {
            if let Some(server) = cluster.cluster.as_mut().and_then(|c| c.server.as_mut()) {
                *server = format!("https://127.0.0.1:{}", port)
            }
        });

        let client_config = Config::from_custom_kubeconfig(config, &KubeConfigOptions::default())
            .await
            .expect("Error building client config");

        kube::Client::try_from(client_config).expect("Error building client")
    }
}
