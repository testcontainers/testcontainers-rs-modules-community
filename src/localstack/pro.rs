use std::borrow::Cow;

use testcontainers::{core::WaitFor, Image};

const NAME: &str = "localstack/localstack-pro";

/// This module provides [LocalStack](https://www.localstack.cloud/) (Pro Edition).
///
/// Currently pinned to [version `4.5`](https://hub.docker.com/layers/localstack/localstack/4.5/images/sha256-acc5bf76bd8542897e6326c82f737a980791b998e4d641bcd1560902938ac305?context=explore)
///
/// # Configuration
///
/// For configuration, LocalStack uses environment variables. You can go [here](https://docs.localstack.cloud/references/configuration/)
/// for the full list.
///
/// Testcontainers support setting environment variables with the method
/// `RunnableImage::with_env_var((impl Into<String>, impl Into<String>))`. You will have to convert
/// the Image into a RunnableImage first.
///
/// ```
/// use testcontainers_modules::{localstack::LocalStackPro, testcontainers::ImageExt};
///
/// let container_request =
///     LocalStackPro::new("YOUR AUTH TOKEN HERE").with_env_var("SERVICES", "s3");
/// ```
///
/// No environment variables are required.
#[derive(Clone)]
pub struct LocalStackPro {
    /// The [auth token](https://docs.localstack.cloud/getting-started/auth-token/)
    /// to activate LocalStack Pro with
    auth_token: Option<String>,
}

impl LocalStackPro {
    /// Create new [`LocalStackPro`](LocalStackPro) container instance using the specified
    /// LocalStack [auth token](https://docs.localstack.cloud/getting-started/auth-token/)
    pub fn new(auth_token: impl Into<String>) -> Self {
        Self::with_auth_token(Some(auth_token))
    }

    /// Create new [`LocalStackPro`](LocalStackPro) container instance using the
    /// [auth token](https://docs.localstack.cloud/getting-started/auth-token/)
    /// from the local `LOCALSTACK_AUTH_TOKEN` environment variable
    pub fn from_env() -> Self {
        Self::with_auth_token(std::env::var("LOCALSTACK_AUTH_TOKEN").ok())
    }

    /// Create new [`LocalStackPro`](LocalStackPro) container instance using the
    /// specified (optional) [auth token](https://docs.localstack.cloud/getting-started/auth-token/)
    pub fn with_auth_token(auth_token: Option<impl Into<String>>) -> Self {
        Self {
            auth_token: auth_token.map(Into::into),
        }
    }
}

impl Default for LocalStackPro {
    fn default() -> Self {
        Self::from_env()
    }
}

impl std::fmt::Debug for LocalStackPro {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LocalStackPro")
            .field("auth_token", &{
                if self.auth_token.is_none() {
                    "[UNSET]"
                } else {
                    "[REDACTED]"
                }
            })
            .finish()
    }
}

impl Image for LocalStackPro {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        super::TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Ready."), WaitFor::healthcheck()]
    }
    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        if let Some(token) = self.auth_token.as_deref() {
            vec![("LOCALSTACK_AUTH_TOKEN", token), ("ACTIVATE_PRO", "1")]
        } else {
            vec![("ACTIVATE_PRO", "0")]
        }
    }
}

#[cfg(test)]
mod tests {
    use testcontainers::runners::AsyncRunner;

    use super::LocalStackPro;

    #[tokio::test]
    #[should_panic]
    #[allow(clippy::result_large_err)]
    async fn fails_on_invalid_token() {
        LocalStackPro::new("not-a-real-auth-token")
            .start()
            .await
            .unwrap();
    }

    #[tokio::test]
    #[allow(clippy::result_large_err)]
    async fn skips_pro_activation_without_token() {
        let container = LocalStackPro::with_auth_token(Option::<&str>::None)
            .start()
            .await;

        assert!(container.is_ok());

        let stdout = container
            .unwrap()
            .stdout_to_vec()
            .await
            .map(|value| String::from_utf8_lossy(&value).to_string())
            .unwrap_or_default();

        assert!(
            stdout.trim().ends_with("Ready."),
            "expected a string ending with \"Ready.\" but got: {stdout}",
        );
    }
}
