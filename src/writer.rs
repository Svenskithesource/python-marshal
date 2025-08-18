use bstr::BString;
use num_bigint::BigInt;
use num_complex::Complex;
use num_traits::{Signed, ToPrimitive};

use crate::{error::Error, Code, Kind, Object};

/// Macro to write Code31x objects (Python 3.11, 3.12, 3.13) which share the same structure
macro_rules! w_code311 {
    ($self:ident, $value:ident, $is_ref:ident) => {
        // https://github.com/python/cpython/blob/3.11/Python/marshal.c#L558
        $self.w_kind(Kind::Code, $is_ref);
        $self.w_long(
            $value
                .argcount
                .try_into()
                .map_err(|_| Error::InvalidConversion)?,
        );
        $self.w_long(
            $value
                .posonlyargcount
                .try_into()
                .map_err(|_| Error::InvalidConversion)?,
        );
        $self.w_long(
            $value
                .kwonlyargcount
                .try_into()
                .map_err(|_| Error::InvalidConversion)?,
        );
        $self.w_long(
            $value
                .stacksize
                .try_into()
                .map_err(|_| Error::InvalidConversion)?,
        );
        $self.w_long(
            $value
                .flags
                .bits()
                .try_into()
                .map_err(|_| Error::InvalidConversion)?,
        );
        $self.w_object(Some((*$value.code).clone()), false)?;
        $self.w_object(Some((*$value.consts).clone()), false)?;
        $self.w_object(Some((*$value.names).clone()), false)?;
        $self.w_object(Some((*$value.localsplusnames).clone()), false)?;
        $self.w_object(Some((*$value.localspluskinds).clone()), false)?;
        $self.w_object(Some((*$value.filename).clone()), false)?;
        $self.w_object(Some((*$value.name).clone()), false)?;
        $self.w_object(Some((*$value.qualname).clone()), false)?;
        $self.w_long(
            $value
                .firstlineno
                .try_into()
                .map_err(|_| Error::InvalidConversion)?,
        );
        $self.w_object(Some((*$value.linetable).clone()), false)?;
        $self.w_object(Some((*$value.exceptiontable).clone()), false)?;
    };
}

/// On windows this is 1000.
/// See https://github.com/python/cpython/blob/3.10/Python/marshal.c#L36
#[cfg(windows)]
static MAX_DEPTH: usize = 1000;

/// See https://github.com/python/cpython/blob/3.10/Python/marshal.c#L38
#[cfg(not(windows))]
static MAX_DEPTH: usize = 2000;

/// A writer for Python objects that serializes them into a binary format
pub struct PyWriter {
    data: Vec<u8>,
    marshal_version: u8,
    references: Vec<Object>,
    /// The current depth of the object being written.
    depth: usize,
}

impl PyWriter {
    pub fn new(references: Vec<Object>, marshal_version: u8) -> Self {
        Self {
            data: Vec::new(),
            marshal_version,
            references,
            depth: 0,
        }
    }

    fn w_u8(&mut self, value: u8) {
        self.data.push(value);
    }

    fn w_u16(&mut self, value: u16) {
        self.data.extend_from_slice(&value.to_le_bytes());
    }

    fn w_long(&mut self, value: i32) {
        self.data.extend_from_slice(&value.to_le_bytes());
    }

    fn w_kind(&mut self, kind: Kind, is_ref: bool) {
        match is_ref {
            true => self.w_u8(kind as u8 | Kind::FlagRef as u8),
            false => self.w_u8(kind as u8),
        }
    }

    #[allow(non_snake_case)]
    fn w_PyLong(&mut self, num: BigInt) -> Result<(), Error> {
        let mut value = num.clone().abs();
        let mut digits: Vec<u16> = vec![];

        while value > BigInt::ZERO {
            digits.push(
                (&value & BigInt::from(0x7FFF))
                    .to_u16()
                    .ok_or(Error::InvalidConversion)?,
            );

            value >>= 15;
        }

        self.w_long((digits.len() as i32) * if num.is_negative() { -1 } else { 1 });

        for digit in digits {
            self.w_u16(digit);
        }

        Ok(())
    }

