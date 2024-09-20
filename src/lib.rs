mod error;
mod reader;

use bitflags::bitflags;
use hashable::HashableHashSet;
use num_bigint::BigInt;
use num_complex::Complex;
use num_derive::{FromPrimitive, ToPrimitive};
use ordered_float::OrderedFloat;
use reader::PyReader;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
};

type PyVersion = (u8, u8);

#[derive(Debug, Clone, Copy, FromPrimitive, ToPrimitive)]
#[repr(u8)]
#[rustfmt::skip]
pub enum Kind {
    Null               = b'0',
    None               = b'N',
    False              = b'F',
    True               = b'T',
    StopIteration      = b'S',
    Ellipsis           = b'.',
    Int                = b'i',
    Int64              = b'I',
    Float              = b'f',
    BinaryFloat        = b'g',
    Complex            = b'x',
    BinaryComplex      = b'y',
    Long               = b'l',
    String             = b's',
    Interned           = b't',
    Ref                = b'r',
    Tuple              = b'(',
    List               = b'[',
    Dict               = b'{',
    Code               = b'c',
    Unicode            = b'u',
    Unknown            = b'?',
    Set                = b'<',
    Frozenset          = b'>',
    ASCII              = b'a',
    ASCIIInterned      = b'A',
    SmallTuple         = b')',
    ShortAscii         = b'z',
    ShortAsciiInterned = b'Z',
    FlagRef            = 0x80,
}

bitflags! {
    #[derive(Clone, Debug)]
    pub struct CodeFlags: u32 {
        const OPTIMIZED                   = 0x1;
        const NEWLOCALS                   = 0x2;
        const VARARGS                     = 0x4;
        const VARKEYWORDS                 = 0x8;
        const NESTED                     = 0x10;
        const GENERATOR                  = 0x20;

        const NOFREE                     = 0x40; // Removed in 3.10

        const COROUTINE                  = 0x80;
        const ITERABLE_COROUTINE        = 0x100;
        const ASYNC_GENERATOR           = 0x200;

        const GENERATOR_ALLOWED        = 0x1000;

        const FUTURE_DIVISION          = 0x2000;
        const FUTURE_ABSOLUTE_IMPORT   = 0x4000;
        const FUTURE_WITH_STATEMENT    = 0x8000;
        const FUTURE_PRINT_FUNCTION   = 0x10000;
        const FUTURE_UNICODE_LITERALS = 0x20000;

        const FUTURE_BARRY_AS_BDFL    = 0x40000;
        const FUTURE_GENERATOR_STOP   = 0x80000;
        const FUTURE_ANNOTATIONS     = 0x100000;

        const NO_MONITORING_EVENTS    = 0x200000; // Added in 3.13
    }
}

#[rustfmt::skip]
#[derive(Clone, Debug)]
pub struct Code310 {
    pub argcount:        u32,
    pub posonlyargcount: u32,
    pub kwonlyargcount:  u32,
    pub nlocals:         u32,
    pub stacksize:       u32,
    pub flags:           CodeFlags,
    pub code:            Arc<Vec<u8>>,
    pub consts:          Arc<Vec<Object>>,
    pub names:           Vec<Arc<String>>,
    pub varnames:        Vec<Arc<String>>,
    pub freevars:        Vec<Arc<String>>,
    pub cellvars:        Vec<Arc<String>>,
    pub filename:        Arc<String>,
    pub name:            Arc<String>,
    pub firstlineno:     u32,
    pub lnotab:          Arc<Vec<u8>>,
}

#[derive(Clone, Debug)]
pub enum Code {
    V310(Code310),
}

#[rustfmt::skip]
#[derive(Clone, Debug)]
pub enum Object {
    None,
    StopIteration,
    Ellipsis,
    Bool      (bool),
    Long      (Arc<BigInt>),
    Float     (f64),
    Complex   (Complex<f64>),
    Bytes     (Arc<Vec<u8>>),
    String    (Arc<String>),
    Tuple     (Arc<Vec<Object>>),
    List      (Arc<RwLock<Vec<Object>>>),
    Dict      (Arc<RwLock<HashMap<ObjectHashable, Object>>>),
    Set       (Arc<RwLock<HashSet<ObjectHashable>>>),
    FrozenSet (Arc<HashSet<ObjectHashable>>),
    Code      (Arc<Code>),
}

#[rustfmt::skip]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ObjectHashable {
    None,
    StopIteration,
    Ellipsis,
    Bool      (bool),
    Long      (Arc<BigInt>),
    Float     (OrderedFloat<f64>),
    Complex   (Complex<OrderedFloat<f64>>),
    Bytes     (Arc<Vec<u8>>),
    String    (Arc<String>),
    Tuple     (Arc<Vec<ObjectHashable>>),
    FrozenSet (Arc<HashableHashSet<ObjectHashable>>),
}

impl From<Object> for ObjectHashable {
    fn from(obj: Object) -> Self {
        match obj {
            Object::None => ObjectHashable::None,
            Object::StopIteration => ObjectHashable::StopIteration,
            Object::Ellipsis => ObjectHashable::Ellipsis,
            Object::Bool(b) => ObjectHashable::Bool(b),
            Object::Long(i) => ObjectHashable::Long(i),
            Object::Float(f) => ObjectHashable::Float(f.into()),
            Object::Complex(c) => ObjectHashable::Complex(Complex {
                re: OrderedFloat(c.re),
                im: OrderedFloat(c.im),
            }),
            Object::Bytes(b) => ObjectHashable::Bytes(b),
            Object::String(s) => ObjectHashable::String(s),
            Object::Tuple(t) => ObjectHashable::Tuple(
                t.iter()
                    .map(|o| ObjectHashable::from(o.clone()))
                    .collect::<Vec<_>>()
                    .into(),
            ),
            Object::FrozenSet(s) => {
                ObjectHashable::FrozenSet(s.iter().cloned().collect::<HashableHashSet<_>>().into())
            }
            _ => panic!("unhashable type"),
        }
    }
}

pub fn load_bytes(data: &[u8], python_version: PyVersion) -> anyhow::Result<Object> {
    if python_version < (3, 0) {
        return Err(anyhow::anyhow!("Python 2.x is not supported"));
    }

    let mut py_reader = PyReader::new(data.to_vec(), python_version);

    let object = py_reader.read_object()?;

    Ok(object)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_bytes() {
        let data = b"\xe3\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\x02\x00\x00\x00C\x00\x00\x00s\x0c\x00\x00\x00t\x00|\x00\x83\x01\x01\x00d\x00S\x00)\x01N)\x01\xda\x05print)\x01\xda\x04name\xa9\x00r\x03\x00\x00\x00\xfa\x07<stdin>\xda\x01f\x01\x00\x00\x00s\x02\x00\x00\x00\x0c\x00";
        let kind = load_bytes(data, (3, 10)).unwrap();
        dbg!(kind);
    }
}
