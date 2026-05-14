use bollard::models::{BuildInfo, ImageSummary};
use bollard::query_parameters::{
    BuildImageOptions, CreateImageOptions, ListImagesOptions, TagImageOptions,
};
use bytes::Bytes;
use futures_util::{Stream, StreamExt};

use crate::docker::{DockerClient, DockerError};

impl DockerClient {
    pub async fn pull_image(&self, name: &str, tag: &str) -> Result<(), DockerError> {
        let options = CreateImageOptions {
            from_image: Some(name.to_string()),
            tag: Some(tag.to_string()),
            ..Default::default()
        };

        let mut stream = self.inner().create_image(Some(options), None, None);

        while let Some(result) = stream.next().await {
            result?;
        }

        Ok(())
    }

    pub async fn remove_image(&self, name: &str) -> Result<(), DockerError> {
        self.inner()
            .remove_image(
                name,
                None::<bollard::query_parameters::RemoveImageOptions>,
                None,
            )
            .await?;
        Ok(())
    }

    pub fn build_image(
        &self,
        tag: &str,
        tar: Bytes,
        no_cache: bool,
    ) -> impl Stream<Item = Result<BuildInfo, DockerError>> + '_ {
        let options = BuildImageOptions {
            t: Some(tag.to_string()),
            rm: true,
            forcerm: true,
            nocache: no_cache,
            ..Default::default()
        };

        self.inner()
            .build_image(options, None, Some(bollard::body_full(tar)))
            .map(|r| r.map_err(DockerError::Api))
    }

    pub async fn tag_image(&self, source: &str, repo: &str, tag: &str) -> Result<(), DockerError> {
        let options = TagImageOptions {
            repo: Some(repo.to_string()),
            tag: Some(tag.to_string()),
        };
        self.inner().tag_image(source, Some(options)).await?;
        Ok(())
    }

    pub async fn list_images(
        &self,
        reference: Option<&str>,
    ) -> Result<Vec<ImageSummary>, DockerError> {
        let filters = reference.map(|r| {
            let mut f = std::collections::HashMap::new();
            f.insert("reference".to_string(), vec![r.to_string()]);
            f
        });
        let options = ListImagesOptions {
            filters,
            ..Default::default()
        };
        let images = self.inner().list_images(Some(options)).await?;
        Ok(images)
    }
}
