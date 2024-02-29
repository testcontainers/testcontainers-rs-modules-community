use testcontainers::{core::WaitFor, Image, ImageArgs};

/// Module to work with [`Mosquitto`] inside of tests.
///
/// Starts a MQTT broker without authentication.
///
///
/// # Example
/// ```
/// use testcontainers::clients;
/// use testcontainers_modules::mosquitto;
///
/// let docker = clients::Cli::default();
/// let mosquitto_instance = docker.run(mosquitto::Mosquitto);
///
/// let broker_url = format!("127.0.0.1:{}", mosquitto_instance.get_host_port_ipv4(1883));
/// ```
///
/// [`Mosquitto`]: https://mosquitto.org/
/// [`Mosquitto docker image`]: https://hub.docker.com/_/eclipse-mosquitto

const NAME: &str = "eclipse-mosquitto";
const TAG: &str = "2.0.18";

#[derive(Debug, Default, Clone)]
pub struct Mosquitto;
#[derive(Debug, Default, Clone)]
pub struct MosquittoArgs;

impl ImageArgs for MosquittoArgs {
    fn into_iterator(self) -> Box<dyn Iterator<Item = String>> {
        Box::new(
            vec![
                "mosquitto".to_string(),
                "-c".to_string(),
                "/mosquitto-no-auth.conf".to_string(),
            ]
            .into_iter(),
        )
    }
}
impl Image for Mosquitto {
    type Args = MosquittoArgs;

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stderr(format!(
            "mosquitto version {} running",
            TAG
        ))]
    }
}
