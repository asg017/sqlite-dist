use crate::PlatformDirectory;
use crate::{
    create_targz, GeneratedAsset, GeneratedAssetKind, GithubRelease, PlatformFile, Project,
};
use std::io;
use std::path::Path;

fn create_loadable_github_release_asset(
    platform_directory: &PlatformDirectory,
) -> io::Result<Vec<u8>> {
    create_targz(
        &platform_directory
            .loadable_files
            .iter()
            .map(|l| &l.file)
            .collect::<Vec<&PlatformFile>>(),
    )
}

fn create_static_github_release_asset(
    platform_directory: &PlatformDirectory,
) -> Option<io::Result<Vec<u8>>> {
    let mut targets = vec![];
    targets.extend(&platform_directory.static_files);
    targets.extend(&platform_directory.header_files);
    if !targets.is_empty() {
        Some(create_targz(&targets))
    } else {
        None
    }
}

fn github_release_artifact_name(
    name: &str,
    version: &str,
    os: &str,
    cpu: &str,
    artifact_type: &str,
) -> String {
    format!("{name}-{version}-{artifact_type}-{os}-{cpu}.tar.gz")
}

fn github_release_artifact_name_loadable(
    project: &Project,
    platform_dir: &PlatformDirectory,
) -> String {
    let name = project.spec.package.name.as_str();
    let version = project.version.to_string();
    let os = platform_dir.os.to_string();
    let cpu = platform_dir.cpu.to_string();
    github_release_artifact_name(name, &version, &os, &cpu, "loadable")
}
fn github_release_artifact_name_static(
    project: &Project,
    platform_dir: &PlatformDirectory,
) -> String {
    let name = project.spec.package.name.as_str();
    let version = project.version.to_string();
    let os = platform_dir.os.to_string();
    let cpu = platform_dir.cpu.to_string();
    github_release_artifact_name(name, &version, &os, &cpu, "static")
}

pub(crate) fn write_platform_files(
    project: &Project,
    ghreleases: &Path,
) -> Result<Vec<GeneratedAsset>, io::Error> {
    let mut loadable_assets = vec![];
    let mut static_assets = vec![];

    for platform_dir in &project.platform_directories {
        let ghl = create_loadable_github_release_asset(platform_dir)?;
        let lname = github_release_artifact_name_loadable(project, platform_dir);
        loadable_assets.push(GeneratedAsset::from(
            GeneratedAssetKind::GithubReleaseLoadable(GithubRelease {
                url: project.release_download_url(&lname),
                platform: (platform_dir.os.clone(), platform_dir.cpu.clone()),
            }),
            &ghreleases.join(lname),
            &ghl,
        )?);

        if let Some(ghs) = create_static_github_release_asset(platform_dir) {
            let sname = github_release_artifact_name_static(project, platform_dir);
            static_assets.push(GeneratedAsset::from(
                GeneratedAssetKind::GithubReleaseStatic(GithubRelease {
                    url: project.release_download_url(&sname),
                    platform: (platform_dir.os.clone(), platform_dir.cpu.clone()),
                }),
                &ghreleases.join(sname),
                &ghs?,
            )?);
        }
    }
    let mut generated_assets = vec![];
    generated_assets.append(&mut loadable_assets);
    generated_assets.append(&mut static_assets);
    Ok(generated_assets)
}
