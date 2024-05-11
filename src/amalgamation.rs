use std::io::{Cursor, Result, Write};
use std::path::Path;

use zip::write::FileOptions;

use crate::spec::TargetAmalgamation;
use crate::{create_targz, GeneratedAsset, GeneratedAssetKind, PlatformFile, Project};

pub(crate) fn write_amalgamation(
    project: &Project,
    amalgamation_dir: &Path,
    amalgamation_config: &TargetAmalgamation,
) -> Result<Vec<GeneratedAsset>> {
    let files: Result<Vec<PlatformFile>> = amalgamation_config
        .include
        .iter()
        .map(|relative_path| {
            let path = project.spec_directory.join(relative_path);
            let data = std::fs::read_to_string(&path)?;
            Ok(PlatformFile {
                name: relative_path.to_owned(),
                data: data.into(),
                metadata: Some(std::fs::metadata(&path)?),
            })
        })
        .collect();
    let files = files?;
    let mut assets = vec![];

    let targz = create_targz(files.iter().collect::<Vec<&PlatformFile>>().as_ref())?;
    assets.push(GeneratedAsset::from(
        GeneratedAssetKind::Amalgamation,
        &amalgamation_dir.join(format!(
            "{}-{}-amalgamation.tar.gz",
            project.spec.package.name, project.version
        )),
        &targz,
    )?);

    let buffer = Cursor::new(Vec::new());
    let mut zipfile = zip::ZipWriter::new(buffer);
    for file in files {
        let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        zipfile.start_file(file.name, options)?;
        zipfile.write_all(&file.data)?;
    }
    assets.push(GeneratedAsset::from(
        GeneratedAssetKind::Amalgamation,
        &amalgamation_dir.join(format!(
            "{}-{}-amalgamation.zip",
            project.spec.package.name, project.version
        )),
        &zipfile.finish()?.into_inner(),
    )?);

    Ok(assets)
}
