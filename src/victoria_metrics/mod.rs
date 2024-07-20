use testcontainers::{core::WaitFor, Image};

const NAME: &str = "victoriametrics/victoria-metrics";
const TAG: &str = "v1.96.0";

/// Module to work with [`VictoriaMetrics`] inside of tests.
///
/// Starts an instance of single-node VictoriaMetrics.
///
/// This module is based on the official [`VictoriaMetrics docker image`].
///
/// # Example
/// ```
/// use testcontainers_modules::{victoria_metrics, testcontainers::runners::SyncRunner};
///
/// let victoria_metrics_instance = victoria_metrics::VictoriaMetrics::default().start().unwrap();
///
/// let import_url = format!("http://127.0.0.1:{}/api/v1/import", victoria_metrics_instance.get_host_port_ipv4(8428).unwrap());
/// let export_url = format!("http://127.0.0.1:{}/api/v1/export", victoria_metrics_instance.get_host_port_ipv4(8428).unwrap());
///
/// // operate on the import and export URLs..
/// ```
///
/// [`VictoriaMetrics`]: https://docs.victoriametrics.com/
/// [`VictoriaMetrics API examples`]: https://docs.victoriametrics.com/url-examples.html#victoriametrics-api-examples
/// [`VictoriaMetrics Docker image`]: https://hub.docker.com/r/victoriametrics/victoria-metrics
#[derive(Debug, Default, Clone)]
pub struct VictoriaMetrics {
    /// (remove if there is another variable)
    /// Field is included to prevent this struct to be a unit struct.
    /// This allows extending functionality (and thus further variables) without breaking changes
    _priv: (),
}

impl Image for VictoriaMetrics {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![
            WaitFor::message_on_stderr("started VictoriaMetrics"),
            WaitFor::message_on_stderr(
                "pprof handlers are exposed at http://127.0.0.1:8428/debug/pprof/",
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        testcontainers::runners::SyncRunner,
        victoria_metrics::VictoriaMetrics as VictoriaMetricsImage,
    };

    #[test]
    fn query_buildinfo() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let node = VictoriaMetricsImage::default().start()?;
        let host_ip = node.get_host()?;
        let host_port = node.get_host_port_ipv4(8428)?;
        let url = format!("http://{host_ip}:{host_port}/api/v1/status/buildinfo");

        let response = reqwest::blocking::get(url)
            .unwrap()
            .json::<serde_json::Value>()
            .unwrap();
        let version = response["data"]["version"].as_str().unwrap();

        assert_eq!(version, "2.24.0");
        Ok(())
    }
}
