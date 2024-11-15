use std::{
    collections::HashMap,
    io::{self, Cursor, Write},
    path::Path,
};

use crate::{AssetPipWheel, Cpu, GeneratedAsset, GeneratedAssetKind, Os, Project};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use semver::Version;
use sha2::{Digest, Sha256};
use zip::{result::ZipError, write::FileOptions, ZipWriter};

mod templates {
    use crate::{pip::platform_target_tag, Cpu, Os};

    use super::PipPackage;

    pub(crate) fn dist_info_metadata(pkg: &PipPackage) -> String {
        let name = &pkg.package_name;
        let version = &pkg.package_version;
        let extra_metadata: String = if pkg.extra_metadata.len() > 1 {
            let mut s = String::new();
            for (key, value) in &pkg.extra_metadata {
                s += format!("{key}: {value}\n").as_str();
            }
            s
        } else {
            "".to_owned()
        };
        format!(
            "Metadata-Version: 2.1
Name: {name}
Version: {version}
Home-page: https://TODO.com
Author: TODO
License: MIT License, Apache License, Version 2.0
Description-Content-Type: text/markdown
{extra_metadata}

TODO readme"
        )
    }

    pub(crate) fn dist_info_entrypoints(entrypoints: &Vec<(String, String)>) -> String {
        let mut txt = String::new();
        for (key, value) in entrypoints {
            txt += format!("[{key}]\n").as_str();
            txt += value;
            txt += "\n\n";
        }

        txt
    }
    pub(crate) fn dist_info_wheel(platform: Option<(&Os, &Cpu)>) -> String {
        let name = env!("CARGO_PKG_NAME");
        let version = env!("CARGO_PKG_VERSION");
        let platform_tag = match platform {
            Some((os, cpu)) => platform_target_tag(os, cpu),
            None => "any".to_owned(),
        };
        let tag = format!("py3-none-{platform_tag}");
        format!(
            "Wheel-Version: 1.0
Generator: {name} {version}
Root-Is-Purelib: false
Tag: {tag}",
        )
    }
    pub(crate) fn dist_info_top_level_txt(pkg: &PipPackage) -> String {
        format!("{}\n", pkg.python_package_name)
    }

    pub(crate) fn dist_info_record(pkg: &PipPackage, record_path: &str) -> String {
        let mut record = String::new();
        for file in &pkg.written_files {
            record.push_str(format!("{},sha256={},{}\n", file.path, file.hash, file.size).as_str());
        }

        // RECORD one can be empty
        record.push_str(format!("{},,\n", record_path).as_str());

        record
    }
    pub(crate) fn base_init_py(pkg: &PipPackage, entrypoint: &str) -> String {
        let version = &pkg.package_version;
        let package_name = &pkg.package_name;
        format!(
            r#"
from os import path
import sqlite3

__version__ = "{version}"
__version_info__ = tuple(__version__.split("."))

def loadable_path():
  """ Returns the full path to the {package_name} loadable SQLite extension bundled with this package """

  loadable_path = path.join(path.dirname(__file__), "{entrypoint}")
  return path.normpath(loadable_path)

def load(conn: sqlite3.Connection)  -> None:
  """ Load the {package_name} SQLite extension into the given database connection. """

  conn.load_extension(loadable_path())

"#,
        )
    }

    pub(crate) fn sqlite_utils_init_py(dep_pkg: &PipPackage) -> String {
        let dep_library = dep_pkg.python_package_name.clone();
        let version = dep_pkg.package_version.clone();
        format!(
            r#"
from sqlite_utils import hookimpl
import {dep_library}

__version__ = "{version}"
__version_info__ = tuple(__version__.split("."))

@hookimpl
def prepare_connection(conn):
  conn.enable_load_extension(True)
  {dep_library}.load(conn)
  conn.enable_load_extension(False)
"#
        )
    }

