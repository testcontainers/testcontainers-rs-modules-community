//! Community maintained modules for [`testcontainers`].
//!
//! Provides modules to use for testing components in accordance with testcontainers-rs.
//! Every module is treated as a feature inside this module.
//!
//! # Usage
//! Depend on [`testcontainers-modules`] and choose the modules to work with by declaring the features.
//! Then start using the modules inside your tests. Please have a look at the documentation of the separate modules
//! for examples on how to use the module.
//!
//! [`testcontainers`]: https://crates.io/crates/testcontainers
//! [`testcontainers-modules`]: https://crates.io/crates/testcontainers-modules

#[cfg(feature = "dynamodb")]
pub mod dynamodb_local;
#[cfg(feature = "elastic_search")]
pub mod elastic_search;
#[cfg(feature = "elasticmq")]
pub mod elasticmq;
#[cfg(feature = "google_cloud_sdk_emulators")]
pub mod google_cloud_sdk_emulators;
#[cfg(feature = "kafka")]
pub mod kafka;
#[cfg(feature = "minio")]
pub mod minio;
#[cfg(feature = "mongo")]
pub mod mongo;
#[cfg(feature = "orientdb")]
pub mod orientdb;
#[cfg(feature = "parity")]
pub mod parity_parity;
#[cfg(feature = "postgres")]
pub mod postgres;
#[cfg(feature = "rabbitmq")]
pub mod rabbitmq;
#[cfg(feature = "redis")]
pub mod redis;
#[cfg(feature = "trufflesuite_ganachecli")]
pub mod trufflesuite_ganachecli;
#[cfg(feature = "zookeeper")]
pub mod zookeeper;

/// Re-exported version of `testcontainers` to avoid version conflicts
pub use testcontainers;
