use testcontainers::{core::WaitFor, Image};

const NAME: &str = "redis";
const TAG: &str = "5.0";

/// Module to work with [`Redis`] inside of tests.
///
/// Starts an instance of Redis.
///
/// This module is based on the official [`Redis docker image`].
///
/// # Example
/// ```
/// use testcontainers::clients;
/// use testcontainers_modules::redis;
///
/// let docker = clients::Cli::default();
/// let redis_instance = docker.run(redis::Redis);
///
/// let redis_url = format!("redis://127.0.0.1:{}", redis_instance.get_host_port_ipv4(6379));
///
/// // do something with the started redis instance..
///```
///
/// [`Redis`]: https://redis.io/
/// [`Redis docker image`]: https://hub.docker.com/_/redis
#[derive(Debug, Default)]
pub struct Redis;

impl Image for Redis {
    type Args = ();

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Ready to accept connections")]
    }
}

#[cfg(test)]
mod tests {
    use redis::Commands;
    use testcontainers::clients;

    use crate::redis::Redis;

    #[test]
    fn redis_fetch_an_integer() {
        let _ = pretty_env_logger::try_init();
        let docker = clients::Cli::default();
        let node = docker.run(Redis);
        let host_port = node.get_host_port_ipv4(6379);
        let url = format!("redis://127.0.0.1:{host_port}");

        let client = redis::Client::open(url.as_ref()).unwrap();
        let mut con = client.get_connection().unwrap();

        con.set::<_, _, ()>("my_key", 42).unwrap();
        let result: i64 = con.get("my_key").unwrap();
        assert_eq!(42, result);
    }
}
