use testcontainers::{
    core::{ContainerPort, WaitFor},
    Image,
};

const FIREFOX_IMAGE: &str = "selenium/standalone-firefox";
const FIREFOX_TAG: &str = "144.0-geckodriver-0.36-grid-4.38.0-20251101";

const CHROME_IMAGE: &str = "selenium/standalone-chrome";
const CHROME_TAG: &str = "136.0-chromedriver-136.0-grid-4.38.0-20251101";

/// Port where the Selenium Grid is listening.
pub const DRIVER_PORT: ContainerPort = ContainerPort::Tcp(4444);
/// Port where the noVNC WebUI is listening.
pub const WEB_UI_PORT: ContainerPort = ContainerPort::Tcp(7900);

/// Module to work with [Selenium] inside of tests.
///
/// Starts an instance of Selenium Standalone (Firefox or Chrome). This uses either:
/// - the official [`selenium/standalone-firefox` docker image], or
/// - the official [`selenium/standalone-chrome` docker image].
///
/// # Example
///
/// ```
/// use testcontainers_modules::{
///     selenium::{self, Selenium},
///     testcontainers::runners::SyncRunner,
/// };
///
/// let selenium_instance = Selenium::new_firefox().start().unwrap();
///
/// let driver_port = selenium_instance
///     .get_host_port_ipv4(selenium::DRIVER_PORT)
///     .unwrap();
/// let web_ui_port = selenium_instance
///     .get_host_port_ipv4(selenium::WEB_UI_PORT)
///     .unwrap();
///
/// let driver_url = format!("http://localhost:{driver_port}");
/// let web_ui_url = format!("http://localhost:{web_ui_port}");
/// ```
///
/// [Selenium]: https://www.selenium.dev/
/// [`selenium/standalone-firefox` docker image]: https://hub.docker.com/r/selenium/standalone-firefox
/// [`selenium/standalone-chrome` docker image]: https://hub.docker.com/r/selenium/standalone-chrome
#[derive(Debug, Clone)]
pub struct Selenium {
    image: String,
    tag: String,
}

impl Selenium {
    /// Creates a new instance of a Selenium Standalone Firefox container.
    ///
    /// Image: [`selenium/standalone-firefox`](https://hub.docker.com/r/selenium/standalone-firefox)
    pub fn new_firefox() -> Self {
        Self {
            image: FIREFOX_IMAGE.to_owned(),
            tag: FIREFOX_TAG.to_owned(),
        }
    }

    /// Creates a new instance of a Selenium Standalone Chrome container.
    ///
    /// Image: [`selenium/standalone-chrome`](https://hub.docker.com/r/selenium/standalone-chrome)
    pub fn new_chrome() -> Self {
        Self {
            image: CHROME_IMAGE.to_owned(),
            tag: CHROME_TAG.to_owned(),
        }
    }
}

impl Default for Selenium {
    fn default() -> Self {
        Self::new_firefox()
    }
}

impl Image for Selenium {
    fn name(&self) -> &str {
        &self.image
    }

    fn tag(&self) -> &str {
        &self.tag
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Started Selenium Standalone")]
    }

    fn expose_ports(&self) -> &[ContainerPort] {
        &[DRIVER_PORT, WEB_UI_PORT]
    }
}

#[cfg(test)]
mod tests {
    use testcontainers::runners::AsyncRunner;

    use super::*;

    #[tokio::test]
    async fn selenium_firefox_check_status() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let selenium = Selenium::new_firefox();
        check_status_impl(selenium).await?;
        Ok(())
    }

    #[tokio::test]
    async fn selenium_chrome_check_status() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let selenium = Selenium::new_chrome();
        check_status_impl(selenium).await?;
        Ok(())
    }

    async fn check_status_impl(
        selenium: Selenium,
    ) -> Result<(), Box<dyn std::error::Error + 'static>> {
        let node = selenium.start().await?;

        let driver_port = node.get_host_port_ipv4(DRIVER_PORT).await?;
        let web_ui_port = node.get_host_port_ipv4(WEB_UI_PORT).await?;

        // Check Selenium Grid Status
        let status_url = format!("http://127.0.0.1:{driver_port}/status");
        let response = reqwest::get(&status_url).await?;
        assert!(response.status().is_success());
        let body = response.text().await?;
        assert!(body.contains("\"ready\": true"));

        // Check WebUI
        let web_ui_url = format!("http://127.0.0.1:{web_ui_port}");
        let response = reqwest::get(&web_ui_url).await?;
        assert!(response.status().is_success());

        Ok(())
    }

    #[tokio::test]
    async fn selenium_firefox_run_js() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let node = Selenium::new_firefox().start().await?;
        let driver_port = node.get_host_port_ipv4(DRIVER_PORT).await?;
        let driver_url = format!("http://127.0.0.1:{driver_port}");

        let client = fantoccini::ClientBuilder::native()
            .connect(&driver_url)
            .await?;

        let result = client.execute("return 1 + 1", vec![]).await?;
        let value = result.as_i64().unwrap();
        assert_eq!(value, 2);

        client.close().await?;

        Ok(())
    }
}
