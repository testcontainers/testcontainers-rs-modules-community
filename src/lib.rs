#![cfg_attr(docsrs, deny(rustdoc::broken_intra_doc_links))]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/testcontainers/testcontainers-rs-modules-community/main/logo.svg"
)]
#![doc = include_str!("../README.md")]
//! Please have a look at the documentation of the separate modules for examples on how to use the module.

#[cfg(feature = "dynamodb")]
#[cfg_attr(docsrs, doc(cfg(feature = "dynamodb")))]
pub mod dynamodb_local;
#[cfg(feature = "elastic_search")]
#[cfg_attr(docsrs, doc(cfg(feature = "elastic_search")))]
pub mod elastic_search;
#[cfg(feature = "elasticmq")]
#[cfg_attr(docsrs, doc(cfg(feature = "elasticmq")))]
pub mod elasticmq;
#[cfg(feature = "google_cloud_sdk_emulators")]
#[cfg_attr(docsrs, doc(cfg(feature = "google_cloud_sdk_emulators")))]
pub mod google_cloud_sdk_emulators;
#[cfg(feature = "kafka")]
#[cfg_attr(docsrs, doc(cfg(feature = "kafka")))]
pub mod kafka;
#[cfg(feature = "minio")]
#[cfg_attr(docsrs, doc(cfg(feature = "minio")))]
pub mod minio;
#[cfg(feature = "mongo")]
#[cfg_attr(docsrs, doc(cfg(feature = "mongo")))]
pub mod mongo;
#[cfg(feature = "mysql")]
#[cfg_attr(docsrs, doc(cfg(feature = "mysql")))]
pub mod mysql;
#[cfg(feature = "orientdb")]
#[cfg_attr(docsrs, doc(cfg(feature = "orientdb")))]
pub mod orientdb;
#[cfg(feature = "parity")]
#[cfg_attr(docsrs, doc(cfg(feature = "parity")))]
pub mod parity_parity;
#[cfg(feature = "postgres")]
#[cfg_attr(docsrs, doc(cfg(feature = "postgres")))]
pub mod postgres;
#[cfg(feature = "rabbitmq")]
#[cfg_attr(docsrs, doc(cfg(feature = "rabbitmq")))]
pub mod rabbitmq;
#[cfg(feature = "redis")]
#[cfg_attr(docsrs, doc(cfg(feature = "redis")))]
pub mod redis;
#[cfg(feature = "trufflesuite_ganachecli")]
#[cfg_attr(docsrs, doc(cfg(feature = "trufflesuite_ganachecli")))]
pub mod trufflesuite_ganachecli;
#[cfg(feature = "zookeeper")]
#[cfg_attr(docsrs, doc(cfg(feature = "zookeeper")))]
pub mod zookeeper;

/// Re-exported version of `testcontainers` to avoid version conflicts
pub use testcontainers;
