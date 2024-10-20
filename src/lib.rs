mod error;
mod reader;
mod writer;

use bitflags::bitflags;
use error::Error;
use hashable::HashableHashSet;
use num_bigint::BigInt;
use num_complex::Complex;
use num_derive::{FromPrimitive, ToPrimitive};
use ordered_float::OrderedFloat;
use reader::PyReader;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use writer::PyWriter;

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

#[derive(Clone, Debug, PartialEq)]
pub enum Code {
    V310(Code310),
}

#[rustfmt::skip]
#[derive(Clone, Debug, PartialEq)]
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
    List      (Arc<Vec<Arc<Object>>>),
    Dict      (Arc<HashMap<ObjectHashable, Arc<Object>>>),
    Set       (Arc<HashSet<ObjectHashable>>),
    FrozenSet (Arc<HashSet<ObjectHashable>>),
    Code      (Arc<Code>),
    LoadRef   (usize),
    WriteRef  (usize),
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
                    .map(|o| ObjectHashable::try_from(o.clone()).unwrap())
                    .collect::<Vec<_>>()
                    .into(),
            )),
            Object::FrozenSet(s) => Ok(ObjectHashable::FrozenSet(
                s.iter().cloned().collect::<HashableHashSet<_>>().into(),
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
                    .map(|o| Object::from(o.clone()))
                    .collect::<Vec<_>>()
                    .into(),
            ),
            ObjectHashable::FrozenSet(s) => {
                Object::FrozenSet(s.iter().cloned().collect::<HashSet<_>>().into())
            }
        }
    }
}

pub fn load_bytes(
    data: &[u8],
    python_version: PyVersion,
) -> anyhow::Result<(Object, HashMap<usize, Arc<Object>>)> {
    if python_version < (3, 0) {
        return Err(anyhow::anyhow!("Python 2.x is not supported"));
    }

    let mut py_reader = PyReader::new(data.to_vec(), python_version);

    let object = py_reader.read_object()?;

    Ok((object, py_reader.references))
}

pub fn dump_bytes(
    obj: Object,
    references: Option<HashMap<usize, Arc<Object>>>,
    python_version: PyVersion,
    marshal_version: u8,
) -> anyhow::Result<Vec<u8>> {
    if python_version < (3, 0) {
        return Err(anyhow::anyhow!("Python 2.x is not supported"));
    }

    let mut py_writer = PyWriter::new(references.unwrap_or(HashMap::new()), marshal_version);

    Ok(py_writer.write_object(Some(obj)))
}

#[cfg(test)]
mod tests {
    use error::Error;

    use super::*;

    #[test]
    fn test_load_long() {
        // 1
        let data = b"i\x01\x00\x00\x00";
        let (kind, _) = load_bytes(data, (3, 10)).unwrap();
        assert_eq!(
            extract_object!(Some(kind), Object::Long(num) => num, Error::UnexpectedObject).unwrap(),
            BigInt::from(1).into()
        );
    }

    #[test]
    fn test_load_float() {
        // 1.0
        let data = b"g\x00\x00\x00\x00\x00\x00\xf0?";
        let (kind, _) = load_bytes(data, (3, 10)).unwrap();
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
        let (kind, _) = load_bytes(data, (3, 10)).unwrap();
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
        let (kind, _) = load_bytes(data, (3, 10)).unwrap();
        assert_eq!(
            extract_object!(Some(kind), Object::Bytes(bytes) => bytes, Error::UnexpectedObject)
                .unwrap(),
            "test".as_bytes().to_vec().into()
        );
    }

    #[test]
    fn test_load_string() {
        // "test"
        let data = b"Z\x04test";
        let (kind, _) = load_bytes(data, (3, 10)).unwrap();
        assert_eq!(
            extract_object!(Some(kind), Object::String(string) => string, Error::UnexpectedObject)
                .unwrap(),
            "test".to_string().into()
        );
    }

    #[test]
    fn test_load_tuple() {
        // Empty tuple
        let data = b")\x00";
        let (kind, refs) = load_bytes(data, (3, 10)).unwrap();
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
        let (kind, refs) = load_bytes(data, (3, 10)).unwrap();
        assert_eq!(
            extract_strings_tuple!(
                extract_object!(Some(kind), Object::Tuple(tuple) => tuple, Error::UnexpectedObject)
                    .unwrap(),
                refs
            )
            .unwrap(),
            vec!["a".to_string().into(), "b".to_string().into()]
        );
    }

