use std::error::Error;
use std::fmt;

#[allow(dead_code)]
#[derive(Debug)]
enum AppError {
    BadNumber(std::num::ParseIntError),
    BadValue,
    Impossible,
    NoPrivilege,
    NoPins,
    Uncaught,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadNumber(e) => write!(f, "bad number: {e}"),
            Self::BadValue => write!(f, "expected 1 or 2"),
            Self::Impossible => write!(
                f,
                "If you read this, you did something that should be impossible, \n\n congrats"
            ),
            Self::NoPrivilege => write!(
                f,
                "You do currently not have access to the given feature in this environment"
            ),
            Self::NoPins => write!(f, "The current device does not have any GPIO pins"),
            Self::Uncaught => write!(f, "You used this program in ways no one saw coming"),
        }
    }
}

impl Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::BadNumber(e) => Some(e),
            Self::BadValue => None,
            Self::Impossible => None,
            Self::NoPrivilege => None,
            Self::NoPins => None,
            Self::Uncaught => None,
        }
    }
}

impl From<std::num::ParseIntError> for AppError {
    fn from(e: std::num::ParseIntError) -> Self {
        Self::BadNumber(e)
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        match e.kind() {
            std::io::ErrorKind::PermissionDenied => Self::NoPrivilege,
            std::io::ErrorKind::NotFound => Self::NoPins,
            _ => Self::Uncaught,
        }
    }
}
