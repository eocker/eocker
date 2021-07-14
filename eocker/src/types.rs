use serde::{
    de::{value, IntoDeserializer},
    Deserialize, Serialize,
};
use std::convert::TryFrom;

pub trait Media {
    fn is_distributable(&self) -> bool;
    fn is_image(&self) -> bool;
    fn is_index(&self) -> bool;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MediaType {
    #[serde(rename = "application/vnd.oci.descriptor.v1+json")]
    OCIContentDescriptor,
    #[serde(rename = "application/vnd.oci.image.index.v1+json")]
    OCIImageIndex,
    #[serde(rename = "application/vnd.oci.image.manifest.v1+json")]
    OCIManifestSchema1,
    #[serde(rename = "application/vnd.oci.image.config.v1+json")]
    OCIConfigJSON,
    #[serde(rename = "application/vnd.oci.image.layer.v1.tar+gzip")]
    OCILayer,
    #[serde(rename = "application/vnd.oci.image.layer.nondistributable.v1.tar+gzip")]
    OCIRestrictedLayer,
    #[serde(rename = "application/vnd.oci.image.layer.v1.tar")]
    OCIUncompressedLayer,
    #[serde(rename = "application/vnd.oci.image.layer.nondistributable.v1.tar")]
    OCIUncompressedRestrictedLayer,
    #[serde(rename = "application/vnd.docker.distribution.manifest.v1+json")]
    DockerManifestSchema1,
    #[serde(rename = "application/vnd.docker.distribution.manifest.v1+prettyjws")]
    DockerManifestSchema1Signed,
    #[serde(rename = "application/vnd.docker.distribution.manifest.v2+json")]
    DockerManifestSchema2,
    #[serde(rename = "application/vnd.docker.distribution.manifest.list.v2+json")]
    DockerManifestList,
    #[serde(rename = "application/vnd.docker.image.rootfs.diff.tar.gzip")]
    DockerLayer,
    #[serde(rename = "application/vnd.docker.container.image.v1+json")]
    DockerConfigJSON,
    #[serde(rename = "application/vnd.docker.plugin.v1+json")]
    DockerPluginConfig,
    #[serde(rename = "application/vnd.docker.image.rootfs.foreign.diff.tar.gzip")]
    DockerForeignLayer,
    #[serde(rename = "application/vnd.docker.image.rootfs.diff.tar")]
    DockerUncompressedLayer,
}

impl TryFrom<&str> for MediaType {
    type Error = value::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::deserialize(s.into_deserializer())
    }
}

impl Media for MediaType {
    fn is_distributable(&self) -> bool {
        match *self {
            MediaType::DockerForeignLayer => true,
            MediaType::OCIUncompressedRestrictedLayer => true,
            MediaType::OCIRestrictedLayer => true,
            _ => false,
        }
    }

    fn is_image(&self) -> bool {
        match *self {
            MediaType::DockerManifestSchema1 => true,
            MediaType::DockerManifestSchema2 => true,
            _ => false,
        }
    }

    fn is_index(&self) -> bool {
        match *self {
            MediaType::DockerManifestList => true,
            MediaType::OCIImageIndex => true,
            _ => false,
        }
    }
}