    pub(crate) fn datasette_init_py(dep_pkg: &PipPackage) -> String {
        let dep_library = dep_pkg.python_package_name.clone();
        let version = dep_pkg.package_version.clone();
        format!(
            r#"
from datasette import hookimpl
import {dep_library}

__version__ = "{version}"
__version_info__ = tuple(__version__.split("."))

@hookimpl
def prepare_connection(conn):
  conn.enable_load_extension(True)
  {dep_library}.load(conn)
  conn.enable_load_extension(False)
"#,
        )
    }
}

pub struct PipPackageFile {
    path: String,
    hash: String,
    size: usize,
}

impl PipPackageFile {
    fn new(path: &str, data: &[u8]) -> Self {
        let hash = URL_SAFE_NO_PAD.encode(Sha256::digest(data));
        Self {
            path: path.to_owned(),
            hash,
            size: data.len(),
        }
    }
}

fn semver_to_pip_version(v: &Version) -> String {
    match (
        (!v.pre.is_empty()).then(|| v.pre.clone()),
        (!v.build.is_empty()).then(|| v.build.clone()),
    ) {
        (None, None) => v.to_string(),
        // ???
        (None, Some(_build)) => v.to_string(),
        (Some(pre), None) => {
            let base = Version::new(v.major, v.minor, v.patch).to_string();
            let (a, b) = pre.split_once('.').unwrap();
            match a {
                "alpha" => format!("{base}a{b}"),
                "beta" => format!("{base}b{b}"),
                "rc" => format!("{base}rc{b}"),
                _ => todo!(),
            }
        }
        (Some(_pre), Some(_build)) => todo!(),
    }
    /*if v.pre.is_empty() && v.build.is_empty() {
        v.to_string()
    } else if v.build.is_empty() {
    }*/
}

pub fn platform_target_tag(os: &Os, cpu: &Cpu) -> String {
    match (os, cpu) {
        (Os::Macos, Cpu::X86_64) => "macosx_10_6_x86_64".to_owned(),
        (Os::Macos, Cpu::Aarch64) => "macosx_11_0_arm64".to_owned(),
        (Os::Linux, Cpu::X86_64) => {
            "manylinux_2_17_x86_64.manylinux2014_x86_64.manylinux1_x86_64".to_owned()
        }
        (Os::Linux, Cpu::Aarch64) => "manylinux_2_17_aarch64.manylinux2014_aarch64".to_owned(),
        (Os::Windows, Cpu::X86_64) => "win_amd64".to_owned(),
        _ => {
            unreachable!(
                "Invalid pip platform {:?}-{:?} provided, should have been filtered out",
                os, cpu
            )
        }
    }
}

pub struct PipPackage {
    pub zipfile: ZipWriter<Cursor<Vec<u8>>>,
    // as-is, with dashes, not python code safe
    pub package_name: String,
    // dashes replaced with underscores
    pub python_package_name: String,

    // not semver, but the special pip version string (ex 1.2a3)
    pub package_version: String,
    pub written_files: Vec<PipPackageFile>,

    pub entrypoints: Vec<(String, String)>,
    pub extra_metadata: Vec<(String, String)>,
}

impl PipPackage {
    pub fn new<S: Into<String>>(package_name: S, package_version: &Version) -> Self {
        let buffer = Cursor::new(Vec::new());
        let zipfile = zip::ZipWriter::new(buffer);
        let package_name = package_name.into();
        Self {
            zipfile,
            package_name: package_name.clone(),
            python_package_name: package_name.replace('-', "_"),
            package_version: semver_to_pip_version(package_version),
            written_files: vec![],
            entrypoints: vec![],
            extra_metadata: vec![],
        }
    }

    pub fn add_entrypoint(&mut self, key: &str, value: &str) {
        self.entrypoints.push((key.to_owned(), value.to_owned()));
    }

