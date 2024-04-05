use testcontainers::{core::WaitFor, Image};

const NAME: &str = "registry";
const TAG: &str = "2";

/// Module to work with a custom Docker registry inside of tests.
///
/// Starts an instance of [`CNCF Distribution`], an easy to use registry for container images.
///
/// # Example
/// ```
/// use testcontainers::clients;
/// use testcontainers_modules::cncf_distribution;
///
/// let docker = clients::Cli::default();
/// let registry = docker.run(cncf_distribution::CncfDistribution);
///
/// let image_name = "test";
/// let image_tag = format!("localhost:{}/{image_name}", registry.get_host_port_ipv4(5000));
///
/// // now you can push an image tagged with `image_tag` and pull it afterwards
/// ```
///
/// [`CNCF Distribution`]: https://distribution.github.io/distribution/
#[derive(Debug, Default, Clone)]
pub struct CncfDistribution;

impl Image for CncfDistribution {
    type Args = ();

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stderr("listening on [::]:5000")]
    }
}

#[cfg(test)]
mod tests {
    use bollard::image::{BuildImageOptions, CreateImageOptions};
    use futures::StreamExt;
    use testcontainers::clients;

    use crate::cncf_distribution;

    const DOCKERFILE: &[u8] = b"
        FROM scratch
        COPY Dockerfile /
    ";

    #[tokio::test]
    async fn distribution_push_pull_image() {
        let _ = pretty_env_logger::try_init();
        let docker = clients::Cli::default();
        let distribution_node = docker.run(cncf_distribution::CncfDistribution);
        let docker = bollard::Docker::connect_with_local_defaults().unwrap();
        let image_tag = format!(
            "localhost:{}/test:latest",
            distribution_node.get_host_port_ipv4(5000)
        );

        let mut archive = tar::Builder::new(Vec::new());
        let mut header = tar::Header::new_gnu();
        header.set_path("Dockerfile").unwrap();
        header.set_size(DOCKERFILE.len() as u64);
        header.set_cksum();
        archive.append(&header, DOCKERFILE).unwrap();

        // Build test image
        let mut build_image = docker.build_image(
            BuildImageOptions {
                dockerfile: "Dockerfile",
                t: &image_tag,
                ..Default::default()
            },
            None,
            Some(archive.into_inner().unwrap().into()),
        );
        while let Some(x) = build_image.next().await {
            println!("Build status: {:?}", x.unwrap());
        }

        // Push image, and then remove it
        let mut push_image = docker.push_image::<String>(&image_tag, None, None);
        while let Some(x) = push_image.next().await {
            println!("Push image: {:?}", x.unwrap());
        }
        docker.remove_image(&image_tag, None, None).await.unwrap();

        // Pull image
        let mut create_image = docker.create_image(
            Some(CreateImageOptions {
                from_image: image_tag.as_str(),
                ..Default::default()
            }),
            None,
            None,
        );
        while let Some(x) = create_image.next().await {
            println!("Create image: {:?}", x.unwrap());
        }

        assert_eq!(
            docker
                .inspect_image(&image_tag)
                .await
                .unwrap()
                .repo_tags
                .unwrap()[0],
            image_tag,
        );
    }
}
