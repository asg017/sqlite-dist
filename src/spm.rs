use serde::Serialize;

use crate::spec::Spec;
use crate::{Cpu, GeneratedAsset, GeneratedAssetKind, Os};

#[derive(Debug, Serialize)]
pub struct PlatformAsset {
    os: Os,
    cpu: Cpu,
    url: String,
    checksum_sha256: String,
}

#[derive(Debug, Serialize)]
pub struct SpmJson {
    pub version: u32,
    pub description: String,
    pub loadable: Vec<PlatformAsset>,
    #[serde(rename = "static")]
    pub static_: Option<Vec<PlatformAsset>>,
}

use std::io::Result;
use std::path::Path;

pub(crate) fn write_spm(
    spec: &Spec,
    gh_release_assets: &[GeneratedAsset],
    spm_path: &Path,
) -> Result<Vec<GeneratedAsset>> {
    let loadable = gh_release_assets
        .iter()
        //.filter(|asset| matches!(asset.kind, GeneratedAssetKind::GithubReleaseLoadable(_)))
        .filter_map(|asset| match &asset.kind {
            GeneratedAssetKind::GithubReleaseLoadable(github_release) => Some(PlatformAsset {
                os: github_release.platform.0.clone(),
                cpu: github_release.platform.1.clone(),
                url: github_release.url.clone(),
                checksum_sha256: asset.checksum_sha256.clone(),
            }),
            _ => None,
        })
        .collect();
    let static_: Vec<PlatformAsset> = gh_release_assets
        .iter()
        .filter_map(|asset| match &asset.kind {
            GeneratedAssetKind::GithubReleaseStatic(github_release) => Some(PlatformAsset {
                os: github_release.platform.0.clone(),
                cpu: github_release.platform.1.clone(),
                url: github_release.url.clone(),
                checksum_sha256: asset.checksum_sha256.clone(),
            }),
            _ => None,
        })
        .collect();
    let static_ = if static_.is_empty() {
        None
    } else {
        Some(static_)
    };
    let spm_json = SpmJson {
        version: 0,
        description: spec.package.description.clone(),
        loadable,
        static_,
    };
    let asset = GeneratedAsset::from(
        GeneratedAssetKind::Spm,
        &spm_path.join("spm.json"),
        serde_json::to_string_pretty(&spm_json)?.as_bytes(),
    )?;
    Ok(vec![asset])
}
