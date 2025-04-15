use crate::{Cpu, GeneratedAsset, GeneratedAssetKind, Os, Project};
use serde::{Deserialize, Serialize};
use std::{
    io::{self, Cursor, Write},
    path::Path,
};
use zip::{write::FileOptions, ZipWriter};

#[derive(Debug, Deserialize, Serialize)]
pub struct Nuspec {
    pub metadata: Metadata,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Metadata {
    pub id: String,
    pub version: String,
    pub title: String,
    pub authors: String,
    pub owners: String,
    pub require_license_acceptance: bool,
    pub description: String,
    pub files: Option<Vec<String>>,
}

impl Nuspec {
    fn new(project: &Project) -> Self {
        let author = project.spec.package.authors.first().unwrap();
        Self {
            metadata: Metadata {
                id: project.spec.package.name.clone(),
                version: project.version.to_string(),
                title: project.spec.package.name.clone(),
                authors: author.clone(),
                owners: author.clone(),
                require_license_acceptance: false,
                description: project.spec.package.description.clone(),
                files: None,
            },
        }
    }

    fn to_xml(&self) -> String {
        let files_xml = if let Some(files) = &self.metadata.files {
            files
                .iter()
                .map(|file| format!("<file src=\"{}\" target=\"{}\" />", file, file))
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            String::new()
        };

        format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://schemas.microsoft.com/packaging/2010/07/nuspec.xsd">
  <metadata>
    <id>{}</id>
    <version>{}</version>
    <title>{}</title>
    <authors>{}</authors>
    <owners>{}</owners>
    <requireLicenseAcceptance>{}</requireLicenseAcceptance>
    <description>{}</description>
  </metadata>
  <files>
    {}
  </files>
</package>"#,
            self.metadata.id,
            self.metadata.version,
            self.metadata.title,
            self.metadata.authors,
            self.metadata.owners,
            self.metadata.require_license_acceptance,
            self.metadata.description,
            files_xml
        )
    }
}

pub struct NugetPackage {
    zip: ZipWriter<Cursor<Vec<u8>>>,
}

impl NugetPackage {
    pub fn new(package_id: &str) -> io::Result<Self> {
        let buffer = Vec::new();
        let cursor = Cursor::new(buffer);
        let mut zip = ZipWriter::new(cursor);
        zip.start_file("[Content_Types].xml", FileOptions::default())?;
        zip.write_all(Self::content_types().as_bytes())?;
        // add props_file to the folder buildTransitive\net46
        let props_file_name = format!("build/netstandard2.0/{}.props", package_id);
        zip.start_file(&props_file_name, FileOptions::default())?;
        zip.write_all(Self::props_file().as_bytes())?;

        // add targets_file to the folder buildTransitive\net46
        let targets_file_name = format!("build/netstandard2.0/{}.targets", package_id);
        zip.start_file(&targets_file_name, FileOptions::default())?;
        zip.write_all(Self::targets_file(package_id).as_bytes())?;
        Ok(Self { zip })
    }

    fn content_types() -> String {
        r#"<?xml version="1.0" encoding="utf-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Default Extension="nuspec" ContentType="application/octet-stream"/>
  <Default Extension="psmdcp" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>
</Types>"#.to_string()
    }

    fn props_file() -> String {
        r#"<Project xmlns="http://schemas.microsoft.com/developer/msbuild/2003">

  <!--
  NuGet packages.config doesn't support native assemblies automatically,
  so copy the native assemblies to the output directory.
  -->
  <ItemGroup Condition="Exists('packages.config') OR
                        Exists('$(MSBuildProjectName).packages.config') OR
                        Exists('packages.$(MSBuildProjectName).config')">
    <Content Include="$(MSBuildThisFileDirectory)\..\..\runtimes\win-x64\native\*.dll"
             Condition="'$(PlatformTarget)' == 'x64'">
      <CopyToOutputDirectory>PreserveNewest</CopyToOutputDirectory>
      <Visible>false</Visible>
      <Link>%(Filename)%(Extension)</Link>
    </Content>

    <Content Include="$(MSBuildThisFileDirectory)\..\..\runtimes\win-x86\native\*.dll"
             Condition="'$(PlatformTarget)' == 'x86'">
      <CopyToOutputDirectory>PreserveNewest</CopyToOutputDirectory>
      <Visible>false</Visible>
      <Link>%(Filename)%(Extension)</Link>
    </Content>

    <Content Include="$(MSBuildThisFileDirectory)\..\..\runtimes\win-arm64\native\*.dll"
             Condition="'$(PlatformTarget)' == 'arm64'">
      <CopyToOutputDirectory>PreserveNewest</CopyToOutputDirectory>
      <Visible>false</Visible>
      <Link>%(Filename)%(Extension)</Link>
    </Content>
  </ItemGroup>

</Project>"#
            .to_string()
    }

