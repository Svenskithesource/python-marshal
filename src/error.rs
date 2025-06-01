use std::{
    fmt::{Display, Formatter, Result},
    string::FromUtf16Error,
};

use crate::{magic::PyVersion, Kind, Object};

/// Represents errors that can occur while reading or writing Python marshal data.
#[derive(Debug)]
#[allow(dead_code)]
pub enum Error {
    UnsupportedPyVersion(PyVersion),
    NoMagicNumber,
    NoTimeStamp,
    NoHash,
    UnsupportedMagicNumber(u32),
    DigitOutOfRange(u16),
    UnnormalizedLong,
    NullInTuple,
    NullInList,
    NullInSet,
    NullInDict,
    UnreadableKind,
    InvalidConversion,
    InvalidKind(Kind),
    InvalidObject(Object),
    InvalidData(std::io::Error),
    InvalidString,
    InvalidUtf16String(std::string::FromUtf16Error),
    InvalidReference(usize),
    InvalidStoreRef,
    UnexpectedObject,
    UnexpectedNull,
    DepthLimitExceeded,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            Error::UnsupportedPyVersion(vers) => write!(
                f,
                "unsupported Python version: {}.{}",
                vers.major, vers.minor
            ),
            Error::NoMagicNumber => write!(f, "no magic number found"),
            Error::NoTimeStamp => write!(f, "no timestamp found"),
            Error::NoHash => write!(f, "no hash found"),
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
            Error::UnreadableKind => write!(f, "bad marshal data (unreadable kind)"),
            Error::InvalidConversion => write!(f, "bad marshal data (invalid conversion)"),
            Error::InvalidKind(kind) => write!(f, "invalid kind: {:?}", kind),
            Error::InvalidObject(obj) => write!(f, "invalid object: {:?}", obj),
            Error::InvalidData(err) => write!(f, "bad marshal data: {:?}", err),
            Error::InvalidString => {
                write!(f, "bad marshal data (invalid string)")
            }
            Error::InvalidUtf16String(err) => {
                write!(f, "bad marshal data (invalid utf16 string): {:?}", err)
            }
            Error::InvalidReference(index) => {
                write!(f, "bad marshal data (invalid reference index: {})", index)
            }
            Error::InvalidStoreRef => write!(f, "bad marshal data (invalid store reference)"),
            Error::DepthLimitExceeded => write!(f, "depth limit exceeded while processing object"),
            Error::UnexpectedObject => write!(f, "unexpected object"),
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
