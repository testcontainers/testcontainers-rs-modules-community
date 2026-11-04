use std::{
    borrow::Cow,
    collections::HashMap,
    io::{self, ErrorKind, Read},
    path::{Path, PathBuf},
};

use futures::StreamExt;
// TODO: export path… not yet happy
pub use k3s_config::{Mirror, Registries};
use testcontainers::{
    bollard::query_parameters::DownloadFromContainerOptionsBuilder,
    core::{ContainerPort, CopyDataSource, CopyToContainer, Mount, WaitFor},
    ContainerAsync, CopyTargetOptions, Image,
};

mod k3d_config;
mod k3s_config;

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
    has_k3d_fixes: bool,
    k3d_image: Option<String>,
    copy_to_sources: Vec<CopyToContainer>,
}

/// Configuration for K3s server command-line arguments.
///
/// This struct allows you to customize the K3s server startup configuration
/// by setting various options like the container snapshotter.
#[derive(Debug, Clone)]
pub struct K3sCmd {
    snapshotter: String,
    extra_args: Vec<String>,
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
            extra_args: Vec::new(),
        }
    }
}

impl Default for K3sCmd {
    fn default() -> Self {
        Self {
            snapshotter: String::from("native"),
            extra_args: Vec::new(),
        }
    }
}

impl Image for K3s {
    fn name(&self) -> &str {
        match self.k3d_image.as_ref() {
            None => NAME,
            Some(image) => image
                .rmatch_indices(":")
                .next()
                .map(|(index, _pattern)| image.split_at(index).0)
                .unwrap_or(NAME),
        }
    }

    fn tag(&self) -> &str {
        match self.k3d_image.as_ref() {
            None => TAG,
            Some(image) => image
                .rmatch_indices(":")
                .next()
                .map(|(index, pattern)| image.split_at(index + pattern.len()).1)
                .unwrap_or(TAG),
        }
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

    fn entrypoint(&self) -> Option<&str> {
        if self.has_k3d_fixes {
            Some("/bin/k3d-entrypoint.sh")
        } else {
            None
        }
    }

    fn copy_to_sources(&self) -> impl IntoIterator<Item = &CopyToContainer> {
        &self.copy_to_sources
    }
}

/// The error type for inferring the K3s config from K3d config.
#[derive(Debug)]
pub enum K3dInferError {
    /// The value in `K3D_CONFIG` cannot be read.
    VarError(std::env::VarError),
    /// The access to the `K3D_CONFIG` failed.
    Io(std::io::Error),
    /// Invalid YAML config in `K3D_CONFIG`
    Yaml((noyalib::Error, String)),
}

impl std::error::Error for K3dInferError {}

impl std::fmt::Display for K3dInferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "Cannot access k3d config: {err}")?,
            Self::VarError(err) => write!(f, "Cannot access K3D_CONFIG var: {err}")?,
            Self::Yaml((err, doc)) => {
                let err = err.render(doc);
                write!(f, "Cannot read k3d config from YAML: {err}")?
            }
        }
        Ok(())
    }
}

impl K3s {
    /// TODO:
    // TODO: error type…
    pub fn infer_from_k3d_config() -> Result<Self, K3dInferError> {
        let config = match std::env::var("K3D_CONFIG") {
            Ok(k3d_config_path) => {
                let mut f = std::fs::File::open(k3d_config_path).map_err(K3dInferError::Io)?;

                let mut yaml = String::new();
                f.read_to_string(&mut yaml).map_err(K3dInferError::Io)?;

                let config: k3d_config::K3dConfig =
                    noyalib::from_str(&yaml).map_err(|err| K3dInferError::Yaml((err, yaml)))?;
                config
            }
            Err(std::env::VarError::NotPresent) => Default::default(),
            Err(e) => {
                return Err(K3dInferError::VarError(e));
            }
        };

        let k3s = Self::default().with_k3d_fixes();

        let mut k3s = match config.registries.config {
            Some(registries) => k3s.with_registries(registries),
            None => k3s,
        };

        if let Some(files) = config.files {
            k3s.copy_to_sources.extend(files.into_iter().map(|file| {
                CopyToContainer::new(
                    CopyDataSource::Data(file.source.as_bytes().to_vec()),
                    CopyTargetOptions::new(file.destination.as_os_str().to_string_lossy())
                        .with_mode(0o644),
                )
            }));
        }

        if let Some(k3s_extra_args) = config.options.k3s.extra_args {
            k3s.cmd
                .extra_args
                .extend(k3s_extra_args.into_iter().map(|arg| arg.arg));
        }

        k3s.k3d_image = config.image;

        Ok(k3s)
    }

