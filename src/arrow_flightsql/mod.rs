use std::borrow::Cow;
use testcontainers::{core::WaitFor, Image};

const NAME: &str = "voltrondata/flight-sql";
const TAG: &str = "v1.4.1-slim";

#[derive(Clone, Debug, Default)]
/// Module to work with [`Arrow FlightSQL`] inside of tests.
///
/// This module is based on the [`voltrondata/flight-sql docker image`](https://hub.docker.com/r/voltrondata/flight-sql).
///
/// # Example
/// ```
/// use arrow_flight::flight_service_client::FlightServiceClient;
/// use arrow_flight::sql::client::FlightSqlServiceClient;
/// use futures::TryStreamExt;
/// use testcontainers::runners::AsyncRunner;
/// use testcontainers_modules::arrow_flightsql::ArrowFlightSQL;
///
/// #[tokio::test]
/// async fn arrow_flightsql_select_version() -> Result<(), Box<dyn std::error::Error + 'static>> {
///     let image = ArrowFlightSQL::default();
///     let instance = image.start().await?;
///     let host = instance.get_host().await?;
///     let port = instance.get_host_port_ipv4(31337).await?;
///     let url = format!("http://{host}:{port}");
///     let service_client = FlightServiceClient::connect(url).await?;
///     let mut client = FlightSqlServiceClient::new_from_inner(service_client);
///     let _ = client.handshake("flight_username", "test").await?;
///
///     let mut statement = client
///         .prepare("SELECT VERSION();".to_string(), None)
///         .await?;
///     let flight_info = statement.execute().await?;
///
///     let ticket = flight_info.endpoint[0]
///         .ticket
///         .as_ref()
///         .expect("Ticket not present")
///         .clone();
///     let flight_data = client.do_get(ticket).await?;
///     let batches: Vec<_> = flight_data.try_collect().await?;
///     let batch = batches.first().expect("batch 0 not present");
///     let array = batch.columns().first().expect("column not present");
///     let data = array.to_data();
///     let buffers = data.buffers();
///     let buffer = buffers.get(1).expect("buffer not present");
///     let values = buffer.to_vec();
///     let version = String::from_utf8(values)?;
///
///     assert_eq!(version, "v1.0.0");
///     Ok(())
/// }
/// ```
///
/// [`Apache Arrow FlightSQL`]: https://arrow.apache.org/docs/format/FlightSql.html
/// [`voltrondata/flight-sql docker image`]: https://hub.docker.com/r/voltrondata/flight-sql
pub struct ArrowFlightSQL {}

impl Image for ArrowFlightSQL {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Flight SQL server - started")]
    }

    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        [
            ("FLIGHT_PASSWORD", "test"),
            ("DATABASE_FILENAME", "test.duckdb"),
        ]
    }
}

#[cfg(test)]
mod tests {
    use crate::arrow_flightsql::ArrowFlightSQL;
    use arrow_flight::flight_service_client::FlightServiceClient;
    use arrow_flight::sql::client::FlightSqlServiceClient;
    use futures::TryStreamExt;
    use testcontainers::runners::AsyncRunner;

    #[tokio::test]
    async fn arrow_flightsql_select_version() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let image = ArrowFlightSQL::default();
        let instance = image.start().await?;
        let host = instance.get_host().await?;
        let port = instance.get_host_port_ipv4(31337).await?;
        let url = format!("http://{host}:{port}");
        let service_client = FlightServiceClient::connect(url).await?;
        let mut client = FlightSqlServiceClient::new_from_inner(service_client);
        let _ = client.handshake("flight_username", "test").await?;

        let mut statement = client
            .prepare("SELECT VERSION();".to_string(), None)
            .await?;
        let flight_info = statement.execute().await?;

        let ticket = flight_info.endpoint[0]
            .ticket
            .as_ref()
            .expect("Ticket not present")
            .clone();
        let flight_data = client.do_get(ticket).await?;
        let batches: Vec<_> = flight_data.try_collect().await?;
        let batch = batches.first().expect("batch 0 not present");
        let array = batch.columns().first().expect("column not present");
        let data = array.to_data();
        let buffers = data.buffers();
        let buffer = buffers.get(1).expect("buffer not present");
        let values = buffer.to_vec();
        let version = String::from_utf8(values)?;

        assert_eq!(version, "v1.0.0");
        Ok(())
    }
}
