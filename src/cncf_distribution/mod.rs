use testcontainers::{core::WaitFor, Image};

const NAME: &str = "registry";
const TAG: &str = "2";

/// Module to work with a custom Docker registry inside of tests.
///
/// Starts an instance of [`CNCF Distribution`], an easy-to-use registry for container images.
///
/// # Example
/// ```
/// use testcontainers_modules::{cncf_distribution, testcontainers::runners::SyncRunner};
///
/// let registry = cncf_distribution::CncfDistribution::default()
///     .start()
///     .unwrap();
///
/// let image_name = "test";
/// let image_tag = format!(
///     "{}:{}/{image_name}",
///     registry.get_host().unwrap(),
///     registry.get_host_port_ipv4(5000).unwrap()
/// );
///
/// // now you can push an image tagged with `image_tag` and pull it afterward
/// ```
///
/// [`CNCF Distribution`]: https://distribution.github.io/distribution/
#[derive(Debug, Default, Clone)]
pub struct CncfDistribution {
    /// (remove if there is another variable)
    /// Field is included to prevent this struct to be a unit struct.
    /// This allows extending functionality (and thus further variables) without breaking changes
    _priv: (),
}

impl Image for CncfDistribution {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stderr("listening on [::]:5000")]
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use testcontainers::{
        bollard::{
            query_parameters::{
                CreateImageOptionsBuilder, PushImageOptionsBuilder, RemoveImageOptions,
            },
            Docker,
        },
        runners::AsyncBuilder,
        GenericBuildableImage, Image,
    };

    use crate::{cncf_distribution, testcontainers::runners::AsyncRunner};

    const DOCKERFILE: &str = "
        FROM scratch
        COPY hello.sh /
    ";

    #[tokio::test]
    async fn distribution_push_pull_image() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let _ = pretty_env_logger::try_init();
        let distribution_node = cncf_distribution::CncfDistribution::default()
            .start()
            .await?;
        let docker = Docker::connect_with_local_defaults().unwrap();

        let image_name = &format!(
            "localhost:{}/test",
            distribution_node.get_host_port_ipv4(5000).await?
        );
        let image_tag = "latest";

        let image = GenericBuildableImage::new(image_name, image_tag)
            .with_dockerfile_string(DOCKERFILE)
            .with_data(b"#!/bin/sh\necho 'Hello World'", "./hello.sh")
            .build_image()
            .await?;

        // Push image, and then remove it
        let mut push_image = docker.push_image(
            image.name(),
            Some(PushImageOptionsBuilder::new().tag(image.tag()).build()),
            None,
        );
        while let Some(x) = push_image.next().await {
            println!("Push image: {:?}", x.unwrap());
        }

        docker
            .remove_image(image.name(), None::<RemoveImageOptions>, None)
            .await
            .unwrap();

        // Pull image
        let mut create_image = docker.create_image(
            Some(
                CreateImageOptionsBuilder::new()
                    .from_image(image.name())
                    .tag(image.tag())
                    .build(),
            ),
            None,
            None,
        );
        while let Some(x) = create_image.next().await {
            println!("Create image: {:?}", x.unwrap());
        }

        assert_eq!(
            docker
                .inspect_image(image.name())
                .await
                .unwrap()
                .repo_tags
                .unwrap()[0],
            format!("{}:{}", image.name(), image.tag())
        );

        // clean-up
        docker
            .remove_image(image.name(), None::<RemoveImageOptions>, None)
            .await?;

        Ok(())
    }
}
