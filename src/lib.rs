pub mod code_objects;
mod error;
pub mod magic;
mod optimizer;
mod reader;
pub mod resolver;
mod walker;
mod writer;

use bitflags::bitflags;
use bstr::BString;
use error::Error;
use hashable::HashableHashSet;
use indexmap::{IndexMap, IndexSet};
use magic::PyVersion;
use num_bigint::BigInt;
use num_complex::Complex;
use num_derive::{FromPrimitive, ToPrimitive};
use optimizer::{get_used_references, ReferenceOptimizer, Transformable};
use ordered_float::OrderedFloat;
use reader::PyReader;
use resolver::get_recursive_refs;
use std::io::{Read, Write};
use writer::PyWriter;

#[derive(Debug, Clone, Copy, FromPrimitive, ToPrimitive, PartialEq, Eq, Hash)]
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
    Int64              = b'I', // Only generated in version 0
    Float              = b'f', // Only generated in marshal version 0
    BinaryFloat        = b'g',
    Complex            = b'x', // Only generated in marshal version 0
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
    FrozenSet          = b'>',
    ASCII              = b'a',
    ASCIIInterned      = b'A',
    SmallTuple         = b')',
    ShortAscii         = b'z',
    ShortAsciiInterned = b'Z',
    FlagRef            = 0x80,
}

