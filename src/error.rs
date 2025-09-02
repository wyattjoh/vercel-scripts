use thiserror::Error;

/// Custom error type for VSS to distinguish between user interruptions and actual errors
#[derive(Error, Debug)]
pub enum VssError {
    /// User interrupted operation (CTRL-C)
    #[error("Interrupted by user")]
    UserInterrupted,

    /// Other errors that should be displayed to the user
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<inquire::InquireError> for VssError {
    fn from(error: inquire::InquireError) -> Self {
        match error {
            inquire::InquireError::OperationInterrupted => VssError::UserInterrupted,
            other => VssError::Other(other.into()),
        }
    }
}

/// Result type alias for VSS operations
pub type VssResult<T> = Result<T, VssError>;
