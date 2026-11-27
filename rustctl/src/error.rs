use std::error::Error;
use std::fmt::self;

#[allow(dead_code)]
#[derive(Debug)]
enum AppError {
    BadNumber(std::num::ParseIntError),
    BadValue,
    Impossible,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadNumber(e) => write!(f, "bad number: {e}"),
            Self::BadValue => write!(f, "expected 1 or 2"),
            Self::Impossible => write!(f, "If you read this, you did something that should be impossible, \n congrats"),
        }
    }
}

impl Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::BadNumber(e) => Some(e),
            Self::BadValue => None,
            Self::Impossible => None,
        }
    }
}

impl From<std::num::ParseIntError> for AppError {
    fn from(e: std::num::ParseIntError) -> Self {
        Self::BadNumber(e)
    }
}
