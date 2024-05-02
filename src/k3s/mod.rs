use testcontainers::core::WaitFor;
use testcontainers::{Image, ImageArgs};

const NAME: &str = "rancher/k3s";
const TAG: &str = "v1.28.8-k3s1";
pub const TRAEFIK_HTTP: u16 = 80;
pub const KUBE_SECURE_PORT: u16 = 6443;
pub const RANCHER_WEBHOOK_PORT: u16 = 8443;

#[derive(Debug, Default, Clone)]
pub struct K3s;

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

    fn expose_ports(&self) -> Vec<u16> {
        vec![KUBE_SECURE_PORT, RANCHER_WEBHOOK_PORT, TRAEFIK_HTTP]
    }
}