    pub fn wheel_name(&self, platform: Option<(&Os, &Cpu)>) -> String {
        let name = &self.python_package_name;
        let version = &self.package_version;
        let python_tag = "py3";
        let abi_tag = "none";
        let platform_tag = match platform {
            Some((os, cpu)) => platform_target_tag(os, cpu),
            None => "any".to_owned(),
        };
        format!("{name}-{version}-{python_tag}-{abi_tag}-{platform_tag}.whl")
    }

    fn write_file(&mut self, path: &str, data: &[u8]) -> Result<(), ZipError> {
        let options = FileOptions::default().compression_method(zip::CompressionMethod::Stored);
        self.zipfile.start_file(path, options)?;
        self.zipfile.write_all(data)?;
        self.written_files.push(PipPackageFile::new(path, data));
        Ok(())
    }

    pub fn write_library_file(&mut self, path: &str, data: &[u8]) -> Result<(), ZipError> {
        self.write_file(
            format!("{}/{}", self.python_package_name, path).as_str(),
            data,
        )
    }

    fn dist_info_file(&self, file: &str) -> String {
        format!(
            "{}-{}.dist-info/{}",
            self.python_package_name, self.package_version, file
        )
    }

    fn write_dist_info_metadata(&mut self) -> Result<(), ZipError> {
        self.write_file(
            self.dist_info_file("METADATA").as_str(),
            templates::dist_info_metadata(self).as_bytes(),
        )
    }

    fn write_dist_info_record(&mut self) -> Result<(), ZipError> {
        let record_path = self.dist_info_file("RECORD");
        self.write_file(
            &record_path,
            templates::dist_info_record(self, &record_path).as_bytes(),
        )
    }
    fn write_dist_info_top_level_txt(&mut self) -> Result<(), ZipError> {
        self.write_file(
            self.dist_info_file("top_level.txt").as_str(),
            templates::dist_info_top_level_txt(self).as_bytes(),
        )
    }
    fn write_dist_info_wheel(&mut self, platform: Option<(&Os, &Cpu)>) -> Result<(), ZipError> {
        self.write_file(
            self.dist_info_file("WHEEL").as_str(),
            templates::dist_info_wheel(platform).as_bytes(),
        )
    }
    fn write_dist_info_entrypoints(&mut self) -> Result<(), ZipError> {
        self.write_file(
            self.dist_info_file("entry_points.txt").as_str(),
            templates::dist_info_entrypoints(&self.entrypoints).as_bytes(),
        )
    }