    fn w_string(&mut self, value: &BString, as_u8: bool) {
        if as_u8 {
            self.w_u8(value.len() as u8);
        } else {
            self.w_long(value.len() as i32);
        }

        self.data
            .extend_from_slice(&value.iter().copied().collect::<Vec<u8>>());
    }

    fn w_float_bin(&mut self, value: f64) {
        self.data.extend_from_slice(&value.to_le_bytes());
    }

    fn w_float_str(&mut self, value: f64) {
        self.w_string(&value.to_string().into(), true);
    }

    fn w_bytes(&mut self, value: &[u8]) {
        self.data.extend_from_slice(value);
    }

    fn w_object(&mut self, obj: Option<Object>, is_ref: bool) -> Result<(), Error> {
        self.depth += 1;

        if self.depth > MAX_DEPTH {
            return Err(Error::DepthLimitExceeded);
        }

        match obj {
            None => self.w_kind(Kind::Null, is_ref),
            Some(Object::None) => self.w_kind(Kind::None, is_ref),
            Some(Object::StopIteration) => self.w_kind(Kind::StopIteration, is_ref),
            Some(Object::Ellipsis) => self.w_kind(Kind::Ellipsis, is_ref),
            Some(Object::Bool(value)) => {
                self.w_kind(
                    {
                        if value {
                            Kind::True
                        } else {
                            Kind::False
                        }
                    },
                    is_ref,
                );
            }
            Some(Object::Long(num)) => {
                let num = num.clone();
                if num >= BigInt::from(i32::MIN) && num <= BigInt::from(i32::MAX) {
                    self.w_kind(Kind::Int, is_ref);
                    self.w_long(num.to_i32().ok_or(Error::InvalidConversion)?);
                } else {
                    self.w_kind(Kind::Long, is_ref);
                    self.w_PyLong(num)?;
                }
            }
            Some(Object::Float(value)) => {
                if self.marshal_version > 1 {
                    self.w_kind(Kind::BinaryFloat, is_ref);
                    self.w_float_bin(value.into_inner());
                } else {
                    self.w_kind(Kind::Float, is_ref);
                    self.w_float_str(value.into_inner());
                }
            }
            Some(Object::Complex(Complex { re, im })) => {
                if self.marshal_version > 1 {
                    self.w_kind(Kind::BinaryComplex, is_ref);
                    self.w_float_bin(re.into_inner());
                    self.w_float_bin(im.into_inner());
                } else {
                    self.w_kind(Kind::Complex, is_ref);
                    self.w_float_str(re.into_inner());
                    self.w_float_str(im.into_inner());
                }
            }
            Some(Object::Bytes(value)) => {
                self.w_kind(Kind::String, is_ref);
                self.w_long(value.len() as i32);
                self.w_bytes(&value);
            }
            Some(Object::String(value)) => {
                let str_value = &value.value;

                match value.kind {
                    Kind::ASCII | Kind::ASCIIInterned | Kind::Interned => {
                        self.w_kind(value.kind, is_ref);
                        self.w_long(str_value.len() as i32);
                        self.w_bytes(&str_value.iter().copied().collect::<Vec<u8>>());
                    }
                    Kind::ShortAscii | Kind::ShortAsciiInterned => {
                        self.w_kind(value.kind, is_ref);
                        self.w_u8(str_value.len() as u8);
                        self.w_bytes(&str_value.iter().copied().collect::<Vec<u8>>());
                    }
                    Kind::Unicode => {
                        self.w_kind(Kind::Unicode, is_ref);
                        self.w_string(str_value, false);
                    }
                    _ => {
                        panic!("Invalid string kind: {:?}", value.kind);
                    }
                }
            }
            Some(Object::Tuple(value)) => {
                let size = value.len();

                if self.marshal_version >= 4 && size <= 255 {
                    self.w_kind(Kind::SmallTuple, is_ref);
                    self.w_u8(size as u8);
                } else {
                    self.w_kind(Kind::Tuple, is_ref);
                    self.w_long(size as i32);
                }

                for item in value.iter() {
                    self.w_object(Some((*item).clone()), false)?;
                }
            }
            Some(Object::List(value)) => {
                let size = value.len();

                self.w_kind(Kind::List, is_ref);
                self.w_long(size as i32);

                for item in value.iter() {
                    self.w_object(Some(item.clone().clone()), false)?;
                }
            }
            Some(Object::Dict(value)) => {
                self.w_kind(Kind::Dict, is_ref);
                for (key, value) in value.iter() {
                    self.w_object(Some((*key).clone().into()), false)?;
                    self.w_object(Some((*value).clone()), false)?;
                }

                self.w_kind(Kind::Null, is_ref); // NULL object terminated
            }
            Some(Object::Set(value)) => {
                let size = value.len();

                self.w_kind(Kind::Set, is_ref);
                self.w_long(size as i32);

                for item in value.iter() {
                    self.w_object(Some((*item).clone().into()), false)?;
                }
            }
            Some(Object::FrozenSet(value)) => {
                let size = value.len();

                self.w_kind(Kind::FrozenSet, is_ref);
                self.w_long(size as i32);

                for item in value.iter() {
                    self.w_object(Some((*item).clone().into()), false)?;
                }
            }
            Some(Object::Code(value)) => {
                let value = value;

                match value {
                    Code::V310(value) => {
                        // https://github.com/python/cpython/blob/3.10/Python/marshal.c#L511
                        self.w_kind(Kind::Code, is_ref);
                        self.w_long(
                            value
                                .argcount
                                .try_into()
                                .map_err(|_| Error::InvalidConversion)?,
                        );
                        self.w_long(
                            value
                                .posonlyargcount
                                .try_into()
                                .map_err(|_| Error::InvalidConversion)?,
                        );
                        self.w_long(
                            value
                                .kwonlyargcount
                                .try_into()
                                .map_err(|_| Error::InvalidConversion)?,
                        );
                        self.w_long(
                            value
                                .nlocals
                                .try_into()
                                .map_err(|_| Error::InvalidConversion)?,
                        );
                        self.w_long(
                            value
                                .stacksize
                                .try_into()
                                .map_err(|_| Error::InvalidConversion)?,
                        );
                        self.w_long(
                            value
                                .flags
                                .bits()
                                .try_into()
                                .map_err(|_| Error::InvalidConversion)?,
                        );
                        self.w_object(Some((*value.code).clone()), false)?;
                        self.w_object(Some((*value.consts).clone()), false)?;
                        self.w_object(Some((*value.names).clone()), false)?;
                        self.w_object(Some((*value.varnames).clone()), false)?;
                        self.w_object(Some((*value.freevars).clone()), false)?;
                        self.w_object(Some((*value.cellvars).clone()), false)?;
                        self.w_object(Some((*value.filename).clone()), false)?;
                        self.w_object(Some((*value.name).clone()), false)?;
                        self.w_long(
                            value
                                .firstlineno
                                .try_into()
                                .map_err(|_| Error::InvalidConversion)?,
                        );
                        self.w_object(Some((*value.linetable).clone()), false)?;
                    }
                    Code::V311(value) => {
                        w_code311!(self, value, is_ref);
                    }
                    Code::V312(value) => {
                        w_code311!(self, value, is_ref);
                    }
                    Code::V313(value) => {
                        w_code311!(self, value, is_ref);
                    }
                }
            }
            Some(Object::LoadRef(index)) => {
                let reference = self.references.get(index);

                match reference {
                    None => {
                        panic!("Reference {index} not found in references list");
                    }
                    Some(_) => {
                        self.w_kind(Kind::Ref, is_ref);
                        self.w_long(index as i32);
                    }
                }
            }
            Some(Object::StoreRef(index)) => {
                let reference = self.references.get(index);

                match reference {
                    None => {
                        return Err(Error::InvalidReference(index));
                    }
                    Some(reference) => {
                        self.w_object(Some((*reference).clone()), true)?;
                    }
                }
            }
        };

        self.depth -= 1;

        Ok(())
    }

    pub fn write_object(&mut self, obj: Option<Object>) -> Result<Vec<u8>, Error> {
        self.w_object(obj, false)?;

        Ok(self.data.clone())
    }
}
