use std::{collections::BTreeMap, path::PathBuf};

use serde::{Deserialize, Serialize};

/// Registry configuration for k3s, see [here](https://docs.k3s.io/installation/private-registry)
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Registries {
    /// The mirrors section defines the names and endpoints of registries
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mirrors: Option<BTreeMap<String, Mirror>>,
    /// TODO: docs
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configs: Option<BTreeMap<String, Config>>,
}

/// A mirror defines the endpoints of registries
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Mirror {
    /// List of endpoints of the registry.
    pub endpoint: Vec<String>,
    /// Regex rewrites applied to image names pulled through this mirror, see
    /// https://docs.k3s.io/installation/private-registry#rewrites for more information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rewrite: Option<BTreeMap<String, String>>,
}

/// TODO: docs
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Config {
    /// TODO: docs
    pub auth: Option<AuthConfig>,
    /// TODO: docs
    pub tls: Option<TlsConfig>,
}

/// TODO: docs
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct AuthConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<String>,
}

/// TODO: docs
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct TlsConfig {
    /// TODO: docs
    pub ca_file: Option<PathBuf>,
    /// TODO: docs
    pub cert_file: Option<PathBuf>,
    /// TODO: docs
    pub key_file: Option<PathBuf>,
    pub insecure_skip_verify: Option<bool>,
}