    #[test]
    fn test_load_list() {
        // Empty list
        let data = b"[\x00\x00\x00\x00";
        let (kind, _) = load_bytes(data, (3, 10)).unwrap();
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
        let (kind, _) = load_bytes(data, (3, 10)).unwrap();
        assert_eq!(
            extract_strings_list!(
                extract_object!(Some(kind), Object::List(list) => list, Error::UnexpectedObject)
                    .unwrap()
            )
            .unwrap(),
            vec!["a".to_string().into(), "b".to_string().into()]
        );
    }

    #[test]
    fn test_reference() {
        // Reference to the first element
        let data = b"\xdb\x01\x00\x00\x00r\x00\x00\x00\x00";
        let (kind, refs) = load_bytes(data, (3, 10)).unwrap();

        assert_eq!(
            extract_object!(Some(kind.clone()), Object::WriteRef(index) => index, Error::UnexpectedObject)
                .unwrap(),
            0
        );

        assert_eq!(
            resolve_object_ref!(Some(kind.clone()), refs).unwrap(),
            Object::List(vec![Object::LoadRef(0).into()].into())
        );
    }

    #[test]
    fn test_load_dict() {
        // Empty dict
        let data = b"{0";
        let (kind, _) = load_bytes(data, (3, 10)).unwrap();
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
        let (kind, _) = load_bytes(data, (3, 10)).unwrap();
        assert_eq!(
            extract_strings_dict!(
                extract_object!(Some(kind), Object::Dict(dict) => dict, Error::UnexpectedObject)
                    .unwrap()
            )
            .unwrap(),
            {
                let mut map = HashMap::new();
                map.insert("a".to_string().into(), "b".to_string().into());
                map.insert("c".to_string().into(), "d".to_string().into());
                map
            }
        );
    }

    #[test]
    fn test_load_set() {
        // Empty set
        let data = b"<\x00\x00\x00\x00";
        let (kind, _) = load_bytes(data, (3, 10)).unwrap();
        assert_eq!(
            extract_strings_set!(
                extract_object!(Some(kind), Object::Set(set) => set, Error::UnexpectedObject)
                    .unwrap()
            )
            .unwrap(),
            HashSet::new()
        );

        // Set with two elements {"a", "b"}
        let data = b"<\x02\x00\x00\x00Z\x01bZ\x01a";
        let (kind, _) = load_bytes(data, (3, 10)).unwrap();
        assert_eq!(
            extract_strings_set!(
                extract_object!(Some(kind), Object::Set(set) => set, Error::UnexpectedObject)
                    .unwrap()
            )
            .unwrap(),
            {
                let mut set = HashSet::new();
                set.insert("a".to_string().into());
                set.insert("b".to_string().into());
                set
            }
        );
    }

    #[test]
    fn test_load_frozenset() {
        // Empty frozenset
        let data = b">\x00\x00\x00\x00";
        let (kind, _) = load_bytes(data, (3, 10)).unwrap();
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
        let (kind, _) = load_bytes(data, (3, 10)).unwrap();
        assert_eq!(
            extract_strings_frozenset!(
                extract_object!(Some(kind), Object::FrozenSet(set) => set, Error::UnexpectedObject)
                    .unwrap()
            )
            .unwrap(),
            {
                let mut set = HashSet::new();
                set.insert("a".to_string().into());
                set.insert("b".to_string().into());
                set
            }
        );
    }

    #[test]
    fn test_load_code() {
        // def f(arg1, arg2=None): print(arg1, arg2)
        let data = b"\xe3\x02\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x02\x00\x00\x00\x03\x00\x00\x00C\x00\x00\x00s\x0e\x00\x00\x00t\x00|\x00|\x01\x83\x02\x01\x00d\x00S\x00\xa9\x01N)\x01\xda\x05print)\x02Z\x04arg1Z\x04arg2\xa9\x00r\x03\x00\x00\x00\xfa\x07<stdin>\xda\x01f\x01\x00\x00\x00s\x02\x00\x00\x00\x0e\x00";
        let (kind, refs) = load_bytes(data, (3, 10)).unwrap();

        let code = (*extract_object!(Some(resolve_object_ref!(Some(kind), refs).unwrap()), Object::Code(code) => code, Error::UnexpectedObject)
                .unwrap()).clone();

        match code {
            Code::V310(code) => {
                assert_eq!(code.argcount, 2);
                assert_eq!(code.posonlyargcount, 0);
                assert_eq!(code.kwonlyargcount, 0);
                assert_eq!(code.nlocals, 2);
                assert_eq!(code.stacksize, 3);
                // assert_eq!(code.flags, );
                assert_eq!(code.code.len(), 14);
                assert_eq!(code.consts.len(), 1);
                assert_eq!(code.names.len(), 1);
                assert_eq!(code.varnames.len(), 2);
                assert_eq!(code.freevars.len(), 0);
                assert_eq!(code.cellvars.len(), 0);
                assert_eq!(code.filename, "<stdin>".to_string().into());
                assert_eq!(code.name, "f".to_string().into());
                assert_eq!(code.firstlineno, 1);
                assert_eq!(code.lnotab.len(), 2);
            }
        }
    }

