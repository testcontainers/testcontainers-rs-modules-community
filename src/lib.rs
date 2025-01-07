// let's document our code for other/future developers
#![deny(missing_docs)]
#![cfg_attr(docsrs, deny(rustdoc::broken_intra_doc_links))]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/testcontainers/testcontainers-rs-modules-community/main/logo.svg"
)]
#![doc = include_str!("../README.md")]
//! Please have a look at the documentation of the separate modules for examples on how to use the module.

#[cfg(feature = "clickhouse")]
#[cfg_attr(docsrs, doc(cfg(feature = "clickhouse")))]
/// **Clickhouse** (analytics database) testcontainer
pub mod clickhouse;
#[cfg(feature = "cncf_distribution")]
#[cfg_attr(docsrs, doc(cfg(feature = "cncf_distribution")))]
/// **CNCF Distribution** (container registry) testcontainer
pub mod cncf_distribution;
#[cfg(feature = "cockroach_db")]
#[cfg_attr(docsrs, doc(cfg(feature = "cockroach_db")))]
/// **CockroachDB** (distributed database) testcontainer
pub mod cockroach_db;
#[cfg(feature = "consul")]
#[cfg_attr(docsrs, doc(cfg(feature = "consul")))]
/// **Consul** (identity-based networking) testcontainer
pub mod consul;
#[cfg(feature = "databend")]
#[cfg_attr(docsrs, doc(cfg(feature = "databend")))]
/// **Databend** (analytics database) testcontainer
pub mod databend;
#[cfg(feature = "dynamodb")]
#[cfg_attr(docsrs, doc(cfg(feature = "dynamodb")))]
/// **DynamoDB** (NoSQL database) testcontainer
pub mod dynamodb_local;
#[cfg(feature = "elastic_search")]
#[cfg_attr(docsrs, doc(cfg(feature = "elastic_search")))]
/// **Elasticsearch** (distributed search engine) testcontainer
pub mod elastic_search;
#[cfg(feature = "elasticmq")]
#[cfg_attr(docsrs, doc(cfg(feature = "elasticmq")))]
/// **ElasticMQ** (message queue) testcontainer
pub mod elasticmq;
#[cfg(feature = "gitea")]
#[cfg_attr(docsrs, doc(cfg(feature = "gitea")))]
/// **Gitea** (self-hosted Git service) testcontainer
pub mod gitea;
#[cfg(feature = "google_cloud_sdk_emulators")]
#[cfg_attr(docsrs, doc(cfg(feature = "google_cloud_sdk_emulators")))]
/// **googles cloud sdk emulator** testcontainer
pub mod google_cloud_sdk_emulators;
#[cfg(feature = "hashicorp_vault")]
#[cfg_attr(docsrs, doc(cfg(feature = "hashicorp_vault")))]
/// ‎**HashiCorp Vault** (secrets management) testcontainer
pub mod hashicorp_vault;
#[cfg(feature = "k3s")]
#[cfg_attr(docsrs, doc(cfg(feature = "k3s")))]
/// **K3s** (lightweight kubernetes) testcontainer
pub mod k3s;
#[cfg(feature = "kafka")]
#[cfg_attr(docsrs, doc(cfg(feature = "kafka")))]
/// **Apache Kafka** (data streaming) testcontainer
pub mod kafka;
#[cfg(feature = "kwok")]
#[cfg_attr(docsrs, doc(cfg(feature = "kwok")))]
/// **KWOK Cluster** (Kubernetes WithOut Kubelet) testcontainer
pub mod kwok;
#[cfg(feature = "localstack")]
#[cfg_attr(docsrs, doc(cfg(feature = "localstack")))]
/// **LocalStack** (local AWS emulation) testcontainer
pub mod localstack;
#[cfg(feature = "mariadb")]
#[cfg_attr(docsrs, doc(cfg(feature = "mariadb")))]
/// **MariaDB** (relational database) testcontainer
pub mod mariadb;
#[cfg(feature = "meilisearch")]
#[cfg_attr(docsrs, doc(cfg(feature = "meilisearch")))]
/// **Meilisearch** (search engine) testcontainer
pub mod meilisearch;
#[cfg(feature = "minio")]
#[cfg_attr(docsrs, doc(cfg(feature = "minio")))]
/// **minio** (object storage) testcontainer
pub mod minio;
#[cfg(feature = "mongo")]
#[cfg_attr(docsrs, doc(cfg(feature = "mongo")))]
/// **MongoDB** (NoSql database) testcontainer
pub mod mongo;
#[cfg(feature = "mosquitto")]
#[cfg_attr(docsrs, doc(cfg(feature = "mosquitto")))]
/// **mosquitto** (mqtt message broker) testcontainer
pub mod mosquitto;
#[cfg(feature = "mssql_server")]
#[cfg_attr(docsrs, doc(cfg(feature = "mssql_server")))]
/// **Microsoft SQL Server** (relational database) testcontainer
pub mod mssql_server;
#[cfg(feature = "mysql")]
#[cfg_attr(docsrs, doc(cfg(feature = "mysql")))]
/// **MySQL** (relational database) testcontainer
pub mod mysql;
#[cfg(feature = "nats")]
#[cfg_attr(docsrs, doc(cfg(feature = "nats")))]
/// **Nats** (message oriented middleware) testcontainer
pub mod nats;
#[cfg(feature = "neo4j")]
#[cfg_attr(docsrs, doc(cfg(feature = "neo4j")))]
/// **Neo4j** (graph database) testcontainer
pub mod neo4j;
#[cfg(feature = "openldap")]
#[cfg_attr(docsrs, doc(cfg(feature = "openldap")))]
/// **Openldap** (ldap authentification) testcontainer
pub mod openldap;
#[cfg(feature = "oracle")]
#[cfg_attr(docsrs, doc(cfg(feature = "oracle")))]
/// **oracle** (relational database) testcontainer
pub mod oracle;
#[cfg(feature = "orientdb")]
#[cfg_attr(docsrs, doc(cfg(feature = "orientdb")))]
/// **orientdb** (nosql database) testcontainer
pub mod orientdb;
#[cfg(feature = "parity")]
#[cfg_attr(docsrs, doc(cfg(feature = "parity")))]
/// **parity_parity** (etherium client) testcontainer
pub mod parity_parity;
#[cfg(feature = "postgres")]
#[cfg_attr(docsrs, doc(cfg(feature = "postgres")))]
/// **Postgres** (relational database) testcontainer
pub mod postgres;
#[cfg(feature = "pulsar")]
#[cfg_attr(docsrs, doc(cfg(feature = "pulsar")))]
/// **Apache Pulsar** (Cloud-Native, Distributed Messaging and Streaming) testcontainer
pub mod pulsar;
#[cfg(feature = "rabbitmq")]
#[cfg_attr(docsrs, doc(cfg(feature = "rabbitmq")))]
/// **rabbitmq** (message broker) testcontainer
pub mod rabbitmq;
#[cfg(feature = "redis")]
#[cfg_attr(docsrs, doc(cfg(feature = "redis")))]
/// **redis** (in memory nosql database) testcontainer
pub mod redis;
#[cfg(feature = "rqlite")]
#[cfg_attr(docsrs, doc(cfg(feature = "rqlite")))]
/// **RQLite** (lightweight, user-friendly, distributed relational database) testcontainer
pub mod rqlite;
#[cfg(feature = "solr")]
#[cfg_attr(docsrs, doc(cfg(feature = "solr")))]
/// **Apache Solr** (distributed search engine) testcontainer
pub mod solr;
#[cfg(feature = "surrealdb")]
#[cfg_attr(docsrs, doc(cfg(feature = "surrealdb")))]
/// **surrealdb** (mutli model database) testcontainer
pub mod surrealdb;
#[cfg(feature = "trufflesuite_ganachecli")]
#[cfg_attr(docsrs, doc(cfg(feature = "trufflesuite_ganachecli")))]
/// **Trufflesuite Ganache CLI** (etherium simulator) testcontainer
pub mod trufflesuite_ganachecli;
#[cfg(feature = "valkey")]
#[cfg_attr(docsrs, doc(cfg(feature = "valkey")))]
/// **Valkey** (in memory nosql database) testcontainer
pub mod valkey;
#[cfg(feature = "victoria_metrics")]
#[cfg_attr(docsrs, doc(cfg(feature = "victoria_metrics")))]
/// **VictoriaMetrics** (monitoring and time series metrics database) testcontainer
pub mod victoria_metrics;
#[cfg(feature = "zookeeper")]
#[cfg_attr(docsrs, doc(cfg(feature = "zookeeper")))]
/// **Apache ZooKeeper** (locking and configuratin management) testcontainer
pub mod zookeeper;

/// Re-exported version of `testcontainers` to avoid version conflicts
pub use testcontainers;
