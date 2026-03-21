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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub import: Option<String>,
    // for TypeScript, .d.ts file?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub types: Option<String>,
    // default export condition, for non-JS files (e.g. native binaries)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,
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

use crate::build::{
    create_targz, Cpu, GeneratedAsset, GeneratedAssetKind, Os, PlatformDirectory, PlatformFile,
    Project,
};

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
    let npm_platform_directories: Vec<&PlatformDirectory> = project
        .platform_directories
        .iter()
        .filter(|platform_dir| {
            matches!(platform_dir.os, Os::Linux | Os::Macos | Os::Windows)
                && matches!(platform_dir.cpu, Cpu::X86_64 | Cpu::Aarch64)
        })
        .collect();
    let entrypoint = &npm_platform_directories
        .first()
        .unwrap()
        .loadable_files
        .first()
        .unwrap()
        .file_stem;

    let platform_pkgs: Vec<PackageJson> = npm_platform_directories
        .iter()
        .map(|platform_dir| {
            let npm_os = match platform_dir.os {
                Os::Linux => "linux",
                Os::Macos => "darwin",
                Os::Windows => "win32",
                _ => unreachable!(
                    "Invalid npm OS {:?}, should be filtered from above.",
                    platform_dir.os
                ),
            };
            let npm_pkg_os_name = match platform_dir.os {
                Os::Linux => "linux",
                Os::Macos => "darwin",
                Os::Windows => "windows",
                _ => unreachable!(
                    "Invalid npm OS {:?}, should be filtered from above.",
                    platform_dir.os
                ),
            };
            let npm_cpu = match platform_dir.cpu {
                Cpu::X86_64 => "x64",
                Cpu::Aarch64 => "arm64",
                _ => unreachable!(
                    "Invalid npm CPU {:?} should be filtered from above.",
                    platform_dir.cpu
                ),
            };
            PackageJson {
                name: format!(
                    "{pkg}-{os}-{cpu}",
                    pkg = project.spec.package.name,
                    os = npm_pkg_os_name,
                    cpu = npm_cpu
                ),
                version: project.version.to_string(),
                author: author.clone(),
                license: project.spec.package.license.clone(),
                description: project.spec.package.description.clone(),
                repository: Repository {
                    repo_type: "git".to_owned(),
                    url: project.spec.package.repo.clone(),
                    directory: None,
                },
                main: None,
                module: None,
                types: None,
                exports: HashMap::from_iter(
                    platform_dir.loadable_files.iter().map(|f| {
                        let path = format!("./{}", f.file.name);
                        (
                            path.clone(),
                            ExportTarget {
                                require: None,
                                import: None,
                                types: None,
                                default: Some(path),
                            },
                        )
                    }),
                ),
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
        .zip(&npm_platform_directories)
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
            url: project.spec.package.repo.clone(),
            directory: None,
        },
        main: Some("./index.cjs".to_owned()),
        module: Some("./index.mjs".to_owned()),
        types: Some("./index.d.ts".to_owned()),
        exports: HashMap::from([(
            ".".to_owned(),
            ExportTarget {
                require: Some("./index.cjs".to_owned()),
                import: Some("./index.mjs".to_owned()),
                types: Some("./index.d.ts".to_owned()),
                default: None,
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

    let platforms = npm_platform_directories
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
                url: project.spec.package.repo.clone(),
                directory: None,
            },
            main: None,
            module: Some("./sqlite3.mjs".to_owned()),
            types: None,
            exports: HashMap::from([(
                ".".to_owned(),
                ExportTarget {
                    require: None,
                    import: Some("./sqlite3.mjs".to_owned()),
                    types: None,
                    default: None,
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
    use crate::build::{Cpu, Os};

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
            .map(|(os, cpu)| {
                let npm_os = match os {
                    Os::Macos => "darwin",
                    Os::Linux => "linux",
                    Os::Windows => "win32",
                    _ => unreachable!("Unsupported npm OS {:?}", os),
                };
                let npm_cpu = match cpu {
                    Cpu::X86_64 => "x64",
                    Cpu::Aarch64 => "arm64",
                    _ => unreachable!("Unsupported npm CPU {:?}", cpu),
                };
                vec![npm_os.to_owned(), npm_cpu.to_owned()]
            })
            .collect();
        let supported_platforms = serde_json::to_string(&supported_platforms)
            .expect("String values should always serialize as JSON");

        let imports = match format {
            JsFormat::CJS => {
                r#"
const { arch, platform } = require("node:process");
"#
            }
            JsFormat::ESM => {
                r#"
import { fileURLToPath } from "node:url";
import { arch, platform } from "node:process";
"#
            }
        };

        let exports = match format {
            JsFormat::CJS => r#"module.exports = {getLoadablePath, load};"#,
            JsFormat::ESM => r#"export {getLoadablePath, load};"#,
        };
        let resolve_loadable = match format {
            JsFormat::CJS => {
                r#"require.resolve(packageName + "/" + ENTRYPOINT_BASE_NAME + "." + extensionSuffix(platform))"#
            }
            JsFormat::ESM => {
                r#"fileURLToPath(import.meta.resolve(packageName + "/" + ENTRYPOINT_BASE_NAME + "." + extensionSuffix(platform)))"#
            }
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
    supportedPlatforms.find(([p, a]) => platform === p && arch === a) !== undefined
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
  const loadablePath = {resolve_loadable};
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
