use semver::Version;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct SpecPackage {
    pub name: String,
    pub authors: Vec<String>,
    pub license: String,
    pub description: String,
    pub homepage: String,
    pub repo: String,
    pub git_tag_format: Option<String>,
}

impl SpecPackage {
    pub(crate) fn git_tag(&self, version: &Version) -> String {
        self.git_tag_format
            .as_ref()
            .map_or(version.to_string(), |f| {
                f.replace("$VERSION", &version.to_string())
            })
    }
}

#[derive(Deserialize)]
pub struct TargetGithubRelease {}
#[derive(Deserialize)]
pub struct TargetSqlpkg {}
#[derive(Deserialize)]
pub struct TargetSpm {}

#[derive(Deserialize)]
pub struct TargetDatasette {}
#[derive(Deserialize)]
pub struct TargetPip {
    pub(crate) extra_init_py: Option<String>,
}

#[derive(Deserialize)]
pub struct TargetSqliteUtils {}

#[derive(Deserialize)]
pub struct TargetNpm {}

#[derive(Deserialize)]
pub struct TargetGem {
    pub module_name: String,
}
#[derive(Deserialize)]
pub struct TargetAmalgamation {
    pub include: Vec<String>,
}

#[derive(Deserialize)]
pub struct Targets {
    pub github_releases: Option<TargetGithubRelease>,
    pub sqlpkg: Option<TargetSqlpkg>,
    pub spm: Option<TargetSpm>,
    pub pip: Option<TargetPip>,
    pub datasette: Option<TargetDatasette>,
    pub sqlite_utils: Option<TargetSqliteUtils>,
    pub npm: Option<TargetNpm>,
    pub gem: Option<TargetGem>,
    pub amalgamation: Option<TargetAmalgamation>,
}
#[derive(Deserialize)]
pub struct Spec {
    pub package: SpecPackage,
    pub targets: Targets,
}
