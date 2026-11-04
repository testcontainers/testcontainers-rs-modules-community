use std::path::PathBuf;

use serde::Deserialize;

#[derive(Deserialize, Default)]
pub(super) struct K3dConfig {
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub files: Option<Vec<File>>,
    #[serde(default)]
    pub registries: Registries,
    #[serde(default)]
    pub options: Options,
}

#[derive(Deserialize, Default)]
pub(super) struct Registries {
    #[serde(default, deserialize_with = "Registries::deserialize_registries")]
    pub config: Option<super::Registries>,
}

impl Registries {
    fn deserialize_registries<'de, D>(
        deserializer: D,
    ) -> Result<Option<super::Registries>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let nested_yaml = String::deserialize(deserializer)?;
        Ok(Some(
            noyalib::from_str::<super::Registries>(&nested_yaml).map_err(|err| {
                let err = err.render(&nested_yaml);
                serde::de::Error::custom(err)
            })?,
        ))
    }
}

#[derive(Deserialize)]
pub(super) struct File {
    pub source: String,
    pub destination: PathBuf,
}

#[derive(Deserialize, Default)]
pub(super) struct Options {
    #[serde(default)]
    pub k3s: K3sExtraArgs,
}

#[derive(Deserialize, Default)]
pub(super) struct K3sExtraArgs {
    #[serde(rename = "extraArgs")]
    #[serde(default)]
    pub extra_args: Option<Vec<K3sExtraArg>>,
}
#[derive(Deserialize)]
pub(super) struct K3sExtraArg {
    pub arg: String,
}
