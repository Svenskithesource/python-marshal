use core::panic;
use std::{
    collections::{HashMap, HashSet},
    io::{Cursor, Read},
    sync::Arc,
};

use num_bigint::{BigInt, BigUint};
use num_complex::Complex;
use num_traits::FromPrimitive;

use crate::{error::Error, Code, Code310, CodeFlags, Kind, Object, ObjectHashable, PyVersion};

pub struct PyReader {
    cursor: Cursor<Vec<u8>>,
    pub references: HashMap<usize, Arc<Object>>,
    version: PyVersion,
}

#[macro_export]
macro_rules! extract_object {
    ($self:expr, $variant:pat => $binding:ident, $err:expr) => {
        match $self.ok_or_else(|| $err) {
            Ok(val) => match val {
                $variant => Ok($binding),
                x => Err(crate::error::Error::InvalidObject(x)),
            },
            Err(e) => Err(e),
        }
    };
}

#[macro_export]
macro_rules! resolve_object_ref {
    // Gets the object from the reference table by index
    ($self:expr, $refs:expr) => {
        match $self.ok_or_else(|| crate::error::Error::UnexpectedNull) {
            Ok(val) => match val {
                Object::LoadRef(index) | Object::StoreRef(index) => {
                    let reference = $refs.get(&index);

                    match reference {
                        Some(obj) => Ok((**obj).clone()),
                        None => Err(crate::error::Error::InvalidReference),
                    }
                }
                x => Ok(x),
            },
            Err(e) => Err(e),
        }
    };
}

#[macro_export]
macro_rules! extract_strings_tuple {
    ($objs:expr, $refs:expr) => {
        $objs
            .iter()
            .map(|o| match resolve_object_ref!(Some(o.clone()), $refs)? {
                Object::String(string) => Ok(string.clone()),
                _ => Err(Error::UnexpectedObject),
            })
            .collect::<Result<Vec<_>, _>>()
    };
}

#[macro_export]
macro_rules! extract_strings_list {
    ($objs:expr) => {
        $objs
            .iter()
            .map(|o| match o.as_ref() {
                Object::String(string) => Ok(string.clone()),
                _ => Err(Error::UnexpectedObject),
            })
            .collect::<Result<Vec<_>, _>>()
    };
}

#[macro_export]
macro_rules! extract_strings_set {
    ($objs:expr) => {
        $objs
            .iter()
            .map(|o| match o {
                ObjectHashable::String(string) => Ok(string.clone()),
                _ => Err(Error::UnexpectedObject),
            })
            .collect::<Result<HashSet<_>, _>>()
    };
}

#[macro_export]
macro_rules! extract_strings_frozenset {
    ($objs:expr) => {
        $objs
            .iter()
            .map(|o| match o {
                ObjectHashable::String(string) => Ok(string.clone()),
                _ => Err(Error::UnexpectedObject),
            })
            .collect::<Result<HashSet<_>, _>>()
    };
}

#[macro_export]
macro_rules! extract_strings_dict {
    ($objs:expr) => {
        $objs
            .iter()
            .map(|(k, v)| match (k, v.as_ref()) {
                (ObjectHashable::String(key), Object::String(value)) => {
                    Ok((key.clone(), value.clone()))
                }
                _ => Err(Error::UnexpectedObject),
            })
            .collect::<Result<HashMap<_, _>, _>>()
    };
}

impl PyReader {
    pub fn new(data: Vec<u8>, version: PyVersion) -> Self {
        Self {
            cursor: Cursor::new(data),
            version,
            references: HashMap::new(),
        }
    }

