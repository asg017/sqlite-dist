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
    /// Set the version
    #[arg(long, value_name = "VERSION", required = true)]
    pub set_version: String,

    /// Change to directory before building
    #[arg(long, value_name = "DIR", short = 'C')]
    pub chdir: Option<PathBuf>,
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
