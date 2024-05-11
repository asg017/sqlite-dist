use std::collections::HashMap;
use std::io::Result;
use std::path::Path;

use crate::{GeneratedAsset, GeneratedAssetKind, Project};

use serde::{Deserialize, Serialize};

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

pub(crate) fn write_sqlpkg(project: &Project, sqlpkg_dir: &Path) -> Result<Vec<GeneratedAsset>> {
    let sqlpkg = Sqlpkg {
        owner: project.spec.package.authors.join(", "),
        name: project.spec.package.name.clone(),
        version: project.version.to_string(),
        homepage: project.spec.package.homepage.clone(),
        repository: project.spec.package.repo.clone(),
        authors: project.spec.package.authors.clone(),
        license: project.spec.package.license.clone(),
        description: project.spec.package.description.clone(),
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
