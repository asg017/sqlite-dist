mod amalgamation;
mod gem;
mod gh_releases;
mod installer_sh;
mod manifest;
mod npm;
mod pip;
mod spec;
mod spm;
mod sqlpkg;

use clap::{value_parser, Arg, ArgMatches, Command};
use flate2::write::GzEncoder;
use flate2::Compression;
use manifest::write_manifest;
use npm::NpmBuildError;
use pip::PipBuildError;
use semver::Version;
use serde::{Serialize, Serializer};
use sha2::{Digest, Sha256};
use spec::Spec;
use std::{
    fs::{self, File},
    io::{self, Write},
    path::PathBuf,
};
use tar::Header;

struct Project {
    version: Version,
    spec: Spec,
    spec_directory: PathBuf,
    platform_directories: Vec<PlatformDirectory>,
}

impl Project {
    pub(crate) fn release_download_url(&self, name: &str) -> String {
        let gh_base = self.spec.package.repo.clone();
        let tag_version = self.version.to_string();
        format!("{gh_base}/releases/download/{tag_version}/{name}")
    }
}

#[derive(Debug, Clone)]
struct PlatformDirectory {
    os: Os,
    cpu: Cpu,
    _path: PathBuf,
    loadable_files: Vec<LoadablePlatformFile>,
    static_files: Vec<PlatformFile>,
    header_files: Vec<PlatformFile>,
}

#[derive(Debug, Clone)]
enum Os {
    Macos,
    Linux,
    Windows,
}