    /// Passes the [`Registries`] configuration into the k3s instance.
    pub fn with_registries(mut self, registries: Registries) -> Self {
        let yaml = noyalib::to_string(&registries).expect("Serializing into YAML worked");

        self.copy_to_sources.push(CopyToContainer::new(
            CopyDataSource::Data(yaml.as_bytes().to_vec()),
            CopyTargetOptions::new("/etc/rancher/k3s/registries.yaml").with_mode(0o400),
        ));

        self
    }

    /// TODO:
    pub fn with_k3d_fixes(mut self) -> Self {
        self.has_k3d_fixes = true;
        self.copy_to_sources.push(CopyToContainer::new(
            // TODO: probably not working with submodule…
            CopyDataSource::Data(
                include_str!("./k3d/pkg/types/fixes/assets/k3d-entrypoint.sh").into(),
            ),
            CopyTargetOptions::new("/bin/k3d-entrypoint.sh").with_mode(0o755),
        ));
        self.copy_to_sources.push(CopyToContainer::new(
            CopyDataSource::Data(
                include_str!("./k3d/pkg/types/fixes/assets/k3d-entrypoint-cgroupv2.sh").into(),
            ),
            CopyTargetOptions::new("/bin/k3d-entrypoint-cgroupv2.sh").with_mode(0o755),
        ));
        self.copy_to_sources.push(CopyToContainer::new(
            CopyDataSource::Data(
                include_str!("./k3d/pkg/types/fixes/assets/k3d-entrypoint-dns.sh").into(),
            ),
            CopyTargetOptions::new("/bin/k3d-entrypoint-dns.sh").with_mode(0o755),
        ));
        self.copy_to_sources.push(CopyToContainer::new(
            CopyDataSource::Data(
                include_str!("./k3d/pkg/types/fixes/assets/k3d-entrypoint-mounts.sh").into(),
            ),
            CopyTargetOptions::new("/bin/k3d-entrypoint-mounts.sh").with_mode(0o755),
        ));
        self
    }

    /// Mounts a host directory to the K3s configuration directory.
    ///
    /// This allows you to access the K3s configuration files (like kubeconfig)
    /// from the host filesystem. The kubeconfig file will be created at
    /// `{conf_mount_path}/k3s.yaml` and can be read using [`read_kube_config`](Self::read_kube_config).
    ///
    /// # Example
    /// ```
    /// use std::path::Path;
    ///
    /// use testcontainers_modules::k3s::K3s;
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
    /// use std::path::Path;
    ///
    /// use testcontainers_modules::k3s::K3s;
    ///
    /// let k3s = K3s::default().with_conf_mount(Path::new("/tmp/k3s-config"));
    /// // After starting the container...
    /// let kubeconfig = k3s.read_kube_config().expect("Failed to read kubeconfig");
    /// ```
    ///
    /// Use [`Self::download_kube_config`] instead.
    #[deprecated(note = "Use `K3s::download_kube_config` instead")]
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

