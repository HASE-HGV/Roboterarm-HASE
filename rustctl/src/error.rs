use std::error::Error;
use std::fmt;

#[allow(dead_code)]
#[derive(Debug)]
enum AppError {
    BadNumber(std::num::ParseIntError),
    BadValue,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadNumber(e) => write!(f, "bad number: {e}"),
            Self::BadValue => write!(f, "expected 1 or 2"),
        }
    }
}

impl Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::BadNumber(e) => Some(e),
            Self::BadValue => None,
        }
    }
}

impl From<std::num::ParseIntError> for AppError {
    fn from(e: std::num::ParseIntError) -> Self {
        Self::BadNumber(e)
    }
}