impl Serialize for Os {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

impl ToString for Os {
    fn to_string(&self) -> String {
        match self {
            Os::Macos => "macos".to_owned(),
            Os::Linux => "linux".to_owned(),
            Os::Windows => "windows".to_owned(),
        }
    }
}

#[derive(Debug, Clone)]
enum Cpu {
    X86_64,
    Aarch64,
}

impl Serialize for Cpu {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

impl ToString for Cpu {
    fn to_string(&self) -> String {
        match self {
            Cpu::X86_64 => "x86_64".to_owned(),
            Cpu::Aarch64 => "aarch64".to_owned(),
        }
    }
}

#[derive(Debug, Clone)]
struct GithubRelease {
    url: String,
    platform: (Os, Cpu),
}

#[derive(Debug, Clone)]
enum GeneratedAssetKind {
    Npm(Option<(Os, Cpu)>),
    Gem((Os, Cpu)),
    Pip((Os, Cpu)),
    Datasette,
    SqliteUtils,
    GithubReleaseLoadable(GithubRelease),
    GithubReleaseStatic(GithubRelease),
    Sqlpkg,
    Spm,
    Amalgamation,
    Manifest,
}

impl ToString for GeneratedAssetKind {
    fn to_string(&self) -> String {
        match self {
            GeneratedAssetKind::Npm(_) => "npm".to_owned(),
            GeneratedAssetKind::Gem(_) => "gem".to_owned(),
            GeneratedAssetKind::Pip(_) => "pip".to_owned(),
            GeneratedAssetKind::Datasette => "datasette".to_owned(),
            GeneratedAssetKind::SqliteUtils => "sqlite-utils".to_owned(),
            GeneratedAssetKind::GithubReleaseLoadable(_) => "github-release-loadable".to_owned(),
            GeneratedAssetKind::GithubReleaseStatic(_) => "github-release-static".to_owned(),
            GeneratedAssetKind::Sqlpkg => "sqlpkg".to_owned(),
            GeneratedAssetKind::Spm => "spm".to_owned(),
            GeneratedAssetKind::Amalgamation => "amalgamation".to_owned(),
            GeneratedAssetKind::Manifest => "sqlite-dist-manifest".to_owned(),
        }
    }
}
impl Serialize for GeneratedAssetKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

#[derive(Serialize)]
struct GeneratedAsset {
    kind: GeneratedAssetKind,
    name: String,
    path: String,
    checksum_sha256: String,
    size: usize,
}
impl GeneratedAsset {
    fn from(kind: GeneratedAssetKind, path: &PathBuf, contents: &[u8]) -> io::Result<Self> {
        File::create(path)?.write_all(contents)?;
        Ok(Self {
            kind,
            name: path.file_name().unwrap().to_str().unwrap().to_string(),
            path: path.to_str().unwrap().to_string(),
            checksum_sha256: base16ct::lower::encode_string(&Sha256::digest(contents)),
            size: contents.len(),
        })
    }
}
//{"kind": "github_release", "name": "...", "path": "./", "checksum_sha256": ""},

#[derive(Debug, Clone)]
struct PlatformFile {
    name: String,
    data: Vec<u8>,
    metadata: Option<std::fs::Metadata>,
}

#[derive(Debug, Clone)]
struct LoadablePlatformFile {
    file_stem: String,
    file: PlatformFile,
}

impl PlatformFile {
    fn new<S: Into<String>, D: Into<Vec<u8>>>(
        name: S,
        data: D,
        metadata: Option<fs::Metadata>,
    ) -> Self {
        Self {
            name: name.into(),
            data: data.into(),
            metadata,
        }
    }
}

use thiserror::Error;

fn create_targz(files: &[&PlatformFile]) -> io::Result<Vec<u8>> {
    let mut tar_gz = Vec::new();
    {
        let enc = GzEncoder::new(&mut tar_gz, Compression::default());
        let mut tar = tar::Builder::new(enc);
        for file in files {
            let mut header = Header::new_gnu();
            header.set_path(file.name.clone())?;
            header.set_size(file.data.len() as u64);
            if let Some(metadata) = &file.metadata {
                header.set_metadata(metadata);
            } else {
                header.set_mode(0o700);
                header.set_mtime(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                );
            }
            header.set_cksum();
            tar.append::<&[u8]>(&header, file.data.as_ref())?;
        }
        tar.finish()?;
    };
    Ok(tar_gz)
}

#[derive(Error, Debug)]
pub enum PlatformDirectoryError {
    #[error("I/O error: {0}")]
    IOError(#[from] io::Error),

    #[error("Expected name of directory")]
    MissingDirectoryName,
    #[error("directory or file name must contains only valid UTF-8 characters")]
    InvalidCharacters,
    #[error("directory {0} is not a valid platform directory. The format must be $OS-$CPU.")]
    InvalidDirectoryName(String),
    #[error("Invalid operation system '{0}'. Must be one of 'macos', 'linux', or 'windows'")]
    InvalidOsValue(String),
    #[error("Invalid CPU name '{0}'. Must be one of 'x86_64' or 'aarch64'")]
    InvalidCpuValue(String),
}

impl PlatformDirectory {
    fn from_path(base_path: PathBuf) -> Result<Self, PlatformDirectoryError> {
        let mut loadable_files = vec![];
        let mut static_files = vec![];
        let mut header_files = vec![];

        let dirname = base_path
            .components()
            .last()
            .ok_or(PlatformDirectoryError::MissingDirectoryName)?
            .as_os_str()
            .to_str()
            .ok_or(PlatformDirectoryError::InvalidCharacters)?;
        let mut s = dirname.split('-');
        let os = match s
            .next()
            .ok_or_else(|| PlatformDirectoryError::InvalidDirectoryName(dirname.to_owned()))?
        {
            "macos" => Os::Macos,
            "linux" => Os::Linux,
            "windows" => Os::Windows,
            os => return Err(PlatformDirectoryError::InvalidOsValue(os.to_owned())),
        };
        let cpu = match s
            .next()
            .ok_or_else(|| PlatformDirectoryError::InvalidDirectoryName(dirname.to_owned()))?
        {
            "x86_64" => Cpu::X86_64,
            "aarch64" => Cpu::Aarch64,
            cpu => return Err(PlatformDirectoryError::InvalidCpuValue(cpu.to_owned())),
        };
        if s.next().is_some() {
            return Err(PlatformDirectoryError::InvalidDirectoryName(
                dirname.to_owned(),
            ));
        }

        let dir = fs::read_dir(&base_path)?;
        for entry in dir {
            let entry_path = entry?.path();
            match entry_path.extension().and_then(|e| e.to_str()) {
                Some("so") | Some("dll") | Some("dylib") => {
                    let name = entry_path
                        .file_name()
                        .expect("file_name to exist because there is an extension")
                        .to_str()
                        .ok_or(PlatformDirectoryError::InvalidCharacters)?
                        .to_string();
                    let data = fs::read(&entry_path)?;
                    let metadata = Some(fs::metadata(&entry_path)?);
                    let file_stem = entry_path
                        .file_stem()
                        .expect("file_stem to exist because there is an extension")
                        .to_str()
                        .ok_or(PlatformDirectoryError::InvalidCharacters)?
                        .to_string();
                    loadable_files.push(LoadablePlatformFile {
                        file_stem,
                        file: PlatformFile {
                            name: name.to_string(),
                            data,
                            metadata,
                        },
                    });
                }
                Some("a") => {
                    let name = entry_path
                        .file_name()
                        .expect("file_name to exist because there is an extension")
                        .to_str()
                        .ok_or(PlatformDirectoryError::InvalidCharacters)?
                        .to_string();
                    let data = fs::read(&entry_path)?;
                    let metadata = Some(fs::metadata(&entry_path)?);
                    static_files.push(PlatformFile {
                        name: name.to_string(),
                        data,
                        metadata,
                    });
                }
                Some("h") => {
                    let name = entry_path
                        .file_name()
                        .expect("file_name to exist because there is an extension")
                        .to_str()
                        .ok_or(PlatformDirectoryError::InvalidCharacters)?
                        .to_string();
                    let data = fs::read(&entry_path)?;
                    let metadata = Some(fs::metadata(&entry_path)?);
                    header_files.push(PlatformFile {
                        name: name.to_string(),
                        data,
                        metadata,
                    });
                }
                _ => {
                    println!("Warning: unknown file type in platform directory");
                }
            }
        }
        Ok(PlatformDirectory {
            os,
            cpu,
            _path: base_path,
            loadable_files,
            static_files,
            header_files,
        })
    }
}

#[derive(Error, Debug)]
pub enum BuildError {
    #[error("`{0}` is a required argument")]
    RequiredArg(String),
    #[error("`{0}` is a required argument")]
    InvalidSpec(toml::de::Error),
    #[error("specfile error: `{0}`")]
    SpecError(String),
    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),

