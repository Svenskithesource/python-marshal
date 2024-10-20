use std::fmt::{Display, Formatter, Result};

use crate::{Kind, Object};

#[derive(Debug)]
#[allow(dead_code)]
pub enum Error {
    DigitOutOfRange(u16),
    UnnormalizedLong,
    NullInTuple,
    NullInList,
    NullInSet,
    NullInDict,
    InvalidKind(Kind),
    InvalidObject(Object),
    UnexpectedObject,
    InvalidReference,
    UnexpectedNull,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
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
            Error::UnexpectedObject => write!(f, "unexpected object"),
            Error::InvalidReference => write!(f, "bad marshal data (invalid reference)"),
            Error::UnexpectedNull => write!(f, "unexpected NULL object"),
        }
    }
}

impl std::error::Error for Error {}
