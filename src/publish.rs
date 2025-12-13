use crate::cli::PublishCommand;
use std::fmt;

#[derive(Debug)]
pub enum PublishError {
    NotImplemented,
}

impl fmt::Display for PublishError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PublishError::NotImplemented => write!(f, "Publish not yet implemented"),
        }
    }
}

pub fn publish(_args: PublishCommand) -> Result<(), PublishError> {
    todo!("Publish command not yet implemented");
}