    /// # Example
    /// ```no_run
    /// use std::path::Path;
    ///
    /// use testcontainers_modules::k3s::{K3s, download_kube_config};
    ///
    /// let k3s = K3s::default();
    /// let k3s_container = k3s.start().await?;
    ///
    /// let kubeconfig = K3s::download_kube_config(&k3s_container).await.expect("Failed to download kubeconfig");
    /// ```
    pub async fn download_kube_config(
        k3s: &ContainerAsync<Self>,
    ) -> Result<String, testcontainers::bollard::errors::Error> {
        use tar::Archive;

        let client = testcontainers::core::client::docker_client_instance()
            .await
            .expect("A running client");
        let id = k3s.id();

        let mut download_stream = client.download_from_container(
            id,
            Some(
                DownloadFromContainerOptionsBuilder::new()
                    .path("/etc/rancher/k3s/k3s.yaml")
                    .build(),
            ),
        );

        let mut data = Vec::new();
        while let Some(chunk) = download_stream.next().await {
            let output = chunk?;
            data.extend(output.iter());
        }

        let mut archive = Archive::new(data.as_slice());

        let mut config = String::new();
        let mut entry = archive
            .entries()
            .unwrap()
            .next()
            .expect("an entry with the k3s.yaml")
            .expect("");

        entry.read_to_string(&mut config).unwrap();

        Ok(config)
    }
}

impl IntoIterator for &K3sCmd {
    type Item = String;
    type IntoIter = <Vec<String> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        let mut cmd = vec![String::from("server")];
        cmd.push(format!("--snapshotter={}", self.snapshotter));
        cmd.extend(self.extra_args.iter().cloned());
        cmd.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use std::env::temp_dir;

    use k8s_openapi::api::core::v1::Pod;
    use kube::{
        api::{Api, DeleteParams, ListParams, Patch, PatchParams, PostParams, ResourceExt},
        config::{KubeConfigOptions, Kubeconfig},
        runtime::wait::{await_condition, conditions::is_pod_running},
        Config,
    };
    use rustls::crypto::CryptoProvider;
    use serde_json::json;
    use serial_test::serial;
    use testcontainers::{runners::AsyncRunner, ContainerAsync, ImageExt};

    use super::*;

    #[serial]
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

    // Based on: https://github.com/kube-rs/kube/blob/main/examples/pod_api.rs
    #[serial]
    #[tokio::test]
    async fn pod_api() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let conf_dir = temp_dir();
        let k3s = K3s::default()
            .with_conf_mount(&conf_dir)
            .with_privileged(true)
            .with_userns_mode("host");

        let k3s_container = k3s.start().await?;

        let client = get_kube_client(&k3s_container).await?;

        // Manage pods
        let pods: Api<Pod> = Api::default_namespaced(client);

        // Create Pod blog
        let p: Pod = serde_json::from_value(json!({
            "apiVersion": "v1",
            "kind": "Pod",
            "metadata": { "name": "busybox" },
            "spec": {
                "containers": [{
                  "name": "busybox",
                  "image": "busybox:1.36.1-musl"
                }],
            }
        }))?;

        let post_params = PostParams::default();
        match pods.create(&post_params, &p).await {
            Ok(o) => {
                let name = o.name_any();
                assert_eq!(p.name_any(), name);
            }
            Err(kube::Error::Api(ae)) => assert_eq!(ae.code, 409), // if you skipped delete, for instance
            Err(e) => return Err(e.into()),                        // any other case is probably bad
        }

        // Watch it phase for a few seconds
        let establish = await_condition(pods.clone(), "busybox", is_pod_running());
        let _ = tokio::time::timeout(std::time::Duration::from_secs(15), establish).await?;

        // Verify we can get it
        let p1cpy = pods.get("busybox").await?;
        if let Some(spec) = &p1cpy.spec {
            assert_eq!(spec.containers[0].name, "busybox");
        }

        // Replace its spec
        let patch = json!({
            "metadata": {
                "resourceVersion": p1cpy.resource_version(),
            },
            "spec": {
                "activeDeadlineSeconds": 5
            }
        });

        let patch_params = PatchParams::default();
        let p_patched = pods
            .patch("busybox", &patch_params, &Patch::Merge(&patch))
            .await?;
        assert_eq!(p_patched.spec.unwrap().active_deadline_seconds, Some(5));

        let lp = ListParams::default().fields(&format!("metadata.name={}", "busybox")); // only want results for our pod
        for p in pods.list(&lp).await? {
            println!("Found Pod: {}", p.name_any());
        }

        // Delete it
        let delete_params = DeleteParams::default();
        pods.delete("busybox", &delete_params)
            .await?
            .map_left(|pdel| {
                assert_eq!(pdel.name_any(), "busybox");
            });

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
