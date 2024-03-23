use std::collections::HashMap;
use std::io::Result;
use std::path::Path;

use crate::spec::Spec;
use crate::{GeneratedAsset, GeneratedAssetKind};

use serde::{Deserialize, Serialize};

/*
#[derive(Debug, Deserialize, Serialize)]
pub struct AssetPath {
  value: String,
  is_rem
}*/

#[derive(Debug, Deserialize, Serialize)]
pub struct Asset {
    //path:
    pub pattern: Option<String>,
    pub files: HashMap<String, String>,
    pub checksums: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Sqlpkg {
    pub owner: String,
    pub name: String,
    pub version: String,
    pub homepage: String,
    pub repository: String,
    pub authors: Vec<String>,
    pub license: String,
    pub description: String,
    pub keywords: Vec<String>,
    pub symbols: Option<Vec<String>>,
    pub assets: Asset,
}

pub(crate) fn write_sqlpkg(sqlpkg_dir: &Path, spec: &Spec) -> Result<Vec<GeneratedAsset>> {
    let sqlpkg = Sqlpkg {
        owner: spec.package.authors.join(", "),
        name: spec.package.name.clone(),
        version: spec.package.version.to_string(),
        homepage: spec.package.homepage.clone(),
        repository: spec.package.repo.clone(),
        authors: spec.package.authors.clone(),
        license: spec.package.license.clone(),
        description: spec.package.description.clone(),
        keywords: vec![], // TODO keywords in spec?
        symbols: None,
        assets: Asset {
            pattern: None,
            files: HashMap::new(),
            checksums: HashMap::new(),
        },
    };
    let asset = GeneratedAsset::from(
        GeneratedAssetKind::Sqlpkg,
        &sqlpkg_dir.join("sqlpkg.json"),
        serde_json::to_string_pretty(&sqlpkg)?.as_bytes(),
    )?;
    Ok(vec![asset])
}