bitflags! {
    #[derive(Clone, Debug, PartialEq)]
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
#[derive(Clone, Debug, PartialEq)]
pub struct Code310 {
    pub argcount:        u32,
    pub posonlyargcount: u32,
    pub kwonlyargcount:  u32,
    pub nlocals:         u32,
    pub stacksize:       u32,
    pub flags:           CodeFlags,
    pub code:            Box<Object>, // Needs to contain Vec<u8> as a value or a reference
    pub consts:          Box<Object>, // Needs to contain Vec<Object> as a value or a reference
    pub names:           Box<Object>, // Needs to contain Vec<PyString> as a value or a reference
    pub varnames:        Box<Object>, // Needs to contain Vec<PyString> as a value or a reference
    pub freevars:        Box<Object>, // Needs to contain Vec<PyString> as a value or a reference
    pub cellvars:        Box<Object>, // Needs to contain Vec<PyString> as a value or a reference
    pub filename:        Box<Object>, // Needs to contain PyString> as a value or a reference
    pub name:            Box<Object>, // Needs to contain PyString as a value or a reference
    pub firstlineno:     u32,
    pub lnotab:          Box<Object>, // Needs to contain Vec<u8>, as a value or a reference
}

// Code object enum for all supported Python versions
#[derive(Clone, Debug, PartialEq)]
pub enum Code {
    // Contains the code object for Python 3.10
    V310(code_objects::Code310),
    // Contains the code object for Python 3.11
    V311(code_objects::Code311),
    // Contains the code object for Python 3.12 which is exactly the same as 3.11 so we use the same struct
    V312(code_objects::Code311),
    // Contains the code object for Python 3.13 which is exactly the same as 3.11 so we use the same struct
    V313(code_objects::Code311),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PyString {
    pub value: BString,
    pub kind: Kind,
}

impl From<String> for PyString {
    fn from(value: String) -> Self {
        Self {
            value: value.clone().into(),
            kind: {
                if value.is_ascii() {
                    if value.len() <= 255 {
                        Kind::ShortAscii
                    } else {
                        Kind::ASCII
                    }
                } else {
                    Kind::Unicode
                }
            }, // Default kind
        }
    }
}

impl PyString {
    pub fn new(value: BString, kind: Kind) -> Self {
        Self { value, kind }
    }
}

#[rustfmt::skip]
#[derive(Clone, Debug, PartialEq)]
pub enum Object {
    None,
    StopIteration,
    Ellipsis,
    Bool      (bool),
    Long      (BigInt),
    Float     (f64),
    Complex   (Complex<f64>),
    Bytes     (Vec<u8>),
    String    (PyString),
    Tuple     (Vec<Box<Object>>),
    List      (Vec<Box<Object>>),
    Dict      (IndexMap<ObjectHashable, Box<Object>>),
    Set       (IndexSet<ObjectHashable>),
    FrozenSet (IndexSet<ObjectHashable>),
    Code      (Box<Code>),
    LoadRef   (usize),
    StoreRef  (usize),
}

// impl Eq for Object {} // Required to check if Code objects are equal with float values

#[rustfmt::skip]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ObjectHashable {
    None,
    StopIteration,
    Ellipsis,
    Bool      (bool),
    Long      (BigInt),
    Float     (OrderedFloat<f64>),
    Complex   (Complex<OrderedFloat<f64>>),
    Bytes     (Vec<u8>),
    String    (PyString),
    Tuple     (Vec<ObjectHashable>),
    FrozenSet (HashableHashSet<ObjectHashable>),
    LoadRef   (usize), // You need to ensure that the reference is hashable
    StoreRef  (usize), // Same as above
}

impl ObjectHashable {
    pub fn from_ref(obj: Object, references: &Vec<Object>) -> Result<Self, Error> {
        // If the object is a reference, resolve it and make sure it's hashable
        match obj {
            Object::LoadRef(index) | Object::StoreRef(index) => {
                if let Some(resolved_obj) = references.get(index) {
                    let resolved_obj = resolved_obj.clone();
                    Self::from_ref(resolved_obj.clone(), references)?;
                    match obj {
                        Object::LoadRef(index) => Ok(Self::LoadRef(index)),
                        Object::StoreRef(index) => Ok(Self::StoreRef(index)),
                        _ => unreachable!(),
                    }
                } else {
                    Err(Error::InvalidReference)
                }
            }
            Object::Tuple(t) => Ok(Self::Tuple(
                // Tuple can contain references
                t.iter()
                    .map(|o| Self::from_ref((**o).clone(), references).unwrap())
                    .collect::<Vec<_>>(),
            )),
            _ => Self::try_from(obj),
        }
    }
}

impl TryFrom<Object> for ObjectHashable {
    type Error = Error;

    fn try_from(obj: Object) -> Result<Self, Self::Error> {
        match obj {
            Object::None => Ok(ObjectHashable::None),
            Object::StopIteration => Ok(ObjectHashable::StopIteration),
            Object::Ellipsis => Ok(ObjectHashable::Ellipsis),
            Object::Bool(b) => Ok(ObjectHashable::Bool(b)),
            Object::Long(i) => Ok(ObjectHashable::Long(i)),
            Object::Float(f) => Ok(ObjectHashable::Float(f.into())),
            Object::Complex(c) => Ok(ObjectHashable::Complex(Complex {
                re: OrderedFloat(c.re),
                im: OrderedFloat(c.im),
            })),
            Object::Bytes(b) => Ok(ObjectHashable::Bytes(b)),
            Object::String(s) => Ok(ObjectHashable::String(s)),
            Object::Tuple(t) => Ok(ObjectHashable::Tuple(
                t.iter()
                    .map(|o| ObjectHashable::try_from((**o).clone()).unwrap())
                    .collect::<Vec<_>>()
                    .into(),
            )),
            Object::FrozenSet(s) => Ok(ObjectHashable::FrozenSet(
                s.iter()
                    .map(|o| (*o).clone())
                    .collect::<HashableHashSet<_>>(),
            )),
            _ => Err(Error::InvalidObject(obj)),
        }
    }
}

impl From<ObjectHashable> for Object {
    fn from(obj: ObjectHashable) -> Self {
        match obj {
            ObjectHashable::None => Object::None,
            ObjectHashable::StopIteration => Object::StopIteration,
            ObjectHashable::Ellipsis => Object::Ellipsis,
            ObjectHashable::Bool(b) => Object::Bool(b),
            ObjectHashable::Long(i) => Object::Long(i),
            ObjectHashable::Float(f) => Object::Float(f.into_inner()),
            ObjectHashable::Complex(c) => Object::Complex(Complex {
                re: c.re.into_inner(),
                im: c.im.into_inner(),
            }),
            ObjectHashable::Bytes(b) => Object::Bytes(b),
            ObjectHashable::String(s) => Object::String(s),
            ObjectHashable::Tuple(t) => Object::Tuple(
                t.iter()
                    .map(|o| Box::new(Object::from((*o).clone())))
                    .collect::<Vec<_>>()
                    .into(),
            ),
            ObjectHashable::FrozenSet(s) => {
                Object::FrozenSet(s.iter().cloned().collect::<IndexSet<_>>())
            }
            ObjectHashable::LoadRef(index) => Object::LoadRef(index),
            ObjectHashable::StoreRef(index) => Object::StoreRef(index),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PycFile {
    pub python_version: PyVersion,
    pub timestamp: Option<u32>, // Only present in Python 3.7 and later
    pub hash: u64,
    pub object: Object,
    pub references: Vec<Object>,
}

pub fn optimize_references(object: Object, references: Vec<Object>) -> (Object, Vec<Object>) {
    // Remove all unused references
    let mut object = object;

    let usage_counter = get_used_references(&mut object, references.clone());

    let mut optimizer = ReferenceOptimizer::new(references, usage_counter);

    object.transform(&mut optimizer);

    (object, optimizer.new_references)
}

pub fn load_bytes(data: &[u8], python_version: PyVersion) -> Result<(Object, Vec<Object>), Error> {
    if python_version < (3, 0) {
        return Err(Error::UnsupportedPyVersion(python_version));
    }

    let mut py_reader = PyReader::new(data.to_vec(), python_version);

    let object = py_reader.read_object()?;

    Ok((object, py_reader.references))
}

pub fn load_pyc(data: impl Read) -> Result<PycFile, Error> {
    let data = data.bytes().collect::<Result<Vec<u8>, _>>()?;

    let magic_number = u32::from_le_bytes(data[0..4].try_into().unwrap());
    let python_version = PyVersion::try_from(magic_number)?;

    let timestamp = if python_version >= (3, 7) {
        Some(u32::from_le_bytes(data[4..8].try_into().unwrap()))
    } else {
        None
    };

    let hash = if python_version >= (3, 7) {
        u64::from_le_bytes(data[8..16].try_into().unwrap())
    } else {
        u64::from_le_bytes(data[4..12].try_into().unwrap())
    };

    let data = &data[16..];

    let (object, references) = load_bytes(data, python_version)?;

    Ok(PycFile {
        python_version,
        timestamp,
        hash,
        object,
        references,
    })
}

pub fn dump_pyc(writer: &mut impl Write, pyc_file: PycFile) -> Result<(), Error> {
    let mut buf = Vec::new();
    let mut py_writer = PyWriter::new(pyc_file.references, 4);

    buf.extend_from_slice(&u32::to_le_bytes(pyc_file.python_version.to_magic()?));
    if let Some(timestamp) = pyc_file.timestamp {
        buf.extend_from_slice(&u32::to_le_bytes(timestamp));
    }
    buf.extend_from_slice(&u64::to_le_bytes(pyc_file.hash));
    buf.extend_from_slice(&py_writer.write_object(Some(pyc_file.object)));

    std::io::copy(&mut buf.as_slice(), writer)?;

    Ok(())
}

pub fn dump_bytes(
    obj: Object,
    references: Option<Vec<Object>>,
    python_version: PyVersion,
    marshal_version: u8,
) -> Result<Vec<u8>, Error> {
    if python_version < (3, 0) {
        return Err(Error::UnsupportedPyVersion(python_version));
    }

    let mut py_writer = PyWriter::new(references.unwrap_or(Vec::new()), marshal_version);

    Ok(py_writer.write_object(Some(obj)))
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};
    use std::io::Write;
    use std::vec;

    use tempfile::NamedTempFile;

    use error::Error;

    use crate::resolver::{get_recursive_refs, resolve_all_refs};

    use super::*;

    #[test]
    fn test_load_long() {
        // 1
        let data = b"i\x01\x00\x00\x00";
        let (kind, _) = load_bytes(data, (3, 10).into()).unwrap();
        assert_eq!(
            extract_object!(Some(kind), Object::Long(num) => num, Error::UnexpectedObject).unwrap(),
            BigInt::from(1).into()
        );

        // 4294967295
        let data = b"l\x03\x00\x00\x00\xff\x7f\xff\x7f\x03\x00";
        let (kind, _) = load_bytes(data, (3, 10).into()).unwrap();

        assert_eq!(
            extract_object!(Some(kind), Object::Long(num) => num, Error::UnexpectedObject).unwrap(),
            BigInt::from(4294967295u32).into()
        );
    }

    #[test]
    fn test_load_float() {
        // 1.0
        let data = b"g\x00\x00\x00\x00\x00\x00\xf0?";
        let (kind, _) = load_bytes(data, (3, 10).into()).unwrap();
        assert_eq!(
            extract_object!(Some(kind), Object::Float(num) => num, Error::UnexpectedObject)
                .unwrap(),
            1.0
        );
    }

    #[test]
    fn test_load_complex() {
        // 3 + 4j
        let data = b"y\x00\x00\x00\x00\x00\x00\x08@\x00\x00\x00\x00\x00\x00\x10@";
        let (kind, _) = load_bytes(data, (3, 10).into()).unwrap();
        assert_eq!(
            extract_object!(Some(kind), Object::Complex(num) => num, Error::UnexpectedObject)
                .unwrap(),
            Complex::new(3.0, 4.0)
        );
    }

    #[test]
    fn test_load_bytes() {
        // b"test"
        let data = b"s\x04\x00\x00\x00test";
        let (kind, _) = load_bytes(data, (3, 10).into()).unwrap();
        assert_eq!(
            extract_object!(Some(kind), Object::Bytes(bytes) => bytes, Error::UnexpectedObject)
                .unwrap(),
            "test".as_bytes().to_vec()
        );
    }

    #[test]
    fn test_load_string() {
        // "test"
        let data = b"Z\x04test";
        let (kind, _) = load_bytes(data, (3, 10).into()).unwrap();
        assert_eq!(
            extract_object!(Some(kind), Object::String(string) => string, Error::UnexpectedObject)
                .unwrap(),
            PyString::new("test".into(), Kind::ShortAsciiInterned).into()
        );

        // "\xe9"
        let data = b"u\x03\x00\x00\x00\xed\xb2\x80";
        let (kind, _) = load_bytes(data, (3, 10).into()).unwrap();

        assert_eq!(
            extract_object!(Some(kind), Object::String(string) => string, Error::UnexpectedObject)
                .unwrap(),
            PyString::new(BString::new([237, 178, 128].to_vec()), Kind::Unicode).into()
        );
    }

    #[test]
    fn test_load_tuple() {
        // Empty tuple
        let data = b")\x00";
        let (kind, refs) = load_bytes(data, (3, 10).into()).unwrap();
        assert_eq!(
            extract_strings_tuple!(
                extract_object!(Some(kind), Object::Tuple(tuple) => tuple, Error::UnexpectedObject)
                    .unwrap(),
                refs
            )
            .unwrap(),
            vec![]
        );

        // Tuple with two elements ("a", "b")
        let data = b")\x02Z\x01aZ\x01b";
        let (kind, refs) = load_bytes(data, (3, 10).into()).unwrap();
        assert_eq!(
            extract_strings_tuple!(
                extract_object!(Some(kind), Object::Tuple(tuple) => tuple, Error::UnexpectedObject)
                    .unwrap(),
                refs
            )
            .unwrap(),
            vec![
                PyString::new("a".into(), Kind::ShortAsciiInterned).into(),
                PyString::new("b".into(), Kind::ShortAsciiInterned).into()
            ]
        );
    }

    #[test]
    fn test_load_list() {
        // Empty list
        let data = b"[\x00\x00\x00\x00";
        let (kind, _) = load_bytes(data, (3, 10).into()).unwrap();
        assert_eq!(
            extract_strings_list!(
                extract_object!(Some(kind), Object::List(list) => list, Error::UnexpectedObject)
                    .unwrap()
            )
            .unwrap(),
            vec![]
        );

        // List with two elements ("a", "b")
        let data = b"[\x02\x00\x00\x00Z\x01aZ\x01b";
        let (kind, _) = load_bytes(data, (3, 10).into()).unwrap();
        assert_eq!(
            extract_strings_list!(
                extract_object!(Some(kind), Object::List(list) => list, Error::UnexpectedObject)
                    .unwrap()
            )
            .unwrap(),
            vec![
                PyString::new("a".into(), Kind::ShortAsciiInterned).into(),
                PyString::new("b".into(), Kind::ShortAsciiInterned).into()
            ]
        );
    }

    #[test]
    fn test_reference() {
        // Reference to the first element
        let data = b"\xdb\x03\x00\x00\x00\xe9\x01\x00\x00\x00r\x01\x00\x00\x00r\x01\x00\x00\x00";
        let (kind, refs) = load_bytes(data, (3, 10).into()).unwrap();

        assert_eq!(
            extract_object!(Some(kind.clone()), Object::StoreRef(index) => index, Error::UnexpectedObject)
                .unwrap(),
            0
        );

        assert_eq!(
            resolve_object_ref!(Some(kind.clone()), refs).unwrap(),
            Object::List(
                vec![
                    Object::StoreRef(1).into(),
                    Object::LoadRef(1).into(),
                    Object::LoadRef(1).into()
                ]
                .into()
            )
        );

        assert_eq!(*refs.get(1).unwrap(), Object::Long(BigInt::from(1)).into());

        // Recursive reference
        let data = b"\xdb\x01\x00\x00\x00r\x00\x00\x00\x00";

        let (kind, refs) = load_bytes(data, (3, 10).into()).unwrap();

        dbg!(&kind, &refs);
        dbg!(get_recursive_refs(kind, refs).unwrap());
    }

    #[test]
    fn test_resolve_refs() {
        // Reference to the first element
        let data = b"\xdb\x03\x00\x00\x00\xe9\x01\x00\x00\x00r\x01\x00\x00\x00r\x01\x00\x00\x00";
        let (kind, refs) = load_bytes(data, (3, 10).into()).unwrap();

        let (obj, refs) = resolve_all_refs(kind, refs).unwrap();

        assert_eq!(
            extract_object!(Some(obj), Object::List(list) => list, Error::UnexpectedObject)
                .unwrap()
                .iter()
                .map(|o| *o.clone())
                .collect::<Vec<_>>(),
            vec![
                Object::Long(BigInt::from(1)),
                Object::Long(BigInt::from(1)),
                Object::Long(BigInt::from(1))
            ]
        );

        assert_eq!(refs.len(), 0);

        let kind = Object::StoreRef(0);
        let refs = vec![
            Object::List(vec![Object::StoreRef(1).into(), Object::LoadRef(1).into()]).into(),
            Object::StoreRef(2),
            Object::Long(BigInt::from(1)).into(),
        ];

        let (kind, refs) = resolve_all_refs(kind, refs).unwrap();

        assert_eq!(
            kind,
            Object::List(
                vec![
                    Object::Long(BigInt::from(1)).into(),
                    Object::Long(BigInt::from(1)).into()
                ]
                .into()
            )
        );

        assert_eq!(refs.len(), 0);
    }

    #[test]
    fn test_load_dict() {
        // Empty dict
        let data = b"{0";
        let (kind, _) = load_bytes(data, (3, 10).into()).unwrap();
        assert_eq!(
            extract_strings_dict!(
                extract_object!(Some(kind), Object::Dict(dict) => dict, Error::UnexpectedObject)
                    .unwrap()
            )
            .unwrap(),
            HashMap::new()
        );

        // Dict with two elements {"a": "b", "c": "d"}
        let data = b"{Z\x01aZ\x01bZ\x01cZ\x01d0";
        let (kind, _) = load_bytes(data, (3, 10).into()).unwrap();
        assert_eq!(
            extract_strings_dict!(
                extract_object!(Some(kind), Object::Dict(dict) => dict, Error::UnexpectedObject)
                    .unwrap()
            )
            .unwrap(),
            {
                let mut map = HashMap::new();
                map.insert(
                    PyString::new("a".into(), Kind::ShortAsciiInterned).into(),
                    PyString::new("b".into(), Kind::ShortAsciiInterned).into(),
                );
                map.insert(
                    PyString::new("c".into(), Kind::ShortAsciiInterned).into(),
                    PyString::new("d".into(), Kind::ShortAsciiInterned).into(),
                );
                map
            }
        );
    }

    #[test]
    fn test_load_set() {
        // Empty set
        let data = b"<\x00\x00\x00\x00";
        let (kind, _) = load_bytes(data, (3, 10).into()).unwrap();
        assert_eq!(
            extract_strings_set!(
                extract_object!(Some(kind), Object::Set(set) => set, Error::UnexpectedObject)
                    .unwrap()
            )
            .unwrap(),
            HashSet::new().into()
        );

        // Set with two elements {"a", "b"}
        let data = b"<\x02\x00\x00\x00Z\x01bZ\x01a";
        let (kind, _) = load_bytes(data, (3, 10).into()).unwrap();
        assert_eq!(
            extract_strings_set!(
                extract_object!(Some(kind), Object::Set(set) => set, Error::UnexpectedObject)
                    .unwrap()
            )
            .unwrap(),
            {
                let mut set = HashSet::new();
                set.insert(PyString::new("a".into(), Kind::ShortAsciiInterned).into());
                set.insert(PyString::new("b".into(), Kind::ShortAsciiInterned).into());
                set
            }
        );
    }

    #[test]
    fn test_load_frozenset() {
        // Empty frozenset
        let data = b">\x00\x00\x00\x00";
        let (kind, _) = load_bytes(data, (3, 10).into()).unwrap();
        assert_eq!(
            extract_strings_frozenset!(
                extract_object!(Some(kind), Object::FrozenSet(set) => set, Error::UnexpectedObject)
                    .unwrap()
            )
            .unwrap(),
            HashSet::new()
        );

        // Frozenset with two elements {"a", "b"}
        let data = b">\x02\x00\x00\x00Z\x01bZ\x01a";
        let (kind, _) = load_bytes(data, (3, 10).into()).unwrap();
        assert_eq!(
            extract_strings_frozenset!(
                extract_object!(Some(kind), Object::FrozenSet(set) => set, Error::UnexpectedObject)
                    .unwrap()
            )
            .unwrap(),
            {
                let mut set = HashSet::new();
                set.insert(PyString::new("a".into(), Kind::ShortAsciiInterned).into());
                set.insert(PyString::new("b".into(), Kind::ShortAsciiInterned).into());
                set
            }
        );
    }

    #[test]
    fn test_load_code310() {
        // def f(arg1, arg2=None): print(arg1, arg2)
        let data = b"\xe3\x02\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x02\x00\x00\x00\x03\x00\x00\x00C\x00\x00\x00s\x0e\x00\x00\x00t\x00|\x00|\x01\x83\x02\x01\x00d\x00S\x00\xa9\x01N)\x01\xda\x05print)\x02Z\x04arg1Z\x04arg2\xa9\x00r\x03\x00\x00\x00\xfa\x07<stdin>\xda\x01f\x01\x00\x00\x00s\x02\x00\x00\x00\x0e\x00";
        let (kind, refs) = load_bytes(data, (3, 10).into()).unwrap();

        let code = extract_object!(Some(resolve_object_ref!(Some(kind), refs).unwrap()), Object::Code(code) => code, Error::UnexpectedObject)
                .unwrap().clone();

        match *code {
            Code::V310(code) => {
                let inner_code = extract_object!(Some(resolve_object_ref!(Some((*code.code).clone()), &refs).unwrap()), Object::Bytes(bytes) => bytes, Error::NullInTuple).unwrap();
                let inner_consts = extract_object!(Some(resolve_object_ref!(Some((*code.consts).clone()), &refs).unwrap()), Object::Tuple(objs) => objs, Error::NullInTuple).unwrap();
                let inner_names = extract_strings_tuple!(extract_object!(Some(resolve_object_ref!(Some((*code.names).clone()), &refs).unwrap()), Object::Tuple(objs) => objs, Error::NullInTuple).unwrap(), &refs).unwrap();
                let inner_varnames = extract_strings_tuple!(extract_object!(Some(resolve_object_ref!(Some((*code.varnames).clone()), &refs).unwrap()), Object::Tuple(objs) => objs, Error::NullInTuple).unwrap(), &refs).unwrap();
                let inner_freevars = extract_strings_tuple!(extract_object!(Some(resolve_object_ref!(Some((*code.freevars).clone()), &refs).unwrap()), Object::Tuple(objs) => objs, Error::NullInTuple).unwrap(), &refs).unwrap();
                let inner_cellvars = extract_strings_tuple!(extract_object!(Some(resolve_object_ref!(Some((*code.cellvars).clone()), &refs).unwrap()), Object::Tuple(objs) => objs, Error::NullInTuple).unwrap(), &refs).unwrap();
                let inner_filename = extract_object!(Some(resolve_object_ref!(Some((*code.filename).clone()), &refs).unwrap()), Object::String(string) => string, Error::UnexpectedObject).unwrap();
                let inner_name = extract_object!(Some(resolve_object_ref!(Some((*code.name).clone()), &refs).unwrap()), Object::String(string) => string, Error::UnexpectedObject).unwrap();
                let inner_lnotab = extract_object!(Some(resolve_object_ref!(Some((*code.lnotab).clone()), &refs).unwrap()), Object::Bytes(bytes) => bytes, Error::NullInTuple).unwrap();

                assert_eq!(code.argcount, 2);
                assert_eq!(code.posonlyargcount, 0);
                assert_eq!(code.kwonlyargcount, 0);
                assert_eq!(code.nlocals, 2);
                assert_eq!(code.stacksize, 3);
                // assert_eq!(code.flags, );
                assert_eq!(inner_code.len(), 14);
                assert_eq!(inner_consts.len(), 1);
                assert_eq!(inner_names.len(), 1);
                assert_eq!(inner_varnames.len(), 2);
                assert_eq!(inner_freevars.len(), 0);
                assert_eq!(inner_cellvars.len(), 0);
                assert_eq!(
                    inner_filename,
                    PyString::new("<stdin>".into(), Kind::ShortAscii).into()
                );
                assert_eq!(
                    inner_name,
                    PyString::new("f".into(), Kind::ShortAsciiInterned).into()
                );
                assert_eq!(code.firstlineno, 1);
                assert_eq!(inner_lnotab.len(), 2);
            }
            _ => panic!("Invalid code object"),
        }
    }

    #[test]
    fn test_load_code311() {
        // def f(arg1, arg2=None): print(arg1, arg2)
        let data = b"\xe3\x02\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x04\x00\x00\x00\x03\x00\x00\x00\xf3&\x00\x00\x00\x97\x00t\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00|\x00|\x01\xa6\x02\x00\x00\xab\x02\x00\x00\x00\x00\x00\x00\x00\x00\x01\x00d\x00S\x00\xa9\x01N)\x01\xda\x05print)\x02\xda\x04arg1\xda\x04arg2s\x02\x00\x00\x00  \xfa\x07<stdin>\xda\x01fr\x07\x00\x00\x00\x01\x00\x00\x00s\x17\x00\x00\x00\x80\x00\x9d\x05\x98d\xa0D\xd1\x18)\xd4\x18)\xd0\x18)\xd0\x18)\xd0\x18)\xf3\x00\x00\x00\x00";
        let (kind, refs) = load_bytes(data, (3, 11).into()).unwrap();

        let code = extract_object!(Some(resolve_object_ref!(Some(kind), refs).unwrap()), Object::Code(code) => code, Error::UnexpectedObject)
                .unwrap().clone();

        match *code {
            Code::V311(code) => {
                let inner_code = extract_object!(Some(resolve_object_ref!(Some((*code.code).clone()), &refs).unwrap()), Object::Bytes(bytes) => bytes, Error::NullInTuple).unwrap();
                let inner_consts = extract_object!(Some(resolve_object_ref!(Some((*code.consts).clone()), &refs).unwrap()), Object::Tuple(objs) => objs, Error::NullInTuple).unwrap();
                let inner_names = extract_strings_tuple!(extract_object!(Some(resolve_object_ref!(Some((*code.names).clone()), &refs).unwrap()), Object::Tuple(objs) => objs, Error::NullInTuple).unwrap(), &refs).unwrap();
                let inner_localsplusnames = extract_strings_tuple!(extract_object!(Some(resolve_object_ref!(Some((*code.localsplusnames).clone()), &refs).unwrap()), Object::Tuple(objs) => objs, Error::NullInTuple).unwrap(), &refs).unwrap();
                let inner_localspluskinds = extract_object!(Some(resolve_object_ref!(Some((*code.localspluskinds).clone()), &refs).unwrap()), Object::Bytes(bytes) => bytes, Error::NullInTuple).unwrap();
                let inner_filename = extract_object!(Some(resolve_object_ref!(Some((*code.filename).clone()), &refs).unwrap()), Object::String(string) => string, Error::UnexpectedObject).unwrap();
                let inner_name = extract_object!(Some(resolve_object_ref!(Some((*code.name).clone()), &refs).unwrap()), Object::String(string) => string, Error::UnexpectedObject).unwrap();
                let inner_linetable = extract_object!(Some(resolve_object_ref!(Some((*code.linetable).clone()), &refs).unwrap()), Object::Bytes(bytes) => bytes, Error::NullInTuple).unwrap();
                let inner_exceptiontable = extract_object!(Some(resolve_object_ref!(Some((*code.exceptiontable).clone()), &refs).unwrap()), Object::Bytes(bytes) => bytes, Error::NullInTuple).unwrap();

                assert_eq!(code.argcount, 2);
                assert_eq!(code.posonlyargcount, 0);
                assert_eq!(code.kwonlyargcount, 0);
                assert_eq!(code.stacksize, 4);
                // assert_eq!(code.flags, );
                assert_eq!(inner_code.len(), 38);
                assert_eq!(inner_consts.len(), 1);
                assert_eq!(inner_names.len(), 1);
                assert_eq!(inner_localsplusnames.len(), 2);
                assert_eq!(inner_localspluskinds.len(), 2);
                assert_eq!(
                    inner_filename,
                    PyString::new("<stdin>".into(), Kind::ShortAscii).into()
                );
                assert_eq!(
                    inner_name,
                    PyString::new("f".into(), Kind::ShortAsciiInterned).into()
                );
                assert_eq!(code.firstlineno, 1);
                assert_eq!(inner_linetable.len(), 23);
                assert_eq!(inner_exceptiontable.len(), 0);
            }
            _ => panic!("Invalid code object"),
        }
    }

    #[test]
    fn test_load_pyc() {
        let data = b"o\r\r\n\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\xe3\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x02\x00\x00\x00@\x00\x00\x00s\x0c\x00\x00\x00e\x00d\x00\x83\x01\x01\x00d\x01S\x00)\x02z\x0ehi from PythonN)\x01\xda\x05print\xa9\x00r\x02\x00\x00\x00r\x02\x00\x00\x00z\x08<string>\xda\x08<module>\x01\x00\x00\x00s\x02\x00\x00\x00\x0c\x00";

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(data).unwrap();

        let obj = load_pyc(&data[..]).unwrap();

        dbg!(&obj); // TODO: Add assertions
    }

    #[test]
    fn test_dump_long() {
        // 1
        let data = b"i\x01\x00\x00\x00";
        let object = Object::Long(BigInt::from(1).into());
        let dumped = dump_bytes(object, None, (3, 10).into(), 4).unwrap();
        assert_eq!(data.to_vec(), dumped);

        // 4294967295
        let data = b"l\x03\x00\x00\x00\xff\x7f\xff\x7f\x03\x00";
        let object = Object::Long(BigInt::from(4294967295u32).into());
        let dumped = dump_bytes(object, None, (3, 10).into(), 4).unwrap();
        assert_eq!(data.to_vec(), dumped);
    }

    #[test]
    fn test_dump_float() {
        // 1.0
        let data = b"g\x00\x00\x00\x00\x00\x00\xf0?";
        let object = Object::Float(1.0);
        let dumped = dump_bytes(object, None, (3, 10).into(), 4).unwrap();
        assert_eq!(data.to_vec(), dumped);
    }

    #[test]
    fn test_dump_complex() {
        // 3 + 4j
        let data = b"y\x00\x00\x00\x00\x00\x00\x08@\x00\x00\x00\x00\x00\x00\x10@";
        let object = Object::Complex(Complex::new(3.0, 4.0));
        let dumped = dump_bytes(object, None, (3, 10).into(), 4).unwrap();
        assert_eq!(data.to_vec(), dumped);
    }

    #[test]
    fn test_dump_bytes() {
        // b"test"
        let data = b"s\x04\x00\x00\x00test";
        let object = Object::Bytes("test".as_bytes().to_vec().into());
        let dumped = dump_bytes(object, None, (3, 10).into(), 4).unwrap();
        assert_eq!(data.to_vec(), dumped);
    }

    #[test]
    fn test_dump_string() {
        // "test"
        let data = b"z\x04test";
        let object = Object::String(PyString::from("test".to_string()).into());
        let dumped = dump_bytes(object, None, (3, 10).into(), 4).unwrap();
        assert_eq!(data.to_vec(), dumped);

        // "\xe9"
        let data = b"u\x03\x00\x00\x00\xed\xb2\x80";
        let object = Object::String(
            PyString::new(BString::new([237, 178, 128].to_vec()), Kind::Unicode).into(),
        );
        let dumped = dump_bytes(object, None, (3, 10).into(), 4).unwrap();
        assert_eq!(data.to_vec(), dumped);
    }

    #[test]
    fn test_dump_tuple() {
        // Empty tuple
        let data = b")\x00";
        let object = Object::Tuple(vec![].into());
        let dumped = dump_bytes(object, None, (3, 10).into(), 4).unwrap();
        assert_eq!(data.to_vec(), dumped);

        // Tuple with two elements ("a", "b")
        let data = b")\x02z\x01az\x01b";
        let object = Object::Tuple(
            vec![
                Object::String(PyString::from("a".to_string()).into()).into(),
                Object::String(PyString::from("b".to_string()).into()).into(),
            ]
            .into(),
        );
        let dumped = dump_bytes(object, None, (3, 10).into(), 4).unwrap();
        assert_eq!(data.to_vec(), dumped);
    }

    #[test]
    fn test_dump_list() {
        // Empty list
        let data = b"[\x00\x00\x00\x00";
        let object = Object::List(vec![].into());
        let dumped = dump_bytes(object, None, (3, 10).into(), 4).unwrap();
        assert_eq!(data.to_vec(), dumped);

        // List with two elements ("a", "b")
        let data = b"[\x02\x00\x00\x00z\x01az\x01b";
        let object = Object::List(
            vec![
                Object::String(PyString::from("a".to_string()).into()).into(),
                Object::String(PyString::from("b".to_string()).into()).into(),
            ]
            .into(),
        );
        dbg!(&object);
        let dumped = dump_bytes(object, None, (3, 10).into(), 4).unwrap();
        assert_eq!(data.to_vec(), dumped);
    }

    #[test]
    fn test_dump_dict() {
        // Empty dict
        let data = b"{0";
        let object = Object::Dict(IndexMap::new().into());
        let dumped = dump_bytes(object, None, (3, 10).into(), 4).unwrap();
        assert_eq!(data.to_vec(), dumped);

        // Dict with two elements {"a": "b", "c": "d"}
        let data1 = b"{z\x01az\x01bz\x01cz\x01d0";
        let data2 = b"{z\x01cz\x01dz\x01az\x01b0"; // Order is not guaranteed
        let object = Object::Dict({
            let mut map = IndexMap::new();
            map.insert(
                ObjectHashable::String(PyString::from("a".to_string()).into()).into(),
                Object::String(PyString::from("b".to_string()).into()).into(),
            );
            map.insert(
                ObjectHashable::String(PyString::from("c".to_string()).into()).into(),
                Object::String(PyString::from("d".to_string()).into()).into(),
            );
            map
        });
        let dumped = dump_bytes(object, None, (3, 10).into(), 4).unwrap();
        assert!(data1.to_vec() == dumped || data2.to_vec() == dumped);
    }

    #[test]
    fn test_dump_set() {
        // Empty set
        let data = b"<\x00\x00\x00\x00";
        let object = Object::Set(IndexSet::new().into());
        let dumped = dump_bytes(object, None, (3, 10).into(), 4).unwrap();
        assert_eq!(data.to_vec(), dumped);

        // Set with two elements {"a", "b"}
        let data1 = b"<\x02\x00\x00\x00z\x01az\x01b";
        let data2 = b"<\x02\x00\x00\x00z\x01bz\x01a"; // Order is not guaranteed
        let object = Object::Set({
            let mut set = IndexSet::new();
            set.insert(ObjectHashable::String(PyString::from("a".to_string()).into()).into());
            set.insert(ObjectHashable::String(PyString::from("b".to_string()).into()).into());
            set
        });
        let dumped = dump_bytes(object, None, (3, 10).into(), 4).unwrap();
        assert!(data1.to_vec() == dumped || data2.to_vec() == dumped);
    }

    #[test]
    fn test_dump_frozenset() {
        // Empty frozenset
        let data = b">\x00\x00\x00\x00";
        let object = Object::FrozenSet(IndexSet::new().into());
        let dumped = dump_bytes(object, None, (3, 10).into(), 4).unwrap();
        assert_eq!(data.to_vec(), dumped);

        // Frozenset with two elements {"a", "b"}
        let data1 = b">\x02\x00\x00\x00z\x01az\x01b"; // Order is not guaranteed
        let data2 = b">\x02\x00\x00\x00z\x01bz\x01a";
        let object = Object::FrozenSet({
            let mut set = IndexSet::new();
            set.insert(ObjectHashable::String(PyString::from("a".to_string()).into()).into());
            set.insert(ObjectHashable::String(PyString::from("b".to_string()).into()).into());
            set
        });
        let dumped = dump_bytes(object, None, (3, 10).into(), 4).unwrap();
        assert!(data1.to_vec() == dumped || data2.to_vec() == dumped || data2.to_vec() == dumped);
    }

    #[test]
    fn test_dump_code() {
        // def f(arg1, arg2=None): print(arg1, arg2)
        let data = b"c\x02\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x02\x00\x00\x00\x03\x00\x00\x00C\x00\x00\x00s\x0e\x00\x00\x00t\x00|\x00|\x01\x83\x02\x01\x00d\x00S\x00)\x01N)\x01z\x05print)\x02z\x04arg1z\x04arg2)\x00)\x00z\x07<stdin>z\x01f\x01\x00\x00\x00s\x02\x00\x00\x00\x0e\x00";

        let object = Code::V310(code_objects::Code310 {
            argcount: 2,
            posonlyargcount: 0,
            kwonlyargcount: 0,
            nlocals: 2,
            stacksize: 3,
            flags: CodeFlags::from_bits_truncate(0x43),
            code: Object::Bytes(vec![116, 0, 124, 0, 124, 1, 131, 2, 1, 0, 100, 0, 83, 0].into())
                .into(),
            consts: Object::Tuple([Object::None.into()].to_vec().into()).into(),
            names: Object::Tuple(
                [Object::String(PyString::from("print".to_string()).into()).into()]
                    .to_vec()
                    .into(),
            )
            .into(),
            varnames: Object::Tuple(
                [
                    Object::String(PyString::from("arg1".to_string()).into()).into(),
                    Object::String(PyString::from("arg2".to_string()).into()).into(),
                ]
                .to_vec()
                .into(),
            )
            .into(),
            freevars: Object::Tuple([].to_vec().into()).into(),
            cellvars: Object::Tuple([].to_vec().into()).into(),
            filename: Object::String(PyString::from("<stdin>".to_string()).into()).into(),
            name: Object::String(PyString::from("f".to_string()).into()).into(),
            firstlineno: 1,
            lnotab: Object::Bytes([14, 0].to_vec().into()).into(),
        });
        let dumped = dump_bytes(Object::Code(object.into()), None, (3, 10).into(), 4).unwrap();

        assert_eq!(data.to_vec(), dumped);
    }

    #[test]
    fn test_recompile() {
        let data = b"c\x02\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x02\x00\x00\x00\x03\x00\x00\x00C\x00\x00\x00s\x0e\x00\x00\x00t\x00|\x00|\x01\x83\x02\x01\x00d\x00S\x00)\x01N)\x01z\x05print)\x02z\x04arg1z\x04arg2)\x00)\x00z\x07<stdin>z\x01f\x01\x00\x00\x00s\x02\x00\x00\x00\x0e\x00";

        let (kind, refs) = load_bytes(data, (3, 10).into()).unwrap();
        let dumped = dump_bytes(kind, Some(refs), (3, 10).into(), 4).unwrap();

        assert_eq!(data.to_vec(), dumped);
    }

    #[test]
    fn test_optimize_references() {
        let data = b"\xdb\x03\x00\x00\x00\xe9\x01\x00\x00\x00r\x01\x00\x00\x00r\x01\x00\x00\x00";
        let (kind, refs) = load_bytes(data, (3, 10).into()).unwrap();

        let (kind, refs) = optimize_references(kind, refs);

        dump_bytes(kind.clone(), Some(refs.clone()), (3, 10).into(), 4).unwrap();

        assert_eq!(
            kind,
            Object::List(
                vec![
                    Object::StoreRef(0).into(),
                    Object::LoadRef(0).into(),
                    Object::LoadRef(0).into()
                ]
                .into()
            )
        );

        assert_eq!(*refs.get(0).unwrap(), Object::Long(BigInt::from(1)).into());

        let kind = Object::StoreRef(0);
        let refs = vec![
            Object::List(vec![Object::StoreRef(1).into(), Object::LoadRef(1).into()]).into(),
            Object::StoreRef(2),
            Object::Long(BigInt::from(1)).into(),
        ];

        let (kind, refs) = optimize_references(kind, refs);

        dump_bytes(kind.clone(), Some(refs.clone()), (3, 10).into(), 4).unwrap();

        assert_eq!(
            kind,
            Object::List(vec![Object::StoreRef(0).into(), Object::LoadRef(0).into(),].into())
        );

        assert_eq!(*refs.get(0).unwrap(), Object::Long(BigInt::from(1)).into());
    }
}
