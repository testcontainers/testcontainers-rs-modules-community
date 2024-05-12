use testcontainers::{core::WaitFor, Image};

const NAME: &str = "registry.k8s.io/kwok/cluster";
const TAG: &str = "v0.5.2-k8s.v1.29.2";
const DEFAULT_WAIT: u64 = 3000;

/// This module provides [Kwok Cluster](https://kwok.sigs.k8s.io/) (Kubernetes WithOut Kubelet).
///
/// Currently pinned to [version `v0.5.2-k8s.v1.29.2`](https://github.com/kubernetes-sigs/kwok/releases/tag/v0.5.2)
///
/// # Configuration
///
/// For configuration, Kwok Cluster uses environment variables. You can go [here](https://kwok.sigs.k8s.io/docs/user/configuration/#a-note-on-cli-flags-environment-variables-and-configuration-files)
/// for the full list.
///
/// Testcontainers support setting environment variables with the method
/// `RunnableImage::with_env_var((impl Into<String>, impl Into<String>))`. You will have to convert
/// the Image into a RunnableImage first.
///
/// ```
/// use testcontainers_modules::kwok::KwokCluster;
/// use testcontainers::RunnableImage;
///
/// let image: RunnableImage<KwokCluster> = KwokCluster::default().into();
/// let image = image.with_env_var(("KWOK_PROMETHEUS_PORT", "9090"));
/// ```
///
/// No environment variables are required.
#[derive(Debug, Default)]
pub struct KwokCluster;

impl Image for KwokCluster {
    type Args = ();

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![
            WaitFor::message_on_stdout("Starting to serve on [::]:8080"),
            WaitFor::millis(DEFAULT_WAIT),
        ]
    }

    fn expose_ports(&self) -> Vec<u16> {
        vec![8080]
    }
}

#[cfg(test)]
mod test {
    use k8s_openapi::api::core::v1::Namespace;
    use kube::{
        api::ListParams,
        client::Client,
        config::{AuthInfo, Cluster, KubeConfigOptions, Kubeconfig, NamedAuthInfo, NamedCluster},
        Api, Config,
    };
    use rustls::crypto::CryptoProvider;

    use crate::{kwok::KwokCluster, testcontainers::runners::AsyncRunner};

    const CLUSTER_NAME: &str = "kwok-kwok";
    const CONTEXT_NAME: &str = "kwok-kwok";
    const CLUSTER_USER: &str = "kwok-kwok";

    #[tokio::test]
    async fn test_kwok_image() {
        if CryptoProvider::get_default().is_none() {
            rustls::crypto::ring::default_provider()
                .install_default()
                .expect("Error initializing rustls provider");
        }

        let node = KwokCluster.start().await;
        let host_port = node.get_host_port_ipv4(8080).await;

        // Create a custom Kubeconfig
        let kubeconfig = Kubeconfig {
            clusters: vec![NamedCluster {
                name: String::from(CLUSTER_NAME),
                cluster: Some(Cluster {
                    server: Some(String::from(format!("http://localhost:{host_port}"))), // your custom endpoint
                    ..Default::default()
                }),
            }],
            contexts: vec![kube::config::NamedContext {
                name: CONTEXT_NAME.to_string(),
                context: Option::from(kube::config::Context {
                    cluster: CLUSTER_NAME.to_string(),
                    user: String::from(CLUSTER_USER),
                    ..Default::default()
                }),
            }],
            auth_infos: vec![NamedAuthInfo {
                name: String::from(CLUSTER_USER),
                auth_info: Some(AuthInfo {
                    token: None,
                    ..Default::default()
                }),
            }],
            current_context: Some(CONTEXT_NAME.to_string()),
            ..Default::default()
        };
        let kubeconfigoptions = KubeConfigOptions {
            context: Some(CONTEXT_NAME.to_string()),
            cluster: Some(CLUSTER_NAME.to_string()),
            user: None,
        };

        // Convert the Kubeconfig into a Config
        let config = Config::from_custom_kubeconfig(kubeconfig, &kubeconfigoptions)
            .await
            .unwrap();

        // Create a Client from Config
        let client = Client::try_from(config).unwrap();

        let api: Api<Namespace> = Api::all(client);
        let namespaces = api.list(&ListParams::default()).await.unwrap();
        assert_eq!(namespaces.items.len(), 4);
        let namespace_names: Vec<&str> = namespaces
            .items
            .iter()
            .map(|namespace| namespace.metadata.name.as_deref().unwrap())
            .collect();
        assert_eq!(
            namespace_names,
            vec!["default", "kube-node-lease", "kube-public", "kube-system"]
        );
    }
}
