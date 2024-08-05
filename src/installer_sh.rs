pub(crate) mod templates {
    use std::collections::HashSet;

    use crate::{Cpu, GeneratedAsset, GeneratedAssetKind, Os, Project};

    struct Case {
        os: Os,
        cpu: Cpu,
        type_: String,
        url: String,
        checksum: String,
    }

    pub(crate) fn install_sh(project: &Project, assets: &[GeneratedAsset]) -> String {
        let mut targets = assets
            .iter()
            .filter_map(|asset| match &asset.kind {
                GeneratedAssetKind::GithubReleaseLoadable(gh_release)
                | GeneratedAssetKind::GithubReleaseStatic(gh_release) => Some(format!(
                    "{}-{}",
                    gh_release.platform.0.to_string(),
                    gh_release.platform.1.to_string()
                )),
                _ => None,
            })
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<String>>();
        targets.sort();
        let targets = targets.join(", ");

        let cases: Vec<_> = assets
            .iter()
            .filter_map(|asset| match &asset.kind {
                GeneratedAssetKind::GithubReleaseLoadable(gh_release) => Some(Case {
                    os: gh_release.platform.0.clone(),
                    cpu: gh_release.platform.1.clone(),
                    type_: "loadable".to_owned(),
                    url: gh_release.url.to_string(),
                    checksum: asset.checksum_sha256.clone(),
                }),
                GeneratedAssetKind::GithubReleaseStatic(gh_release) => Some(Case {
                    os: gh_release.platform.0.clone(),
                    cpu: gh_release.platform.1.clone(),
                    type_: "static".to_owned(),
                    url: gh_release.url.to_string(),
                    checksum: asset.checksum_sha256.clone(),
                }),
                _ => None,
            })
            .collect();

        let usage = part_usage(project.version.to_string().as_str(), &targets);
        let current_target = part_current_target();
        let process_arguments = part_process_arguments();
        let main = part_main(cases);
        format!(
            r#"#!/bin/sh
set -e

if [ -n "$NO_COLOR" ]; then
    BOLD=""
    RESET=""
else
    BOLD="\033[1m"
    RESET="\033[0m"
fi

{usage}

{current_target}

{process_arguments}

{main}

main "$@"
"#
        )
    }

    fn part_usage(version: &str, targets: &str) -> String {
        // TODO: build commit, build date, project name NOT hello
        format!(
            r#"
usage() {{
    cat <<EOF
sqlite-hello-install {version}

USAGE:
    $0 [static|loadable] [--target=target] [--prefix=path]

OPTIONS:
    --target
            Specify a different target platform to install. Available targets: {targets}

    --prefix
            Specify a different directory to save the binaries. Defaults to the current working directory.
EOF
}}

"#
        )
    }
    fn part_current_target() -> String {
        r#"
current_target() {
  if [ "$OS" = "Windows_NT" ]; then
    # TODO disambiguate between x86 and arm windows
    target="windows-x86_64"
    return 0
  fi
  case $(uname -sm) in
  "Darwin x86_64") target=macos-x86_64 ;;
  "Darwin arm64") target=macos-aarch64 ;;
  "Linux x86_64") target=linux-x86_64 ;;
  *) target=$(uname -sm);;
  esac
}
"#
        .to_owned()
    }
    fn part_process_arguments() -> String {
        (r#"
process_arguments() {
  while [[ $# -gt 0 ]]; do
      case "$1" in
          --help)
              usage
              exit 0
              ;;
          --target=*)
              target="\${1#*=}"
              ;;
          --prefix=*)
              prefix="\${1#*=}"
              ;;
          static|loadable)
              type="$1"
              ;;
          *)
              echo "Unrecognized option: $1"
              usage
              exit 1
              ;;
      esac
      shift
  done
  if [ -z "$type" ]; then
    type=loadable
  fi
  if [ "$type" != "static" ] && [ "$type" != "loadable" ]; then
      echo "Invalid type '$type'. It must be either 'static' or 'loadable'."
      usage
      exit 1
  fi
  if [ -z "$prefix" ]; then
    prefix="$PWD"
  fi
  if [ -z "$target" ]; then
    current_target
  fi
}

"#)
        .to_owned()
    }

    fn case(case: &Case) -> String {
        format!(
            r#"    "{os}-{cpu}-{t}")
      url="{url}"
      checksum="{checksum}"
      ;;"#,
            os = case.os.to_string(),
            cpu = case.cpu.to_string(),
            t = case.type_,
            url = case.url,
            checksum = case.checksum
        )
    }
    fn part_main(cases: Vec<Case>) -> String {
        let cases = cases.iter().map(case).collect::<Vec<_>>().join("\n");
        format!(
            r#"
main() {{
    local type=""
    local target=""
    local prefix=""
    local url=""
    local checksum=""

    process_arguments "$@"

    echo "${{BOLD}}Type${{RESET}}: $type"
    echo "${{BOLD}}Target${{RESET}}: $target"
    echo "${{BOLD}}Prefix${{RESET}}: $prefix"

    case "$target-$type" in
{cases}
    *)
      echo "Unsupported platform $target" 1>&2
      exit 1
      ;;
    esac

    extension="\${{url##*.}}"

    if [ "$extension" = "zip" ]; then
      tmpfile="$prefix/tmp.zip"
    else
      tmpfile="$prefix/tmp.tar.gz"
    fi

    curl --fail --location --progress-bar --output "$tmpfile" "$url"

    if ! echo "$checksum $tmpfile" | sha256sum --check --status; then
      echo "Checksum fail!"  1>&2
      rm $tmpfile
      exit 1
    fi

    if [ "$extension" = "zip" ]; then
      unzip "$tmpfile" -d $prefix
      rm $tmpfile
    else
      tar -xzf "$tmpfile" -C $prefix
      rm $tmpfile
    fi

    echo "✅ $target $type binaries installed at $prefix."
}}

"#
        )
    }
}