    pub fn end(mut self, platform: Option<(&Os, &Cpu)>) -> Result<Cursor<Vec<u8>>, ZipError> {
        self.write_dist_info_metadata()?;
        self.write_dist_info_wheel(platform)?;
        if !self.entrypoints.is_empty() {
            self.write_dist_info_entrypoints()?;
        }
        self.write_dist_info_top_level_txt()?;
        self.write_dist_info_record()?;
        self.zipfile.finish()
    }
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum PipBuildError {
    #[error("Zipfile error: {0}")]
    ZipError(#[from] ZipError),
    #[error("I/O error: {0}")]
    IOError(#[from] io::Error),
}

pub(crate) fn write_base_packages(
    project: &Project,
    pip_path: &Path,
) -> Result<Vec<GeneratedAsset>, PipBuildError> {
    let mut assets = vec![];
    for platform_dir in &project.platform_directories {
        // only a subset of platforms are supported in pip
        match (&platform_dir.os, &platform_dir.cpu) {
            (Os::Macos, Cpu::X86_64)
            | (Os::Macos, Cpu::Aarch64)
            | (Os::Linux, Cpu::X86_64)
            | (Os::Linux, Cpu::Aarch64)
            | (Os::Windows, Cpu::X86_64) => (),
            //(Os::Linux, Cpu::Aarch64) => todo!(),
            //(Os::Windows, Cpu::Aarch64) => todo!(),
            _ => continue,
        }
        let mut pkg = PipPackage::new(&project.spec.package.name, &project.version);
        assert!(!platform_dir.loadable_files.is_empty());
        let entrypoint = &platform_dir.loadable_files.first().expect("TODO").file_stem;
        let mut init_py = templates::base_init_py(&pkg, entrypoint);
        if let Some(extra_init_py) = project
            .spec
            .targets
            .pip
            .as_ref()
            .and_then(|pip| pip.extra_init_py.as_deref())
        {
            let contents = std::fs::read_to_string(project.spec_directory.join(extra_init_py))?;
            init_py += &contents;
        }
        pkg.write_library_file("__init__.py", init_py.as_bytes())?;

        for f in &platform_dir.loadable_files {
            pkg.write_library_file(f.file.name.as_str(), &f.file.data)?;
        }
        let platform = Some((&platform_dir.os, &platform_dir.cpu));
        let wheel_name = pkg.wheel_name(platform);
        let result = pkg.end(platform)?.into_inner();
        let wheel_path = pip_path.join(wheel_name);
        assets.push(GeneratedAsset::from(
            GeneratedAssetKind::Pip(AssetPipWheel::Standard((
                platform_dir.os.clone(),
                platform_dir.cpu.clone(),
            ))),
            &wheel_path,
            &result,
        )?);
    }
    Ok(assets)
}

pub(crate) fn write_datasette(
    project: &Project,
    datasette_path: &Path,
) -> Result<GeneratedAsset, PipBuildError> {
    let datasette_package_name = format!("datasette-{}", project.spec.package.name);
    let dep_pkg = PipPackage::new(&project.spec.package.name, &project.version);
    let mut pkg = PipPackage::new(datasette_package_name.clone(), &project.version);
    pkg.write_library_file(
        "__init__.py",
        templates::datasette_init_py(&dep_pkg).as_bytes(),
    )?;

    pkg.add_entrypoint(
        "datasette",
        format!(
            "{} = {}",
            dep_pkg.python_package_name, pkg.python_package_name
        )
        .as_str(),
    );
    pkg.extra_metadata
        .push(("Requires-Dist".to_owned(), "datasette".to_owned()));
    pkg.extra_metadata.push((
        "Requires-Dist".to_owned(),
        format!("{} (=={})", &project.spec.package.name, &project.version),
    ));

    let wheel_name = pkg.wheel_name(None);
    let result = pkg.end(None)?.into_inner();
    Ok(GeneratedAsset::from(
        GeneratedAssetKind::Datasette,
        &datasette_path.join(wheel_name),
        &result,
    )?)
}

pub(crate) fn write_sqlite_utils(
    project: &Project,
    sqlite_utils_path: &Path,
) -> Result<GeneratedAsset, PipBuildError> {
    let sqlite_utils_name = format!("sqlite-utils-{}", project.spec.package.name);
    let dep_pkg = PipPackage::new(&project.spec.package.name, &project.version);
    let mut pkg = PipPackage::new(sqlite_utils_name.clone(), &project.version);
    pkg.write_library_file(
        "__init__.py",
        templates::sqlite_utils_init_py(&dep_pkg).as_bytes(),
    )?;

    pkg.add_entrypoint(
        "sqlite_utils",
        format!(
            "{} = {}",
            dep_pkg.python_package_name, pkg.python_package_name
        )
        .as_str(),
    );

    pkg.extra_metadata
        .push(("Requires-Dist".to_owned(), "sqlite-utils".to_owned()));
    pkg.extra_metadata.push((
        "Requires-Dist".to_owned(),
        format!("{} (=={})", &project.spec.package.name, &project.version),
    ));

    let wheel_name = pkg.wheel_name(None);

    let result = pkg.end(None)?.into_inner();
    Ok(GeneratedAsset::from(
        GeneratedAssetKind::SqliteUtils,
        &sqlite_utils_path.join(wheel_name),
        &result,
    )?)
}
