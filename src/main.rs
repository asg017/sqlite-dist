pub(crate) mod targets;
pub(crate) mod cli;
pub(crate) mod manifest;
pub(crate) mod spec;
pub(crate) mod build;

use build::build;
fn main() {
    let args = cli::Args::parse_args();

    match build(args) {
        Ok(_) => std::process::exit(0),
        Err(error) => {
            eprintln!("Build error: {error}");
            std::process::exit(1);
        }
    }
}
