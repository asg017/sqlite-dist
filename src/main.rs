pub(crate) mod targets;
pub(crate) mod cli;
pub(crate) mod spec;
pub(crate) mod build;
pub(crate) mod publish;

use build::build;
use publish::publish;
use cli::Command;

fn main() {
    let args = cli::Args::parse_args();

    let result = match args.command {
        Command::Build(build_cmd) => {
            build(build_cmd).map_err(|e| format!("Build error: {e}"))
        }
        Command::Publish(publish_cmd) => {
            publish(publish_cmd).map_err(|e| format!("Publish error: {e}"))
        }
    };

    match result {
        Ok(_) => std::process::exit(0),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
