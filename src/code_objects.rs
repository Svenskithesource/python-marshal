use crate::{extract_object, extract_strings_tuple, resolve_object_ref, CodeFlags, Error, Object};

/// Represents a Python code object for Python 3.10.
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
    pub filename:        Box<Object>, // Needs to contain PyString as a value or a reference
    pub name:            Box<Object>, // Needs to contain PyString as a value or a reference
    pub firstlineno:     u32,
    pub lnotab:          Box<Object>, // Needs to contain Vec<u8>, as a value or a reference
}

impl Code310 {
    pub fn new(
        argcount: u32,
        posonlyargcount: u32,
        kwonlyargcount: u32,
        nlocals: u32,
        stacksize: u32,
        flags: CodeFlags,
        code: Box<Object>,
        consts: Box<Object>,
        names: Box<Object>,
        varnames: Box<Object>,
        freevars: Box<Object>,
        cellvars: Box<Object>,
        filename: Box<Object>,
        name: Box<Object>,
        firstlineno: u32,
        lnotab: Box<Object>,
        references: &[Object],
    ) -> Result<Self, Error> {
        // Ensure all corresponding values are of the correct type
        extract_object!(Some(resolve_object_ref!(Some((*code).clone()), references)?), Object::Bytes(bytes) => bytes, Error::NullInTuple)?;
        extract_object!(Some(resolve_object_ref!(Some((*consts).clone()), references)?), Object::Tuple(objs) => objs, Error::NullInTuple)?;
        extract_strings_tuple!(
            extract_object!(Some(resolve_object_ref!(Some((*names).clone()), references)?), Object::Tuple(objs) => objs, Error::NullInTuple)?,
            references
        )?;
        extract_strings_tuple!(
            extract_object!(Some(resolve_object_ref!(Some((*varnames).clone()), references)?), Object::Tuple(objs) => objs, Error::NullInTuple)?,
            references
        )?;
        extract_strings_tuple!(
            extract_object!(Some(resolve_object_ref!(Some((*freevars).clone()), references)?), Object::Tuple(objs) => objs, Error::NullInTuple)?,
            references
        )?;
        extract_strings_tuple!(
            extract_object!(Some(resolve_object_ref!(Some((*cellvars).clone()), references)?), Object::Tuple(objs) => objs, Error::NullInTuple)?,
            references
        )?;

        extract_object!(Some(resolve_object_ref!(Some((*filename).clone()), references)?), Object::String(string) => string, Error::UnexpectedObject)?;
        extract_object!(Some(resolve_object_ref!(Some((*name).clone()), references)?), Object::String(string) => string, Error::UnexpectedObject)?;
        extract_object!(Some(resolve_object_ref!(Some((*lnotab).clone()), references)?), Object::Bytes(bytes) => bytes, Error::NullInTuple)?;

        Ok(Self {
            argcount,
            posonlyargcount,
            kwonlyargcount,
            nlocals,
            stacksize,
            flags,
            code,
            consts,
            names,
            varnames,
            freevars,
            cellvars,
            filename,
            name,
            firstlineno,
            lnotab,
        })
    }
}

/// Represents a Python code object for Python 3.11, 3.12, 3.13. They all share the same structure.
#[rustfmt::skip]
#[derive(Clone, Debug, PartialEq)]
pub struct Code311 {
    pub argcount:        u32,
    pub posonlyargcount: u32,
    pub kwonlyargcount:  u32,
    pub stacksize:       u32,
    pub flags:           CodeFlags,
    pub code:            Box<Object>, // Needs to contain Vec<u8> as a value or a reference
    pub consts:          Box<Object>, // Needs to contain Vec<Object> as a value or a reference
    pub names:           Box<Object>, // Needs to contain Vec<PyString> as a value or a reference
    pub localsplusnames: Box<Object>, // Needs to contain Vec<PyString> as a value or a reference
    pub localspluskinds: Box<Object>, // Needs to contain Vec<u8> as a value or a reference
    pub filename:        Box<Object>, // Needs to contain PyString> as a value or a reference
    pub name:            Box<Object>, // Needs to contain PyString as a value or a reference
    pub qualname:        Box<Object>, 
    pub firstlineno:     u32,
    pub linetable:       Box<Object>, 
    pub exceptiontable:  Box<Object>,
}

impl Code311 {
    pub fn new(
        argcount: u32,
        posonlyargcount: u32,
        kwonlyargcount: u32,
        stacksize: u32,
        flags: CodeFlags,
        code: Box<Object>,
        consts: Box<Object>,
        names: Box<Object>,
        localsplusnames: Box<Object>,
        localspluskinds: Box<Object>,
        filename: Box<Object>,
        name: Box<Object>,
        qualname: Box<Object>,
        firstlineno: u32,
        linetable: Box<Object>,
        exceptiontable: Box<Object>,
        references: &[Object],
    ) -> Result<Self, Error> {
        // Ensure all corresponding values are of the correct type
        extract_object!(Some(resolve_object_ref!(Some((*code).clone()), references)?), Object::Bytes(bytes) => bytes, Error::NullInTuple)?;
        extract_object!(Some(resolve_object_ref!(Some((*consts).clone()), references)?), Object::Tuple(objs) => objs, Error::NullInTuple)?;
        extract_strings_tuple!(
            extract_object!(Some(resolve_object_ref!(Some((*names).clone()), references)?), Object::Tuple(objs) => objs, Error::NullInTuple)?,
            references
        )?;
        extract_strings_tuple!(
            extract_object!(Some(resolve_object_ref!(Some((*localsplusnames).clone()), references)?), Object::Tuple(objs) => objs, Error::NullInTuple)?,
            references
        )?;
        extract_object!(Some(resolve_object_ref!(Some((*localspluskinds).clone()), references)?), Object::Bytes(bytes) => bytes, Error::NullInTuple)?;
        extract_object!(Some(resolve_object_ref!(Some((*filename).clone()), references)?), Object::String(string) => string, Error::UnexpectedObject)?;
        extract_object!(Some(resolve_object_ref!(Some((*name).clone()), references)?), Object::String(string) => string, Error::UnexpectedObject)?;
        extract_object!(Some(resolve_object_ref!(Some((*qualname).clone()), references)?), Object::String(string) => string, Error::UnexpectedObject)?;
        extract_object!(Some(resolve_object_ref!(Some((*linetable).clone()), references)?), Object::Bytes(bytes) => bytes, Error::NullInTuple)?;
        extract_object!(Some(resolve_object_ref!(Some((*exceptiontable).clone()), references)?), Object::Bytes(bytes) => bytes, Error::NullInTuple)?;

        Ok(Self {
            argcount,
            posonlyargcount,
            kwonlyargcount,
            stacksize,
            flags,
            code,
            consts,
            names,
            localsplusnames,
            localspluskinds,
            filename,
            name,
            qualname,
            firstlineno,
            linetable,
            exceptiontable,
        })
    }
}
