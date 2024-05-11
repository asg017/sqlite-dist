use crate::spec::TargetGem;
use crate::{Cpu, Os};
use crate::{GeneratedAsset, GeneratedAssetKind, Project};
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256, Sha512};
use std::io::{self, Cursor};
use std::path::Path;
use std::{collections::HashMap, io::Write};
use tar::{Builder, Header};

#[derive(Debug, Deserialize, Serialize)]
pub struct Gemspec {
    name: String,
    version: String,
    // https://guides.rubygems.org/specification-reference/#authors=
    authors: Vec<String>,
    // https://guides.rubygems.org/specification-reference/#email
    email: Vec<String>,
    homepage: String,
    summary: String,
    description: String,
    licenses: Vec<String>,
    // https://guides.rubygems.org/specification-reference/#metadata
    metadata: HashMap<String, String>,
    platform: String,
    module_name: String,
}

fn gem_checksum_sha256(data: &[u8]) -> String {
    base16ct::lower::encode_string(&Sha256::digest(data))
}
fn gem_checksum_sha512(data: &[u8]) -> String {
    base16ct::lower::encode_string(&Sha512::digest(data))
}

fn gem_metadata_list_helper(items: Vec<String>) -> String {
    items
        .iter()
        .map(|item| {
            format!("- {}", {
                serde_json::to_string(&serde_json::Value::String(item.to_owned()))
                    .expect("String JSON to serialize")
            })
        })
        .collect::<Vec<String>>()
        .join("\n")
}

fn ruby_platform(os: &Os, cpu: &Cpu) -> String {
    let os = match os {
        Os::Macos => "darwin",
        Os::Linux => "linux",
        Os::Windows => "mingw32",
    };
    let cpu = match cpu {
        Cpu::X86_64 => "x86_64",
        Cpu::Aarch64 => "arm64",
    };
    format!("{cpu}-{os}")
}
#[allow(clippy::too_many_arguments)]
fn gem_metadata_template(
    os: &Os,
    cpu: &Cpu,
    name: &str,
    version: &str,
    files: Vec<String>,
    email: &str,
    authors: Vec<String>,
    licenses: Vec<String>,
    description: &str,
    summary: &str,
    homepage: &str,
) -> String {
    let ruby_platform = ruby_platform(os, cpu);
    let date = chrono::offset::Local::now().format("%Y-%m-%d").to_string();
    let authors = gem_metadata_list_helper(authors);
    let files = gem_metadata_list_helper(files);
    let licenses = gem_metadata_list_helper(licenses);

    // ?
    let version = version.replace('-', ".");
    format!(
        r#"--- !ruby/object:Gem::Specification
name: {name}
version: !ruby/object:Gem::Version
  version: {version}
platform: {ruby_platform}
authors:
{authors}
autorequire:
bindir: bin
cert_chain: []
date: {date} 00:00:00.000000000 Z
dependencies: []
description: '{description}'
summary: '{summary}'
email:
- {email}
executables: []
extensions: []
extra_rdoc_files: []
files:
{files}
homepage: '{homepage}'
licenses:
{licenses}
post_install_message:
rdoc_options: []
require_paths:
- lib
required_ruby_version: !ruby/object:Gem::Requirement
  requirements:
  - - ">="
    - !ruby/object:Gem::Version
      version: '0'
required_rubygems_version: !ruby/object:Gem::Requirement
  requirements:
  - - ">="
    - !ruby/object:Gem::Version
      version: '0'
requirements: []
rubygems_version: 3.4.10
signing_key:
specification_version: 4
test_files: []
"#
    )
}

fn checksums_yaml_gz(metadata_gz: &[u8], data_targz: &[u8]) -> io::Result<Vec<u8>> {
    let checksums_yaml = format!(
        r#"---
SHA256:
  metadata.gz: '{}'
  data.tar.gz: '{}'
SHA512:
  metadata.gz: '{}'
  data.tar.gz: '{}'
"#,
        gem_checksum_sha256(metadata_gz),
        gem_checksum_sha256(data_targz),
        gem_checksum_sha512(metadata_gz),
        gem_checksum_sha512(data_targz),
    );
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(checksums_yaml.as_bytes())?;
    encoder.finish()
}

pub struct Gem {
    library_tarball: Builder<GzEncoder<Vec<u8>>>,
    library_filenames: Vec<String>,
}