    fn r_u8(&mut self) -> Result<u8, std::io::Error> {
        let mut buf = [0; 1];
        self.cursor.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    fn r_u16(&mut self) -> Result<u16, std::io::Error> {
        let mut buf = [0; 2];
        self.cursor.read_exact(&mut buf)?;
        let value = u16::from_le_bytes(buf);
        Ok(value)
    }

    fn r_long(&mut self) -> Result<i32, std::io::Error> {
        let mut buf = [0; 4];
        self.cursor.read_exact(&mut buf)?;
        let value = i32::from_le_bytes(buf);
        Ok(value)
    }

    fn r_long64(&mut self) -> Result<i64, std::io::Error> {
        let mut buf = [0; 8];
        self.cursor.read_exact(&mut buf)?;
        let value = i64::from_le_bytes(buf);
        Ok(value)
    }

    fn r_bytes(&mut self, length: usize) -> Result<Vec<u8>, std::io::Error> {
        let mut buf = vec![0; length];
        self.cursor.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn r_string(&mut self, length: usize) -> Result<String, Error> {
        let bytes = self.r_bytes(length)?;
        let string = String::from_utf8(bytes)?;
        Ok(string)
    }

    fn r_float_str(&mut self) -> Result<f64, Error> {
        let n = self.r_u8()?;
        let s = self.r_string(n as usize)?;
        Ok(s.parse()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?)
    }

    fn r_float_bin(&mut self) -> Result<f64, std::io::Error> {
        let mut buf = [0; 8];
        self.cursor.read_exact(&mut buf)?;
        let value = f64::from_le_bytes(buf);
        Ok(value)
    }

    fn r_vec(&mut self, length: usize, kind: Kind) -> Result<Vec<Object>, Error> {
        let mut vec = Vec::with_capacity(length);

        for _ in 0..length {
            let obj = self.r_object()?;

            if obj.is_none() {
                return Err(match kind {
                    Kind::Tuple => Error::NullInTuple,
                    Kind::List => Error::NullInList,
                    Kind::Set => Error::NullInSet,
                    _ => Error::InvalidKind(kind),
                });
            }

            vec.push(obj.unwrap());
        }

        Ok(vec)
    }

    fn r_hashmap(&mut self) -> Result<HashMap<ObjectHashable, Object>, Error> {
        let mut map = HashMap::new();

        loop {
            match self.r_object()? {
                None => break,
                Some(key) => match self.r_object()? {
                    None => break,
                    Some(value) => {
                        map.insert(ObjectHashable::try_from(key)?, value);
                    }
                },
            }
        }

        Ok(map)
    }

    fn set_reference(&mut self, index: usize, obj: Arc<Object>) {
        self.references.insert(index, obj);
    }

    fn r_object(&mut self) -> Result<Option<Object>, Error> {
        let code = self.r_u8()?;

        let flag = (code & Kind::FlagRef as u8) != 0;

        let obj_kind = Kind::from_u8(code & !(Kind::FlagRef as u8)).unwrap();

        let mut idx: Option<usize> = match obj_kind {
            Kind::SmallTuple
            | Kind::Tuple
            | Kind::List
            | Kind::Dict
            | Kind::Set
            | Kind::FrozenSet
            | Kind::Code
                if flag =>
            {
                let i = self.references.len();
                self.set_reference(i, Object::None.into());
                Some(i)
            }
            _ => None,
        };

        let obj = match obj_kind {
            Kind::Null => None,
            Kind::None => Some(Object::None),
            Kind::Ellipsis => Some(Object::Ellipsis),
            Kind::False => Some(Object::Bool(false)),
            Kind::True => Some(Object::Bool(true)),
            Kind::Int => {
                let value = Object::Long(BigInt::from(self.r_long()?).into());

                Some(value)
            }
            Kind::Int64 => {
                let value = Object::Long(BigInt::from(self.r_long64()?).into());

                Some(value)
            }
            Kind::Long => {
                let n = self.r_long()?;
                let number = {
                    let size = n.wrapping_abs() as usize;
                    let mut value = BigUint::ZERO;

                    for i in 0..size {
                        let digit = self.r_u16()?;

                        if digit > (1 << 15) {
                            return Err(Error::DigitOutOfRange(digit));
                        }

                        value |= BigUint::from(digit) << (i * 15);
                    }

                    value
                };

                let signed = BigInt::from_biguint(
                    match n.cmp(&0) {
                        std::cmp::Ordering::Less => num_bigint::Sign::Minus,
                        std::cmp::Ordering::Equal => num_bigint::Sign::NoSign,
                        std::cmp::Ordering::Greater => num_bigint::Sign::Plus,
                    },
                    number,
                );

                let value = Object::Long(signed.into());

                Some(value)
            }
            Kind::Float => {
                let value = Object::Float(self.r_float_str()?);

                Some(value)
            }
            Kind::BinaryFloat => {
                let value = Object::Float(self.r_float_bin()?);

                Some(value)
            }
            Kind::Complex => {
                let real = self.r_float_str()?;
                let imag = self.r_float_str()?;
                let value = Object::Complex(Complex::new(real, imag));

                Some(value)
            }
            Kind::BinaryComplex => {
                let real = self.r_float_bin()?;
                let imag = self.r_float_bin()?;
                let value = Object::Complex(Complex::new(real, imag));

                Some(value)
            }
            Kind::String => {
                let length = self.r_long()?;
                let value = Object::Bytes(self.r_bytes(length as usize)?.into());

                Some(value)
            }
            Kind::ASCIIInterned | Kind::ASCII | Kind::Interned | Kind::Unicode => {
                let length = self.r_long()?;
                let value = Object::String(Arc::new(self.r_string(length as usize)?));

                Some(value)
            }
            Kind::ShortAsciiInterned | Kind::ShortAscii => {
                let length = self.r_u8()?;
                let value = Object::String(Arc::new(self.r_string(length as usize)?));

                Some(value)
            }
            Kind::Tuple => {
                let length = self.r_long()?;
                let value = Object::Tuple(self.r_vec(length as usize, Kind::Tuple)?.into());

                Some(value)
            }
            Kind::SmallTuple => {
                let length = self.r_u8()?;
                let value = Object::Tuple(self.r_vec(length as usize, Kind::Tuple)?.into());

                Some(value)
            }
            Kind::List => {
                let length = self.r_long()?;
                let value = Object::List(
                    self.r_vec(length as usize, Kind::List)?
                        .into_iter()
                        .map(|o| o.into())
                        .collect::<Vec<_>>()
                        .into(),
                );

                Some(value)
            }
            Kind::Dict => {
                let value = Object::Dict(
                    self.r_hashmap()?
                        .into_iter()
                        .map(|(k, v)| (k, v.into()))
                        .collect::<HashMap<_, _>>()
                        .into(),
                );

                Some(value)
            }
            Kind::Set => {
                let length = self.r_long()?;
                let value = self
                    .r_vec(length as usize, Kind::Set)?
                    .into_iter()
                    .map(|o| match ObjectHashable::try_from(o) {
                        Ok(obj) => Ok(obj),
                        Err(_) => Err(Error::UnexpectedObject),
                    })
                    .collect::<Result<HashSet<_>, _>>()?
                    .into();

                if flag {
                    idx = Some(self.references.len());
                    self.set_reference(idx.unwrap(), Object::Set(Arc::clone(&value)).into());
                }

                Some(Object::Set(value))
            }
            Kind::FrozenSet => {
                let length = self.r_long()?;
                let value = Object::FrozenSet(
                    self.r_vec(length as usize, Kind::FrozenSet)?
                        .into_iter()
                        .map(|o| match ObjectHashable::try_from(o) {
                            Ok(obj) => Ok(obj),
                            Err(_) => Err(Error::UnexpectedObject),
                        })
                        .collect::<Result<HashSet<_>, _>>()?
                        .into(),
                )
                .into();

                Some(value)
            }
            Kind::Code => {
                let value = match self.version {
                    PyVersion {
                        major: 3,
                        minor: 10,
                    } => {
                        let argcount = self.r_long()?;
                        let posonlyargcount = self.r_long()?;
                        let kwonlyargcount = self.r_long()?;
                        let nlocals = self.r_long()?;
                        let stacksize = self.r_long()?;
                        let flags = CodeFlags::from_bits_truncate(self.r_long()? as u32);
                        let code = extract_object!(Some(resolve_object_ref!(self.r_object()?, self.references)?), Object::Bytes(bytes) => bytes, Error::NullInTuple)?;
                        let consts = extract_object!(Some(resolve_object_ref!(self.r_object()?, self.references)?), Object::Tuple(objs) => objs, Error::NullInTuple)?;
                        let names = extract_strings_tuple!(
                            extract_object!(Some(resolve_object_ref!(self.r_object()?, self.references)?), Object::Tuple(objs) => objs, Error::NullInTuple)?,
                            self.references
                        )?;

                        let varnames = extract_strings_tuple!(
                            extract_object!(Some(resolve_object_ref!(self.r_object()?, self.references)?), Object::Tuple(objs) => objs, Error::NullInTuple)?,
                            self.references
                        )?;
                        let freevars = extract_strings_tuple!(
                            extract_object!(Some(resolve_object_ref!(self.r_object()?, self.references)?), Object::Tuple(objs) => objs, Error::NullInTuple)?,
                            self.references
                        )?;
                        let cellvars = extract_strings_tuple!(
                            extract_object!(Some(resolve_object_ref!(self.r_object()?, self.references)?), Object::Tuple(objs) => objs, Error::NullInTuple)?,
                            self.references
                        )?;
                        let filename = extract_object!(Some(resolve_object_ref!(self.r_object()?, self.references)?), Object::String(string) => string, Error::UnexpectedObject)?;
                        let name = extract_object!(Some(resolve_object_ref!(self.r_object()?, self.references)?), Object::String(string) => string, Error::UnexpectedObject)?;
                        let firstlineno = self.r_long()?;
                        let lnotab = extract_object!(Some(resolve_object_ref!(self.r_object()?, self.references)?), Object::Bytes(bytes) => bytes, Error::NullInTuple)?;

                        Object::Code(Arc::new(Code::V310(Code310 {
                            argcount: argcount.try_into().unwrap(),
                            posonlyargcount: posonlyargcount.try_into().unwrap(),
                            kwonlyargcount: kwonlyargcount.try_into().unwrap(),
                            nlocals: nlocals.try_into().unwrap(),
                            stacksize: stacksize.try_into().unwrap(),
                            flags,
                            code,
                            consts,
                            names,
                            varnames,
                            freevars,
                            cellvars,
                            filename,
                            name,
                            firstlineno: firstlineno.try_into().unwrap(),
                            lnotab,
                        })))
                    }
                    _ => {
                        panic!("Unsupported version: {:?}", self.version);
                    }
                };

                Some(value)
            }
            Kind::Ref => {
                let index = self.r_long()? as usize;

                let reference = self.references.get(&index);

                match reference {
                    Some(_) => Some(Object::LoadRef(index)),
                    None => return Err(Error::InvalidReference),
                }
            }
            Kind::Unknown => return Err(Error::InvalidKind(obj_kind)),
            Kind::StopIteration | Kind::FlagRef => todo!(),
        };

        match (&obj, idx) {
            (None, _)
            | (Some(Object::None), _)
            | (Some(Object::StopIteration), _)
            | (Some(Object::Ellipsis), _)
            | (Some(Object::Bool(_)), _)
            | (Some(Object::LoadRef(_)), _) => {}
            (Some(x), Some(i)) if flag => {
                idx = Some(i);
                self.set_reference(i, x.clone().into());
            }
            (Some(x), None) if flag => {
                idx = Some(self.references.len());
                self.set_reference(idx.unwrap(), x.clone().into());
            }
            (Some(_), _) => {}
        };

        match flag {
            true => Ok(Some(Object::StoreRef(idx.unwrap()))),
            false => Ok(obj),
        }
    }

    pub fn read_object(&mut self) -> Result<Object, Error> {
        if self.cursor.position() == self.cursor.get_ref().len() as u64 {
            panic!("EOF, don't know what to do");
        }

        let object = self.r_object()?;

        Ok(object.unwrap())
    }
}
