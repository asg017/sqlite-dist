use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Repository {
    #[serde(rename = "type")]
    pub repo_type: String,
    pub url: String,
    pub directory: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ExportTarget {
    // for CJS, should end in .cjs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub require: Option<String>,
    // for ESM, should end in .mjs
    pub import: String,
    // for TypeScript, .d.ts file?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub types: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PackageJson {
    pub name: String,
    pub version: String,
    pub author: String,
    pub license: String,
    pub description: String,
    pub repository: Repository,

    // CJS file?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main: Option<String>,
    // ESM file?
    pub module: String,
    // path to .d.ts file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub types: Option<String>,

    pub exports: HashMap<String, ExportTarget>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<HashMap<String, String>>,

    #[serde(rename = "optionalDependencies")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional_dependencies: Option<HashMap<String, String>>,

    #[serde(rename = "devDependencies")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dev_dependencies: Option<HashMap<String, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<Vec<String>>,
}

use crate::{create_targz, Cpu, GeneratedAsset, GeneratedAssetKind, Os, PlatformFile, Project};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum NpmBuildError {
    #[error("I/O error: {0}")]
    IOError(#[from] io::Error),
    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),
}

struct NpmPlatformPackage {
    name: String,
    os: Os,
    cpu: Cpu,
    data: Vec<u8>,
}
pub(crate) fn write_npm_packages(
    project: &Project,
    npm_ouput_directory: &Path,
    emscripten_dir: &Option<PathBuf>,
) -> Result<Vec<GeneratedAsset>, NpmBuildError> {
    let mut assets = vec![];
    let author = project.spec.package.authors.first().unwrap();
    let entrypoint = &project
        .platform_directories
        .first()
        .unwrap()
        .loadable_files
        .first()
        .unwrap()
        .file_stem;

    let platform_pkgs: Vec<PackageJson> = project
        .platform_directories
        .iter()
        .map(|platform_dir| {
            let npm_os = match platform_dir.os {
                Os::Linux => "linux",
                Os::Macos => "darwin",
                Os::Windows => "windows",
            };
            let npm_cpu = match platform_dir.cpu {
                Cpu::X86_64 => "x64",
                Cpu::Aarch64 => "arm64",
            };
            PackageJson {
                name: format!(
                    "{pkg}-{os}-{cpu}",
                    pkg = project.spec.package.name,
                    os = npm_os,
                    cpu = npm_cpu
                ),
                version: project.version.to_string(),
                author: author.clone(),
                license: project.spec.package.license.clone(),
                description: project.spec.package.description.clone(),
                repository: Repository {
                    repo_type: "git".to_owned(),
                    url: "https://TODO".to_owned(),
                    directory: None,
                },
                main: Some("./index.cjs".to_owned()),
                module: "./index.mjs".to_owned(),
                types: Some("./index.d.ts".to_owned()),
                exports: HashMap::from([(
                    ".".to_owned(),
                    ExportTarget {
                        require: Some("./index.cjs".to_owned()),
                        import: "./index.mjs".to_owned(),
                        types: Some("./index.d.ts".to_owned()),
                    },
                )]),
                files: vec![].into(),
                keywords: vec![].into(),
                dependencies: None,
                optional_dependencies: None,
                dev_dependencies: None,
                os: Some(vec![npm_os.to_owned()]),
                cpu: Some(vec![npm_cpu.to_owned()]),
            }
        })
        .collect();

    let pkg_targzs: Result<Vec<NpmPlatformPackage>, NpmBuildError> = platform_pkgs
        .iter()
        .zip(&project.platform_directories)
        .map(|(pkg, platform_dir)| {
            let mut files = vec![
                PlatformFile::new("package/README.md", "TODO", None),
                PlatformFile::new("package/package.json", serde_json::to_string(&pkg)?, None),
            ];
            for loadable_file in &platform_dir.loadable_files {
                files.push(PlatformFile::new(
                    format!("package/{}", loadable_file.file.name),
                    loadable_file.file.data.clone(),
                    loadable_file.file.metadata.clone(),
                ));
            }

            Ok(NpmPlatformPackage {
                name: pkg.name.clone(),
                os: platform_dir.os.clone(),
                cpu: platform_dir.cpu.clone(),
                data: create_targz(&files.iter().collect::<Vec<&PlatformFile>>())?,
            })
        })
        .collect();
    let pkg_targzs = pkg_targzs?;

    let top_pkg = PackageJson {
        name: project.spec.package.name.clone(),
        version: project.version.to_string(),
        author: author.clone(),
        license: project.spec.package.license.clone(),
        description: project.spec.package.description.clone(),
        repository: Repository {
            repo_type: "git".to_owned(),
            url: "https://TODO".to_owned(),
            directory: None,
        },
        main: Some("./index.cjs".to_owned()),
        module: "./index.mjs".to_owned(),
        types: Some("./index.d.ts".to_owned()),
        exports: HashMap::from([(
            ".".to_owned(),
            ExportTarget {
                require: Some("./index.cjs".to_owned()),
                import: "./index.mjs".to_owned(),
                types: Some("./index.d.ts".to_owned()),
            },
        )]),
        files: vec![].into(),
        keywords: vec![].into(),
        dependencies: None,
        optional_dependencies: Some(HashMap::from_iter(
            platform_pkgs
                .iter()
                .map(|pkg| (pkg.name.clone(), pkg.version.clone())),
        )),
        dev_dependencies: None,
        os: None,
        cpu: None,
    };

    let platforms = project
        .platform_directories
        .iter()
        .map(|pd| (pd.os.clone(), pd.cpu.clone()))
        .collect::<Vec<(Os, Cpu)>>();
    let pkg_name = project.spec.package.name.clone();
    let top_pkg_targz_files = vec![
        PlatformFile::new("package/README.md", "TODO", None),
        PlatformFile::new(
            "package/package.json",
            serde_json::to_string(&top_pkg)?,
            None,
        ),
        PlatformFile::new(
            "package/index.mjs",
            templates::index_js(pkg_name.clone(), entrypoint, &platforms, JsFormat::ESM),
            None,
        ),
        PlatformFile::new(
            "package/index.cjs",
            templates::index_js(pkg_name.clone(), entrypoint, &platforms, JsFormat::CJS),
            None,
        ),
        PlatformFile::new("package/index.d.ts", templates::index_dts(), None),
    ];
    if let Some(emscripten_dir) = emscripten_dir {
        let wasm_pkg_json = PackageJson {
            name: format!("{}-wasm-demo", project.spec.package.name),
            version: project.version.to_string(),
            author: author.clone(),
            license: project.spec.package.license.clone(),
            description: project.spec.package.description.clone(),
            repository: Repository {
                repo_type: "git".to_owned(),
                url: "https://TODO".to_owned(),
                directory: None,
            },
            main: None,
            module: "./sqlite3.mjs".to_owned(),
            types: None,
            exports: HashMap::from([(
                ".".to_owned(),
                ExportTarget {
                    require: None,
                    import: "./sqlite3.mjs".to_owned(),
                    types: None,
                },
            )]),
            files: vec![].into(),
            keywords: vec![].into(),
            dependencies: None,
            optional_dependencies: None,
            dev_dependencies: None,
            os: None,
            cpu: None,
        };
        let wasm_pkg_targz_files = vec![
            PlatformFile::new("package/README.md", "TODO", None),
            PlatformFile::new(
                "package/package.json",
                serde_json::to_string(&wasm_pkg_json)?,
                None,
            ),
            PlatformFile::new(
                "package/sqlite3.mjs",
                fs::read(emscripten_dir.join("sqlite3.mjs"))?,
                None,
            ),
            PlatformFile::new(
                "package/sqlite3.wasm",
                fs::read(emscripten_dir.join("sqlite3.wasm"))?,
                None,
            ),
        ];
        let wasm_pkg_targz =
            create_targz(&wasm_pkg_targz_files.iter().collect::<Vec<&PlatformFile>>())?;
        assets.push(GeneratedAsset::from(
            GeneratedAssetKind::Npm(None),
            &npm_ouput_directory.join(format!("{}.tar.gz", wasm_pkg_json.name)),
            &wasm_pkg_targz,
        )?);
    }
    let top_pkg_targz = create_targz(&top_pkg_targz_files.iter().collect::<Vec<&PlatformFile>>());

    for pkg in pkg_targzs {
        assets.push(GeneratedAsset::from(
            GeneratedAssetKind::Npm(Some((pkg.os.clone(), pkg.cpu.clone()))),
            &npm_ouput_directory.join(format!("{}.tar.gz", pkg.name)),
            &pkg.data,
        )?);
    }
    assets.push(GeneratedAsset::from(
        GeneratedAssetKind::Npm(None),
        &npm_ouput_directory.join(format!("{}.tar.gz", top_pkg.name)),
        &top_pkg_targz?,
    )?);
    Ok(assets)
}

#[allow(clippy::upper_case_acronyms)]
enum JsFormat {
    CJS,
    ESM,
}
mod templates {
    use crate::{Cpu, Os};

    use super::JsFormat;
    pub(crate) fn index_dts() -> String {
        r#"

/**
 * TODO JSDoc
 */
export declare function getLoadablePath(): string;


interface Db {
    loadExtension(file: string, entrypoint?: string | undefined): void;
}

/**
 * TODO JSDoc
 */
export declare function load(db: Db): void;

"#
        .to_string()
    }
    pub(crate) fn index_js(
        pkg_name: String,
        entrypoint: &str,
        supported_platforms: &[(Os, Cpu)],
        format: JsFormat,
    ) -> String {
        let base_package_name = serde_json::to_string(&serde_json::Value::String(pkg_name.clone()))
            .expect("String value should always serialize as JSON");
        let entrypoint_base_name =
            serde_json::to_string(&serde_json::Value::String(entrypoint.to_owned()))
                .expect("String value should always serialize as JSON");

        let supported_platforms: Vec<Vec<String>> = supported_platforms
            .iter()
            .map(|(os, cpu)| vec![os.to_string(), cpu.to_string()])
            .collect();
        let supported_platforms = serde_json::to_string(&supported_platforms)
            .expect("String values should always serialize as JSON");

        let imports = match format {
            JsFormat::CJS => {
                r#"
const { join } = require("node:path");
const { fileURLToPath } = require("node:url");
const { arch, platform } = require("node:process");
const { statSync } = require("node:fs");
"#
            }
            JsFormat::ESM => {
                r#"
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { arch, platform } from "node:process";
import { statSync } from "node:fs";
"#
            }
        };

        let exports = match format {
            JsFormat::CJS => r#"module.exports = {getLoadablePath, load};"#,
            JsFormat::ESM => r#"export {getLoadablePath, load};"#,
        };

        format!(
            r#"
{imports}

const BASE_PACKAGE_NAME = {base_package_name};
const ENTRYPOINT_BASE_NAME = {entrypoint_base_name};
const supportedPlatforms = {supported_platforms};

const invalidPlatformErrorMessage = `Unsupported platform for ${{BASE_PACKAGE_NAME}}, on a ${{platform}}-${{arch}} machine. Supported platforms are (${{supportedPlatforms
  .map(([p, a]) => `${{p}}-${{a}}`)
  .join(",")}}). Consult the ${{BASE_PACKAGE_NAME}} NPM package README for details.`;

const extensionNotFoundErrorMessage = packageName => `Loadble extension for ${{BASE_PACKAGE_NAME}} not found. Was the ${{packageName}} package installed?`;

function validPlatform(platform, arch) {{
  return (
    supportedPlatforms.find(([p, a]) => platform == p && arch === a) !== null
  );
}}
function extensionSuffix(platform) {{
  if (platform === "win32") return "dll";
  if (platform === "darwin") return "dylib";
  return "so";
}}
function platformPackageName(platform, arch) {{
  const os = platform === "win32" ? "windows" : platform;
  return `${{BASE_PACKAGE_NAME}}-${{os}}-${{arch}}`;
}}

function getLoadablePath() {{
  if (!validPlatform(platform, arch)) {{
    throw new Error(
      invalidPlatformErrorMessage
    );
  }}
  const packageName = platformPackageName(platform, arch);
  const loadablePath = join(
    fileURLToPath(new URL(".", import.meta.url)),
    "..",
    packageName,
    `${{ENTRYPOINT_BASE_NAME}}.${{extensionSuffix(platform)}}`
  );
  if (!statSync(loadablePath, {{ throwIfNoEntry: false }})) {{
    throw new Error(extensionNotFoundErrorMessage(packageName));
  }}

  return loadablePath;
}}

function load(db) {{
  db.loadExtension(getLoadablePath());
}}

{exports}
"#
        )
    }
}
