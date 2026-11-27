use std::error::Error;
use std::fmt;
use std::io::{self, BufRead};
use std::net::{AddrParseError, IpAddr};
use std::num::ParseIntError;

pub(crate) struct Data {
    pub ip: String,
    pub mode: u8,
}

#[derive(Debug)]
pub(crate) enum AppError {
    Io(io::Error),
    InvalidIp(AddrParseError),
    InvalidMode(ParseIntError),
    InvalidRange,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::InvalidIp(err) => write!(f, "invalid IP address: {err}"),
            Self::InvalidMode(err) => write!(f, "invalid mode input: {err}"),
            Self::InvalidRange => write!(f, "invalid mode: expected 1 (raw) or 2 (ik)"),
        }
    }
}

impl Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::InvalidIp(err) => Some(err),
            Self::InvalidMode(err) => Some(err),
            Self::InvalidRange => None,
        }
    }
}

impl From<io::Error> for AppError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<AddrParseError> for AppError {
    fn from(err: AddrParseError) -> Self {
        Self::InvalidIp(err)
    }
}

impl From<ParseIntError> for AppError {
    fn from(err: ParseIntError) -> Self {
        Self::InvalidMode(err)
    }
}

pub(crate) fn read_input<R: BufRead>(reader: &mut R) -> Result<String, AppError> {
    let mut s = String::new();
    let bytes_read = reader.read_line(&mut s)?;
    if bytes_read == 0 {
        return Err(AppError::Io(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "unexpected end of input",
        )));
    }

    Ok(s.trim_end_matches(['\r', '\n']).to_string())
}

pub(crate) fn input() -> Result<String, AppError> {
    let mut stdin = io::stdin().lock();
    read_input(&mut stdin)
}

pub(crate) fn gap() {
    println!();
}

pub(crate) fn parse_ip(ip_input: &str) -> Result<String, AppError> {
    let ip = ip_input.trim();
    ip.parse::<IpAddr>()?;
    Ok(ip.to_string())
}

pub(crate) fn parse_mode(mode_input: &str) -> Result<u8, AppError> {
    let mode: u8 = mode_input.trim().parse()?;
    if mode != 1 && mode != 2 {
        return Err(AppError::InvalidRange);
    }

    Ok(mode)
}

pub(crate) fn pull_connection_data() -> Result<Data, AppError> {
    println!("please enter IP-address");
    gap();
    let ip = parse_ip(&input()?)?;

    println!("please enter the mode you wish to use");
    gap();
    let mode = parse_mode(&input()?)?;

    Ok(Data { ip, mode })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn read_input_trims_crlf_and_lf() {
        let mut cursor = Cursor::new(b"127.0.0.1\r\n");
        assert_eq!(read_input(&mut cursor).unwrap(), "127.0.0.1");

        let mut cursor = Cursor::new(b"127.0.0.1\n");
        assert_eq!(read_input(&mut cursor).unwrap(), "127.0.0.1");
    }

    #[test]
    fn read_input_returns_error_on_eof() {
        let mut cursor = Cursor::new(b"");
        let err = read_input(&mut cursor).unwrap_err();
        assert!(matches!(err, AppError::Io(ref io_err) if io_err.kind() == io::ErrorKind::UnexpectedEof));
    }

    #[test]
    fn parse_ip_validates_addresses() {
        assert_eq!(parse_ip(" 192.168.1.42 ").unwrap(), "192.168.1.42");
        assert_eq!(parse_ip("::1").unwrap(), "::1");
        assert!(matches!(parse_ip(""), Err(AppError::InvalidIp(_))));
        assert!(matches!(parse_ip("999.0.0.1"), Err(AppError::InvalidIp(_))));
    }

    #[test]
    fn parse_mode_rejects_non_numbers_and_out_of_range() {
        assert_eq!(parse_mode("1").unwrap(), 1);
        assert_eq!(parse_mode(" 2 ").unwrap(), 2);
        assert!(matches!(parse_mode(""), Err(AppError::InvalidMode(_))));
        assert!(matches!(parse_mode("abc"), Err(AppError::InvalidMode(_))));
        assert!(matches!(parse_mode("256"), Err(AppError::InvalidMode(_))));
        assert!(matches!(parse_mode("0"), Err(AppError::InvalidRange)));
        assert!(matches!(parse_mode("3"), Err(AppError::InvalidRange)));
    }
}