    #[error("Invalid platform directory: {0}")]
    PlayformDirectoryError(#[from] PlatformDirectoryError),

    #[error("Error building a pip package: {0}")]
    PipBuildEror(#[from] PipBuildError),
    #[error("Error building an npm package: {0}")]
    NpmBuildEror(#[from] NpmBuildError),
}

fn build(matches: ArgMatches) -> Result<(), BuildError> {
    // Get the values of arguments
    let input_dir = matches
        .get_one::<PathBuf>("input")
        .ok_or_else(|| BuildError::RequiredArg("input".to_owned()))?;
    let output_dir = matches
        .get_one::<PathBuf>("output")
        .ok_or_else(|| BuildError::RequiredArg("output".to_owned()))?;
    let input_file = matches
        .get_one::<PathBuf>("file")
        .ok_or_else(|| BuildError::RequiredArg("file".to_owned()))?;
    let version = matches
        .get_one::<String>("version")
        .ok_or_else(|| BuildError::RequiredArg("version".to_owned()))?;
    let version = Version::parse(version).unwrap();

    std::fs::create_dir_all(output_dir)?;

    let spec: Spec = match toml::from_str(fs::read_to_string(input_file)?.as_str()) {
        Ok(spec) => spec,
        Err(err) => {
            eprintln!("{}", err);
            return Err(BuildError::InvalidSpec(err));
        }
    };

    if spec.targets.sqlpkg.is_some() && spec.targets.github_releases.is_none() {
        return Err(BuildError::SpecError(
            "sqlpkg target requires the github_releases target".to_owned(),
        ));
    }
    if spec.targets.spm.is_some() && spec.targets.github_releases.is_none() {
        return Err(BuildError::SpecError(
            "spm target requires the github_releases target".to_owned(),
        ));
    }
    if spec.targets.datasette.is_some() && spec.targets.pip.is_none() {
        return Err(BuildError::SpecError(
            "datasette target requires the pip target".to_owned(),
        ));
    }
    if spec.targets.sqlite_utils.is_some() && spec.targets.pip.is_none() {
        return Err(BuildError::SpecError(
            "sqlite_utils target requires the pip target".to_owned(),
        ));
    }

    let platform_directories: Result<Vec<PlatformDirectory>, BuildError> = fs::read_dir(input_dir)?
        .map(|entry| {
            PlatformDirectory::from_path(
                entry
                    .map_err(|_| {
                        BuildError::SpecError("Could not read entry in input directory".to_owned())
                    })?
                    .path(),
            )
            .map_err(BuildError::PlayformDirectoryError)
        })
        .collect();
    let platform_directories = platform_directories?;

    let project = Project {
        version,
        spec,
        spec_directory: input_file.parent().unwrap().to_path_buf(),
        platform_directories,
    };

    let mut generated_assets: Vec<GeneratedAsset> = vec![];
    if project.spec.targets.github_releases.is_some() {
        let path = output_dir.join("github_releases");
        std::fs::create_dir(&path)?;
        let gh_release_assets = gh_releases::write_platform_files(&project, &path)?;

        if project.spec.targets.sqlpkg.is_some() {
            let sqlpkg_dir = output_dir.join("sqlpkg");
            std::fs::create_dir(&sqlpkg_dir)?;
            generated_assets.extend(sqlpkg::write_sqlpkg(&project, &sqlpkg_dir)?);
        };

        if project.spec.targets.spm.is_some() {
            let path = output_dir.join("spm");
            std::fs::create_dir(&path)?;
            generated_assets.extend(spm::write_spm(&project.spec, &gh_release_assets, &path)?);
        };

        if let Some(amalgamation_config) = &project.spec.targets.amalgamation {
            let amalgamation_path = output_dir.join("amalgamation");
            std::fs::create_dir(&amalgamation_path)?;
            generated_assets.extend(amalgamation::write_amalgamation(
                &project,
                &amalgamation_path,
                amalgamation_config,
            )?);
        };

        generated_assets.extend(gh_release_assets);
    };

    if project.spec.targets.pip.is_some() {
        let pip_path = output_dir.join("pip");
        std::fs::create_dir(&pip_path)?;
        generated_assets.extend(pip::write_base_packages(&project, &pip_path)?);
        if project.spec.targets.datasette.is_some() {
            let datasette_path = output_dir.join("datasette");
            std::fs::create_dir(&datasette_path)?;
            generated_assets.push(pip::write_datasette(&project, &datasette_path)?);
        }
        if project.spec.targets.sqlite_utils.is_some() {
            let sqlite_utils_path = output_dir.join("sqlite_utils");
            std::fs::create_dir(&sqlite_utils_path)?;
            generated_assets.push(pip::write_sqlite_utils(&project, &sqlite_utils_path)?);
        }
    };
    if project.spec.targets.npm.is_some() {
        let npm_output_directory = output_dir.join("npm");
        std::fs::create_dir(&npm_output_directory)?;
        generated_assets.extend(npm::write_npm_packages(&project, &npm_output_directory)?);
    };
    if let Some(gem_config) = &project.spec.targets.gem {
        let gem_path = output_dir.join("gem");
        std::fs::create_dir(&gem_path)?;
        generated_assets.extend(gem::write_gems(&project, &gem_path, gem_config)?);
    };

    let github_releases_checksums_txt = generated_assets
        .iter()
        .filter(|ga| {
            matches!(
                ga.kind,
                GeneratedAssetKind::GithubReleaseLoadable(_)
                    | GeneratedAssetKind::GithubReleaseStatic(_)
                    | GeneratedAssetKind::Sqlpkg
                    | GeneratedAssetKind::Spm
            )
        })
        .map(|ga| format!("{} {}", ga.name, ga.checksum_sha256))
        .collect::<Vec<String>>()
        .join("\n");
    File::create(output_dir.join("checksums.txt"))?
        .write_all(github_releases_checksums_txt.as_bytes())?;
    File::create(output_dir.join("install.sh"))?.write_all(
        crate::installer_sh::templates::install_sh(&project, &generated_assets).as_bytes(),
    )?;
    write_manifest(output_dir, &generated_assets)?;
    Ok(())
}

fn main() {
    let matches = Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author("Alex Garcia")
        .about("Package and distribute pre-compiled SQLite extensions")
        .arg(
            Arg::new("input")
                .long("input")
                .value_name("INPUT_DIR")
                .help("Sets the input directory")
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .value_name("OUTPUT_DIR")
                .help("Sets the output directory")
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("version")
                .long("version")
                .value_name("VERSION")
                .help("Set the version ")
                .required(true),
        )
        .arg(
            Arg::new("file")
                .value_name("FILE")
                .help("Sets the input file")
                .required(true)
                .index(1)
                .value_parser(value_parser!(PathBuf)),
        )
        .disable_version_flag(true)
        .get_matches();

    match build(matches) {
        Ok(_) => std::process::exit(0),
        Err(error) => {
            eprintln!("Build error: {error}");
            std::process::exit(1);
        }
    }
}
