use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = env!("CARGO_PKG_NAME"),
    version = env!("CARGO_PKG_VERSION"),
    author = "Alex Garcia",
    about = "Package and distribute pre-compiled SQLite extensions"
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Parser, Debug)]
pub enum Command {
    /// Build SQLite distribution packages
    Build(BuildCommand),
    /// Publish distribution packages
    Publish(PublishCommand),
}

#[derive(Parser, Debug)]
pub struct BuildCommand {
    /// Sets the input file
    #[arg(value_name = "FILE")]
    pub file: PathBuf,

    /// Sets the input directory
    #[arg(long, value_name = "INPUT_DIR")]
    pub input: Option<PathBuf>,

    /// Sets the output directory
    #[arg(long, value_name = "OUTPUT_DIR")]
    pub output: Option<PathBuf>,

    /// Set the version
    #[arg(long, value_name = "VERSION", required = true)]
    pub set_version: String,
}

#[derive(Parser, Debug)]
pub struct PublishCommand {
    // Placeholder for publish-specific flags
}

impl Args {
    pub fn parse_args() -> Self {
        <Self as clap::Parser>::parse()
    }
}
