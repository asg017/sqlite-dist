pub(crate) mod targets;
pub(crate) mod cli;
pub(crate) mod spec;
pub(crate) mod build;

use build::build;
use cli::Command;

fn main() {
    let args = cli::Args::parse_args();

    match args.command {
        Command::Build(build_cmd) => {
            match build(build_cmd) {
                Ok(_) => std::process::exit(0),
                Err(error) => {
                    eprintln!("Build error: {error}");
                    std::process::exit(1);
                }
            }
        }
        Command::Publish(_publish_cmd) => {
            todo!("Publish command not yet implemented");
        }
    }
}
