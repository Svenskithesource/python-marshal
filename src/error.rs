use std::{
    fmt::{Display, Formatter, Result},
    string::FromUtf16Error,
};

use crate::{magic::PyVersion, Kind, Object};

#[derive(Debug)]
#[allow(dead_code)]
pub enum Error {
    UnsupportedPyVersion(PyVersion),
    UnsupportedMagicNumber(u32),
    DigitOutOfRange(u16),
    UnnormalizedLong,
    NullInTuple,
    NullInList,
    NullInSet,
    NullInDict,
    InvalidKind(Kind),
    InvalidObject(Object),
    InvalidData(std::io::Error),
    InvalidString,
    InvalidUtf16String(std::string::FromUtf16Error),
    UnexpectedObject,
    InvalidReference,
    UnexpectedNull,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            Error::UnsupportedPyVersion(vers) => write!(
                f,
                "unsupported Python version: {}.{}",
                vers.major, vers.minor
            ),
            Error::UnsupportedMagicNumber(magic) => {
                write!(f, "unsupported magic number: 0x{:08X}", magic)
            }
            Error::DigitOutOfRange(digit) => write!(
                f,
                "bad marshal data (digit out of range in long): {}",
                digit
            ),
            Error::UnnormalizedLong => write!(f, "bad marshal data (unnormalized long data)"),
            Error::NullInTuple => write!(f, "NULL object in marshal data for tuple"),
            Error::NullInList => write!(f, "NULL object in marshal data for list"),
            Error::NullInSet => write!(f, "NULL object in marshal data for set"),
            Error::NullInDict => write!(f, "NULL object in marshal data for dict"),
            Error::InvalidKind(kind) => write!(f, "invalid kind: {:?}", kind),
            Error::InvalidObject(obj) => write!(f, "invalid object: {:?}", obj),
            Error::InvalidData(err) => write!(f, "bad marshal data: {:?}", err),
            Error::InvalidString => {
                write!(f, "bad marshal data (invalid string)")
            }
            Error::InvalidUtf16String(err) => {
                write!(f, "bad marshal data (invalid utf16 string): {:?}", err)
            }
            Error::UnexpectedObject => write!(f, "unexpected object"),
            Error::InvalidReference => write!(f, "bad marshal data (invalid reference)"),
            Error::UnexpectedNull => write!(f, "unexpected NULL object"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::InvalidData(err)
    }
}

impl From<std::string::FromUtf16Error> for Error {
    fn from(err: FromUtf16Error) -> Self {
        Error::InvalidUtf16String(err)
    }
}
