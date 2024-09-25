use std::borrow::Cow;

use testcontainers::{core::WaitFor, Image};

/// Module to work with [`Mosquitto`] inside of tests.
///
/// Starts a MQTT broker without authentication.
///
///
/// # Example
/// ```
/// use testcontainers_modules::{mosquitto, testcontainers::runners::SyncRunner};
///
/// let mosquitto_instance = mosquitto::Mosquitto::default().start().unwrap();
///
/// let broker_url = format!(
///     "{}:{}",
///     mosquitto_instance.get_host().unwrap(),
///     mosquitto_instance.get_host_port_ipv4(1883).unwrap()
/// );
/// ```
///
/// [`Mosquitto`]: https://mosquitto.org/
/// [`Mosquitto docker image`]: https://hub.docker.com/_/eclipse-mosquitto

const NAME: &str = "eclipse-mosquitto";
const TAG: &str = "2.0.18";

#[allow(missing_docs, reason = "not having docs here is currently allowed to adress the missing docs problem one place at a time. If you would like to help us, documenting one of these places helps other devs tremendously")]
#[derive(Debug, Default, Clone)]
pub struct Mosquitto {
    _priv: (),
}

impl Image for Mosquitto {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stderr(format!(
            "mosquitto version {} running",
            TAG
        ))]
    }

    fn cmd(&self) -> impl IntoIterator<Item = impl Into<Cow<'_, str>>> {
        ["mosquitto", "-c", "/mosquitto-no-auth.conf"]
    }
}