    #[test]
    fn test_dump_long() {
        // 1
        let data = b"i\x01\x00\x00\x00";
        let object = Object::Long(BigInt::from(1).into());
        let dumped = dump_bytes(object, None, (3, 10), 4).unwrap();
        assert_eq!(data.to_vec(), dumped);
    }

    #[test]
    fn test_dump_float() {
        // 1.0
        let data = b"g\x00\x00\x00\x00\x00\x00\xf0?";
        let object = Object::Float(1.0);
        let dumped = dump_bytes(object, None, (3, 10), 4).unwrap();
        assert_eq!(data.to_vec(), dumped);
    }

    #[test]
    fn test_dump_complex() {
        // 3 + 4j
        let data = b"y\x00\x00\x00\x00\x00\x00\x08@\x00\x00\x00\x00\x00\x00\x10@";
        let object = Object::Complex(Complex::new(3.0, 4.0));
        let dumped = dump_bytes(object, None, (3, 10), 4).unwrap();
        assert_eq!(data.to_vec(), dumped);
    }

    #[test]
    fn test_dump_bytes() {
        // b"test"
        let data = b"s\x04\x00\x00\x00test";
        let object = Object::Bytes("test".as_bytes().to_vec().into());
        let dumped = dump_bytes(object, None, (3, 10), 4).unwrap();
        assert_eq!(data.to_vec(), dumped);
    }

    #[test]
    fn test_dump_string() {
        // "test"
        let data = b"z\x04test";
        let object = Object::String("test".to_string().into());
        let dumped = dump_bytes(object, None, (3, 10), 4).unwrap();
        assert_eq!(data.to_vec(), dumped);
    }

    #[test]
    fn test_dump_tuple() {
        // Empty tuple
        let data = b")\x00";
        let object = Object::Tuple(vec![].into());
        let dumped = dump_bytes(object, None, (3, 10), 4).unwrap();
        assert_eq!(data.to_vec(), dumped);

        // Tuple with two elements ("a", "b")
        let data = b")\x02z\x01az\x01b";
        let object = Object::Tuple(
            vec![
                Object::String("a".to_string().into()),
                Object::String("b".to_string().into()),
            ]
            .into(),
        );
        let dumped = dump_bytes(object, None, (3, 10), 4).unwrap();
        assert_eq!(data.to_vec(), dumped);
    }

    #[test]
    fn test_dump_list() {
        // Empty list
        let data = b"[\x00\x00\x00\x00";
        let object = Object::List(vec![].into());
        let dumped = dump_bytes(object, None, (3, 10), 4).unwrap();
        assert_eq!(data.to_vec(), dumped);

        // List with two elements ("a", "b")
        let data = b"[\x02\x00\x00\x00z\x01az\x01b";
        let object = Object::List(
            vec![
                Object::String("a".to_string().into()).into(),
                Object::String("b".to_string().into()).into(),
            ]
            .into(),
        );
        let dumped = dump_bytes(object, None, (3, 10), 4).unwrap();
        assert_eq!(data.to_vec(), dumped);
    }

    #[test]
    fn test_dump_dict() {
        // Empty dict
        let data = b"{0";
        let object = Object::Dict(HashMap::new().into());
        let dumped = dump_bytes(object, None, (3, 10), 4).unwrap();
        assert_eq!(data.to_vec(), dumped);

        // Dict with two elements {"a": "b", "c": "d"}
        let data1 = b"{z\x01az\x01bz\x01cz\x01d0";
        let data2 = b"{z\x01cz\x01dz\x01az\x01b0"; // Order is not guaranteed
        let object = Object::Dict(
            {
                let mut map = HashMap::new();
                map.insert(
                    ObjectHashable::String("a".to_string().into()),
                    Object::String("b".to_string().into()).into(),
                );
                map.insert(
                    ObjectHashable::String("c".to_string().into()),
                    Object::String("d".to_string().into()).into(),
                );
                map
            }
            .into(),
        );
        let dumped = dump_bytes(object, None, (3, 10), 4).unwrap();
        assert!(data1.to_vec() == dumped || data2.to_vec() == dumped);
    }

