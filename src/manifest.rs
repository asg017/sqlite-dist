use crate::{GeneratedAsset, GeneratedAssetKind};
use serde::{Deserialize, Serialize};
use std::io::Result;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize)]
pub struct ManifestBuildInfo {
    sqlite_dist_version: String,
}

#[derive(Serialize)]
pub struct Manifest<'a> {
    build_info: ManifestBuildInfo,

    artifacts: &'a [GeneratedAsset],
}

pub(crate) fn write_manifest(
    manifest_dir: &Path,
    generated_assets: &[GeneratedAsset],
) -> Result<GeneratedAsset> {
    let manifest = Manifest {
        build_info: ManifestBuildInfo {
            sqlite_dist_version: "TODO".to_owned(),
        },
        artifacts: generated_assets,
    };
    let asset = GeneratedAsset::from(
        GeneratedAssetKind::Sqlpkg,
        &manifest_dir.join("sqlite-dist-manifest.json"),
        serde_json::to_string_pretty(&manifest)?.as_bytes(),
    )?;
    Ok(asset)
}
