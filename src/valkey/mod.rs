use std::{borrow::Cow, collections::BTreeMap};

use testcontainers::{
    core::{ContainerPort, WaitFor},
    CopyDataSource, CopyToContainer, Image,
};

const NAME: &str = "valkey/valkey";
const TAG: &str = "8.0.2-alpine";

/// Default port (6379) on which Valkey is exposed
pub const VALKEY_PORT: ContainerPort = ContainerPort::Tcp(6379);

/// Module to work with [`Valkey`] inside of tests.
/// Valkey is a high-performance data structure server that primarily serves key/value workloads.
///
/// Starts an instance of Valkey based on the official [`Valkey docker image`].
///
/// By default, Valkey is exposed on Port 6379 ([`VALKEY_PORT`]), just like Redis, and has no access control.
/// Currently, for communication with Valkey we can still use redis library.
///
/// # Example
/// ```
/// use redis::Commands;
/// use testcontainers_modules::{
///     testcontainers::runners::SyncRunner,
///     valkey::{Valkey, VALKEY_PORT},
/// };
///
/// let valkey_instance = Valkey::default().start().unwrap();
/// let host_ip = valkey_instance.get_host().unwrap();
/// let host_port = valkey_instance.get_host_port_ipv4(VALKEY_PORT).unwrap();
///
/// let url = format!("redis://{host_ip}:{host_port}");
/// let client = redis::Client::open(url.as_ref()).unwrap();
/// let mut con = client.get_connection().unwrap();
///
/// con.set::<_, _, ()>("my_key", 42).unwrap();
/// let result: i64 = con.get("my_key").unwrap();
/// ```
///
/// [`Valkey`]: https://valkey.io/
/// [`Valeky docker image`]: https://hub.docker.com/r/valkey/valkey
/// [`VALKEY_PORT`]: super::VALKEY_PORT
#[derive(Debug, Default, Clone)]
pub struct Valkey {
    env_vars: BTreeMap<String, String>,
    tag: Option<String>,
    copy_to_container: Vec<CopyToContainer>,
}

impl Valkey {
    /// Create a new Valkey instance with the latest image.
    ///
    /// # Example
    /// ```
    /// use testcontainers_modules::{
    ///     testcontainers::runners::SyncRunner,
    ///     valkey::{Valkey, VALKEY_PORT},
    /// };
    ///
    /// let valkey_instance = Valkey::latest().start().unwrap();
    /// ```
    pub fn latest() -> Self {
        Self {
            tag: Some("latest".to_string()),
            ..Default::default()
        }
    }

    /// Add extra flags by passing additional start arguments.
    ///
    /// # Example
    /// ```
    /// use testcontainers_modules::{
    ///     testcontainers::runners::SyncRunner,
    ///     valkey::{Valkey, VALKEY_PORT},
    /// };
    ///
    /// let valkey_instance = Valkey::default()
    ///     .with_valkey_extra_flags("--maxmemory 2mb")
    ///     .start()
    ///     .unwrap();
    /// ```
    pub fn with_valkey_extra_flags(self, valkey_extra_flags: &str) -> Self {
        let mut env_vars = self.env_vars;
        env_vars.insert(
            "VALKEY_EXTRA_FLAGS".to_string(),
            valkey_extra_flags.to_string(),
        );
        Self {
            env_vars,
            tag: self.tag,
            copy_to_container: self.copy_to_container,
        }
    }

    /// Add custom valkey configuration.
    ///
    /// # Example
    /// ```
    /// use testcontainers_modules::{
    ///     testcontainers::runners::SyncRunner,
    ///     valkey::{Valkey, VALKEY_PORT},
    /// };
    ///
    /// let valkey_instance = Valkey::default()
    ///     .with_valkey_conf("maxmemory 2mb".to_string().into_bytes())
    ///     .start()
    ///     .unwrap();
    /// ```
    pub fn with_valkey_conf(self, valky_conf: impl Into<CopyDataSource>) -> Self {
        let mut copy_to_container = self.copy_to_container;
        copy_to_container.push(CopyToContainer::new(
            valky_conf.into(),
            "/usr/local/etc/valkey/valkey.conf",
        ));
        Self {
            env_vars: self.env_vars,
            tag: self.tag,
            copy_to_container,
        }
    }
}

