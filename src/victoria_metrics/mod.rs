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
/// use testcontainers::clients;
/// use testcontainers_modules::victoria_metrics;
///
/// let docker = clients::Cli::default();
/// let victoria_metrics_instance = docker.run(victoria_metrics::VictoriaMetrics);
///
/// let import_url = format!("http://127.0.0.1:{}/api/v1/import", victoria_metrics_instance.get_host_port_ipv4(8428));
/// let export_url = format!("http://127.0.0.1:{}/api/v1/export", victoria_metrics_instance.get_host_port_ipv4(8428));
///
/// // operate on the import and export URLs..
/// ```
///
/// [`VictoriaMetrics`]: https://docs.victoriametrics.com/
/// [`VictoriaMetrics API examples`]: https://docs.victoriametrics.com/url-examples.html#victoriametrics-api-examples
/// [`VictoriaMetrics Docker image`]: https://hub.docker.com/r/victoriametrics/victoria-metrics
#[derive(Debug, Default)]
pub struct VictoriaMetrics;

impl Image for VictoriaMetrics {
    type Args = ();

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
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
    use crate::victoria_metrics::VictoriaMetrics as VictoriaMetricsImage;
    use testcontainers::clients;

    #[test]
    fn query_buildinfo() {
        let docker = clients::Cli::default();
        let node = docker.run(VictoriaMetricsImage);
        let host_port = node.get_host_port_ipv4(8428);
        let url = format!("http://127.0.0.1:{}/api/v1/status/buildinfo", host_port);

        let response = reqwest::blocking::get(url)
            .unwrap()
            .json::<serde_json::Value>()
            .unwrap();
        let version = response["data"]["version"].as_str().unwrap();

        assert_eq!(version, "2.24.0");
    }
}