impl Gem {
    pub fn new() -> Self {
        let tar_gz: Vec<u8> = Vec::new();
        let enc = GzEncoder::new(tar_gz, Compression::default());
        let tar = tar::Builder::new(enc);
        Self {
            library_tarball: tar,
            library_filenames: vec![],
        }
    }
    pub fn write_library_file(&mut self, path: &str, data: &[u8]) -> io::Result<()> {
        let mut header = Header::new_gnu();

        header.set_path(path)?;
        header.set_size(data.len() as u64);
        header.set_mode(0o777);
        header.set_cksum();
        self.library_tarball.append::<&[u8]>(&header, data)?;
        self.library_filenames.push(path.to_string());
        Ok(())
    }

    fn metadata_gz(&self, os: &Os, cpu: &Cpu, project: &Project) -> io::Result<Vec<u8>> {
        let metadata = gem_metadata_template(
            os,
            cpu,
            &project.spec.package.name,
            project.version.to_string().as_str(),
            self.library_filenames.clone(),
            "TODO",
            project.spec.package.authors.clone(),
            vec![project.spec.package.license.clone()],
            &project.spec.package.description,
            &project.spec.package.description,
            "https://github.com/TODO",
        );
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(metadata.as_bytes())?;
        encoder.finish()
    }

    pub fn complete(
        mut self,
        os: &Os,
        cpu: &Cpu,
        project: &Project,
    ) -> io::Result<(String, Vec<u8>)> {
        let mut gem_tar: Vec<u8> = Vec::new();
        {
            let mut tar = tar::Builder::new(Cursor::new(&mut gem_tar));
            let mut header = Header::new_gnu();

            let metadata_gz = self.metadata_gz(os, cpu, project)?;
            header.set_path("metadata.gz")?;
            header.set_size(metadata_gz.len() as u64);
            header.set_cksum();
            tar.append::<&[u8]>(&header, metadata_gz.as_ref())?;

            self.library_tarball.finish()?;
            let data_tar_gz = self.library_tarball.into_inner()?.finish()?;
            header.set_path("data.tar.gz")?;
            header.set_size(data_tar_gz.len() as u64);
            header.set_cksum();
            tar.append::<&[u8]>(&header, data_tar_gz.as_ref())?;

            let checksums_yaml_gz = checksums_yaml_gz(&metadata_gz, &data_tar_gz)?;
            header.set_path("checksums.yaml.gz")?;
            header.set_size(checksums_yaml_gz.len() as u64);
            header.set_cksum();
            tar.append::<&[u8]>(&header, checksums_yaml_gz.as_ref())?;

            tar.finish()?;
        }
        Ok((
            format!(
                "{}-{}-{}.gem",
                project.spec.package.name,
                // ?
                project.version.to_string().replace('-', "."),
                ruby_platform(os, cpu)
            ),
            gem_tar,
        ))
    }
}

pub(crate) fn write_gems(
    project: &Project,
    gem_path: &Path,
    gem_config: &TargetGem,
) -> io::Result<Vec<GeneratedAsset>> {
    let mut assets = vec![];
    for platform_dir in &project.platform_directories {
        let mut gem = Gem::new();
        assert!(!platform_dir.loadable_files.is_empty());
        let loadable_name = platform_dir.loadable_files[0].file.name.clone();
        let entrypoint = &platform_dir.loadable_files[0].file_stem;

        gem.write_library_file(
            format!("lib/{}", loadable_name).as_str(),
            platform_dir.loadable_files[0].file.data.as_ref(),
        )?;

        gem.write_library_file(
            format!("lib/{}.rb", project.spec.package.name.replace('-', "_")).as_str(),
            templates::lib_rb(&project.version, entrypoint, &gem_config.module_name).as_bytes(),
        )?;
        let (gem_name, data) = gem.complete(&platform_dir.os, &platform_dir.cpu, project)?;
        assets.push(GeneratedAsset::from(
            GeneratedAssetKind::Gem((platform_dir.os.clone(), platform_dir.cpu.clone())),
            &gem_path.join(gem_name),
            &data,
        )?);
    }
    Ok(assets)
}

mod templates {
    use semver::Version;

    pub(crate) fn lib_rb(version: &Version, entrypoint: &str, module_name: &str) -> String {
        format!(
            r#"
module {module_name}
  class Error < StandardError; end
  VERSION = "{version}"
  def self.loadable_path
    File.expand_path('{entrypoint}', File.dirname(__FILE__))
  end
  def self.load(db)
    db.load_extension(self.loadable_path)
  end
end

"#
        )
    }
}