impl Image for Valkey {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        self.tag.as_deref().unwrap_or(TAG)
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Ready to accept connections")]
    }

    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        &self.env_vars
    }

    fn copy_to_sources(&self) -> impl IntoIterator<Item = &CopyToContainer> {
        &self.copy_to_container
    }

    fn cmd(&self) -> impl IntoIterator<Item = impl Into<Cow<'_, str>>> {
        if !self.copy_to_container.is_empty() {
            vec!["valkey-server", "/usr/local/etc/valkey/valkey.conf"]
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use redis::Commands;
    use testcontainers::Image;

    use crate::{
        testcontainers::runners::SyncRunner,
        valkey::{Valkey, TAG, VALKEY_PORT},
    };

    #[test]
    fn valkey_fetch_an_integer() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let _ = pretty_env_logger::try_init();
        let node = Valkey::default().start()?;

        let tag = node.image().tag.clone();
        assert_eq!(None, tag);
        let tag_from_method = node.image().tag();
        assert_eq!(TAG, tag_from_method);
        assert_eq!(0, node.image().copy_to_container.len());

        let host_ip = node.get_host()?;
        let host_port = node.get_host_port_ipv4(VALKEY_PORT)?;
        let url = format!("redis://{host_ip}:{host_port}");
        let client = redis::Client::open(url.as_ref()).unwrap();
        let mut con = client.get_connection().unwrap();

        con.set::<_, _, ()>("my_key", 42).unwrap();
        let result: i64 = con.get("my_key").unwrap();
        assert_eq!(42, result);
        Ok(())
    }

    #[test]
    fn valkey_latest() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let _ = pretty_env_logger::try_init();
        let node = Valkey::latest().start()?;

        let tag = node.image().tag.clone();
        assert_eq!(Some("latest".to_string()), tag);
        let tag_from_method = node.image().tag();
        assert_eq!("latest", tag_from_method);
        assert_eq!(0, node.image().copy_to_container.len());

        let host_ip = node.get_host()?;
        let host_port = node.get_host_port_ipv4(VALKEY_PORT)?;
        let url = format!("redis://{host_ip}:{host_port}");
        let client = redis::Client::open(url.as_ref()).unwrap();
        let mut con = client.get_connection().unwrap();

        con.set::<_, _, ()>("my_key", 42).unwrap();
        let result: i64 = con.get("my_key").unwrap();
        assert_eq!(42, result);
        Ok(())
    }

    #[test]
    fn valkey_extra_flags() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let _ = pretty_env_logger::try_init();
        let node = Valkey::default()
            .with_valkey_extra_flags("--maxmemory 2mb")
            .start()?;
        let tag = node.image().tag.clone();
        assert_eq!(None, tag);
        let tag_from_method = node.image().tag();
        assert_eq!(TAG, tag_from_method);
        assert_eq!(0, node.image().copy_to_container.len());

        let host_ip = node.get_host()?;
        let host_port = node.get_host_port_ipv4(VALKEY_PORT)?;
        let url = format!("redis://{host_ip}:{host_port}");

        let client = redis::Client::open(url.as_ref()).unwrap();
        let mut con = client.get_connection().unwrap();
        let max_memory: HashMap<String, isize> = redis::cmd("CONFIG")
            .arg("GET")
            .arg("maxmemory")
            .query(&mut con)
            .unwrap();
        let max = *max_memory.get("maxmemory").unwrap();
        assert_eq!(2097152, max);
        Ok(())
    }

    #[test]
    fn valkey_conf() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let _ = pretty_env_logger::try_init();
        let node = Valkey::default()
            .with_valkey_conf("maxmemory 2mb".to_string().into_bytes())
            .start()?;
        let tag = node.image().tag.clone();
        assert_eq!(None, tag);
        let tag_from_method = node.image().tag();
        assert_eq!(TAG, tag_from_method);
        assert_eq!(1, node.image().copy_to_container.len());

        let host_ip = node.get_host()?;
        let host_port = node.get_host_port_ipv4(VALKEY_PORT)?;
        let url = format!("redis://{host_ip}:{host_port}");

        let client = redis::Client::open(url.as_ref()).unwrap();
        let mut con = client.get_connection().unwrap();
        let max_memory: HashMap<String, isize> = redis::cmd("CONFIG")
            .arg("GET")
            .arg("maxmemory")
            .query(&mut con)
            .unwrap();
        let max = *max_memory.get("maxmemory").unwrap();
        assert_eq!(2097152, max);
        Ok(())
    }
}