    #[test]
    fn test_dump_set() {
        // Empty set
        let data = b"<\x00\x00\x00\x00";
        let object = Object::Set(HashSet::new().into());
        let dumped = dump_bytes(object, None, (3, 10), 4).unwrap();
        assert_eq!(data.to_vec(), dumped);

        // Set with two elements {"a", "b"}
        let data1 = b"<\x02\x00\x00\x00z\x01az\x01b";
        let data2 = b"<\x02\x00\x00\x00z\x01bz\x01a"; // Order is not guaranteed
        let object = Object::Set(
            {
                let mut set = HashSet::new();
                set.insert(ObjectHashable::String("a".to_string().into()));
                set.insert(ObjectHashable::String("b".to_string().into()));
                set
            }
            .into(),
        );
        let dumped = dump_bytes(object, None, (3, 10), 4).unwrap();
        assert!(data1.to_vec() == dumped || data2.to_vec() == dumped);
    }

    #[test]
    fn test_dump_frozenset() {
        // Empty frozenset
        let data = b">\x00\x00\x00\x00";
        let object = Object::FrozenSet(HashSet::new().into());
        let dumped = dump_bytes(object, None, (3, 10), 4).unwrap();
        assert_eq!(data.to_vec(), dumped);

        // Frozenset with two elements {"a", "b"}
        let data1 = b">\x02\x00\x00\x00z\x01az\x01b"; // Order is not guaranteed
        let data2 = b">\x02\x00\x00\x00z\x01bz\x01a";
        let object = Object::FrozenSet(
            {
                let mut set = HashSet::new();
                set.insert(ObjectHashable::String("a".to_string().into()));
                set.insert(ObjectHashable::String("b".to_string().into()));
                set
            }
            .into(),
        );
        let dumped = dump_bytes(object, None, (3, 10), 4).unwrap();
        assert!(data1.to_vec() == dumped || data2.to_vec() == dumped || data2.to_vec() == dumped);
    }

    #[test]
    fn test_dump_code() {
        // def f(arg1, arg2=None): print(arg1, arg2)
        let data = b"c\x02\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x02\x00\x00\x00\x03\x00\x00\x00C\x00\x00\x00s\x0e\x00\x00\x00t\x00|\x00|\x01\x83\x02\x01\x00d\x00S\x00)\x01N)\x01z\x05print)\x02z\x04arg1z\x04arg2)\x00)\x00z\x07<stdin>z\x01f\x01\x00\x00\x00s\x02\x00\x00\x00\x0e\x00";

        let object = Code::V310(Code310 {
            argcount: 2,
            posonlyargcount: 0,
            kwonlyargcount: 0,
            nlocals: 2,
            stacksize: 3,
            flags: CodeFlags::from_bits_truncate(0x43),
            code: vec![116, 0, 124, 0, 124, 1, 131, 2, 1, 0, 100, 0, 83, 0].into(),
            consts: [Object::None].to_vec().into(),
            names: ["print".to_string().into()].to_vec().into(),
            varnames: ["arg1".to_string().into(), "arg2".to_string().into()]
                .to_vec()
                .into(),
            freevars: [].to_vec(),
            cellvars: [].to_vec(),
            filename: "<stdin>".to_string().into(),
            name: "f".to_string().into(),
            firstlineno: 1,
            lnotab: [14, 0].to_vec().into(),
        });
        let dumped = dump_bytes(Object::Code(Arc::new(object)), None, (3, 10), 4).unwrap();

        assert_eq!(data.to_vec(), dumped);
    }

    #[test]
    fn test_recompile() {
        let data = b"c\x02\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x02\x00\x00\x00\x03\x00\x00\x00C\x00\x00\x00s\x0e\x00\x00\x00t\x00|\x00|\x01\x83\x02\x01\x00d\x00S\x00)\x01N)\x01z\x05print)\x02z\x04arg1z\x04arg2)\x00)\x00z\x07<stdin>z\x01f\x01\x00\x00\x00s\x02\x00\x00\x00\x0e\x00";

        let (kind, refs) = load_bytes(data, (3, 10)).unwrap();
        let dumped = dump_bytes(kind, Some(refs), (3, 10), 4).unwrap();

        assert_eq!(data.to_vec(), dumped);
    }
}