    fn targets_file(package_id: &str) -> String {
        format!(
            r#"<Project xmlns="http://schemas.microsoft.com/developer/msbuild/2003">

  <PropertyGroup>
    <EnableUnsupportedPlatformTargetCheck Condition="'$(EnableUnsupportedPlatformTargetCheck)' == ''">true</EnableUnsupportedPlatformTargetCheck>
  </PropertyGroup>

  <Target Name="_CheckForUnsupportedPlatformTarget"
          Condition="'$(EnableUnsupportedPlatformTargetCheck)' == 'true'"
          AfterTargets="_CheckForInvalidConfigurationAndPlatform">

    <!--
    Special case .NET Core portable applications.  When building a portable .NET Core app,
    the PlatformTarget is empty, and you don't know until runtime (i.e. which dotnet.exe)
    what processor architecture will be used.
    -->
    <Error Condition="('$(PlatformTarget)' != 'x64' AND '$(PlatformTarget)' != 'x86') AND
                      ('$(OutputType)' == 'Exe' OR '$(OutputType)'=='WinExe') AND
                      !('$(TargetFrameworkIdentifier)' == '.NETCoreApp' AND '$(PlatformTarget)' == '')"
           Text="{} currently supports 'x64' and 'x86' processor architectures. Please ensure your application is targeting 'x64' or 'x86'." />
  </Target>
  
</Project>"#,
            package_id
        )
    }

    pub fn add_nuspec(&mut self, nuspec: &Nuspec) -> io::Result<()> {
        let nuspec_file_name = format!("{}.nuspec", nuspec.metadata.id);
        let nuspec_content = nuspec.to_xml();
        self.zip
            .start_file(&nuspec_file_name, FileOptions::default())?;
        self.zip.write_all(nuspec_content.as_bytes())?;
        Ok(())
    }

    pub fn add_files(&mut self, project: &Project) -> io::Result<()> {
        for platform_dir in &project.platform_directories {
            if !(matches!(platform_dir.os, Os::Linux | Os::Macos | Os::Windows)
                && matches!(platform_dir.cpu, Cpu::X86_64 | Cpu::Aarch64))
            {
                continue;
            }
            for loadable_file in &platform_dir.loadable_files {
                let os = match platform_dir.os {
                    Os::Windows => "win",
                    Os::Linux => "linux",
                    Os::Macos => "osx",
                    Os::Android => "android",
                    Os::Ios => "ios",
                    Os::IosSimulator => "ios-simulator",
                };
                let cpu = match platform_dir.cpu {
                    Cpu::Aarch64 => "arm64",
                    Cpu::X86_64 => "x64",
                    Cpu::I686 => "x86",
                    Cpu::Armv7a => "armv7",
                };
                let file_path =
                    format!("runtimes/{}-{}/native/{}", os, cpu, loadable_file.file.name);
                self.zip.start_file(&file_path, FileOptions::default())?;
                self.zip.write_all(&loadable_file.file.data)?;
            }
        }
        Ok(())
    }

    pub fn add_cs_file(&mut self, project: &Project) -> io::Result<()> {
        let package_name = project.spec.targets.nuget.as_ref().map_or_else(
            || project.spec.package.name.replace('-', "_"),
            |nuget| nuget.friendly_name.clone(),
        );
        let cs_file_content = format!(
            r#"// <auto-generated/>

namespace Microsoft.Data.Sqlite
{{
    public static class Sqlite{0}Extensions
    {{
        public static void Load{0}(this SqliteConnection connection)
          => connection.LoadExtension("{1}");
    }}
}}"#,
            package_name,
            project.platform_directories[0].loadable_files[0]
                .file
                .name
                .replace(".dll", "")
                .replace(".so", "")
                .replace(".dylib", "")
        );
        let cs_file_path = format!(
            "contentFiles/cs/netstandard2.0/{}/{}.cs",
            package_name, package_name
        );
        self.zip.start_file(&cs_file_path, FileOptions::default())?;
        self.zip.write_all(cs_file_content.as_bytes())?;
        Ok(())
    }

    fn update_nuspec_with_files(&self, nuspec: &mut Nuspec) {
        nuspec.metadata.files = Some(vec![format!(
            "<file src=\"*.cs\" target=\"contentFiles/cs/netstandard2.0/{}\"/>",
            nuspec.metadata.id
        )]);
    }

    pub fn finish(mut self) -> io::Result<Vec<u8>> {
        let result = self.zip.finish()?;
        Ok(result.into_inner())
    }
}

pub(crate) fn write_nuget_packages(
    project: &Project,
    nuget_output_directory: &Path,
) -> io::Result<Vec<GeneratedAsset>> {
    let mut assets = vec![];
    let mut nuspec = Nuspec::new(project);
    let mut package = NugetPackage::new(&nuspec.metadata.id)?;
    package.add_nuspec(&nuspec)?;
    package.add_files(project)?;
    package.add_cs_file(project)?;
    package.update_nuspec_with_files(&mut nuspec);
    let buffer = package.finish()?;
    let output_path = nuget_output_directory.join(format!(
        "{}.{}.nupkg",
        nuspec.metadata.id, nuspec.metadata.version
    ));
    std::fs::write(&output_path, &buffer)?;
    assets.push(GeneratedAsset::from(
        GeneratedAssetKind::Nuget,
        &output_path,
        &buffer,
    )?);
    Ok(assets)
}
