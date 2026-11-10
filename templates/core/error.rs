#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    InvalidInput(&'static str),
    NotFound,
    Storage,
    SchemaTooNew,
    Unavailable,
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidInput(message) => message,
            Self::NotFound => "This note no longer exists.",
            Self::Storage => "Local storage could not complete this operation.",
            Self::SchemaTooNew => "This database needs a newer version of the app.",
            Self::Unavailable => "The notes session is closed or unavailable.",
        })
    }
}

impl std::error::Error for AppError {}
