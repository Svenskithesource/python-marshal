use std::{collections::HashMap, fs::OpenOptions, sync::Arc};

use bstr::BString;
use num_bigint::BigInt;
use num_complex::Complex;
use num_traits::{Signed, ToPrimitive};
use std::io::Write;

use crate::{Code, Kind, Object};

pub struct PyWriter {
    data: Vec<u8>,
    marshal_version: u8,
    references: Vec<Object>,
}

impl PyWriter {
    pub fn new(references: Vec<Object>, marshal_version: u8) -> Self {
        Self {
            data: Vec::new(),
            marshal_version,
            references,
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
    fn w_PyLong(&mut self, num: BigInt) {
        let mut value = num.clone().abs();
        let mut digits: Vec<u16> = vec![];

        while value > BigInt::ZERO {
            digits.push((&value & BigInt::from(0x7FFF)).to_u16().unwrap());

            value >>= 15;
        }

        self.w_long((digits.len() as i32) * if num.is_negative() { -1 } else { 1 });

        for digit in digits {
            self.w_u16(digit);
        }
    }

    fn w_string(&mut self, value: &BString, as_u8: bool) {
        if as_u8 {
            self.w_u8(value.len() as u8);
        } else {
            self.w_long(value.len() as i32);
        }

        self.data
            .extend_from_slice(&value.iter().map(|&x| x as u8).collect::<Vec<u8>>());
    }

    fn w_float_bin(&mut self, value: f64) {
        self.data.extend_from_slice(&value.to_le_bytes());
    }

    fn w_float_str(&mut self, value: f64) {
        self.w_string(&value.to_string().into(), true);
    }

    fn w_bytes(&mut self, value: &Vec<u8>) {
        self.data.extend_from_slice(&value);
    }

    fn w_object(&mut self, obj: Option<Object>) {
        let is_ref = obj.as_ref().is_some()
            && self.references.iter().any(|x| match *x {
                Object::Float(f) => {
                    // Compare floating point numbers by their bytes
                    if let Object::Float(ref value) = *obj.as_ref().unwrap() {
                        f.to_le_bytes() == value.to_le_bytes()
                    } else {
                        false
                    }
                }
                Object::Complex(Complex { re, im }) => {
                    // Compare floating point numbers by their bytes
                    if let Object::Complex(Complex {
                        re: ref re2,
                        im: ref im2,
                    }) = *obj.as_ref().unwrap()
                    {
                        re.to_le_bytes() == re2.to_le_bytes()
                            && im.to_le_bytes() == im2.to_le_bytes()
                    } else {
                        false
                    }
                }
                _ => *x == *obj.as_ref().unwrap(),
            });

        let obj_clone = obj.clone();
        let cursor_pos = self.data.len();

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
                let num = (*num).clone();
                if num >= BigInt::from(i32::MIN) && num <= BigInt::from(i32::MAX) {
                    self.w_kind(Kind::Int, is_ref);
                    self.w_long(num.to_i32().unwrap());
                } else {
                    self.w_kind(Kind::Long, is_ref);
                    self.w_PyLong(num);
                }
            }
            Some(Object::Float(value)) => {
                if self.marshal_version > 1 {
                    self.w_kind(Kind::BinaryFloat, is_ref);
                    self.w_float_bin(value);
                } else {
                    self.w_kind(Kind::Float, is_ref);
                    self.w_float_str(value);
                }
            }
            Some(Object::Complex(Complex { re, im })) => {
                if self.marshal_version > 1 {
                    self.w_kind(Kind::BinaryComplex, is_ref);
                    self.w_float_bin(re);
                    self.w_float_bin(im);
                } else {
                    self.w_kind(Kind::Complex, is_ref);
                    self.w_float_str(re);
                    self.w_float_str(im);
                }
            }
            Some(Object::Bytes(value)) => {
                self.w_kind(Kind::String, is_ref);
                self.w_long(value.len() as i32);
                self.w_bytes(&*value);
            }
            Some(Object::String(value)) => {
                let str_value = &*&value.value;

                match value.kind {
                    Kind::ASCII | Kind::ASCIIInterned | Kind::Interned => {
                        self.w_kind(value.kind, is_ref);
                        self.w_long(str_value.len() as i32);
                        self.w_bytes(&str_value.iter().map(|&x| x as u8).collect::<Vec<u8>>());
                    }
                    Kind::ShortAscii | Kind::ShortAsciiInterned => {
                        self.w_kind(value.kind, is_ref);
                        self.w_u8(str_value.len() as u8);
                        self.w_bytes(&str_value.iter().map(|&x| x as u8).collect::<Vec<u8>>());
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
                    self.w_object(Some((**item).clone()));
                }
            }
            Some(Object::List(value)) => {
                let size = value.len();

                self.w_kind(Kind::List, is_ref);
                self.w_long(size as i32);

                for item in value.iter() {
                    self.w_object(Some((*item.clone()).clone()));
                }
            }
            Some(Object::Dict(value)) => {
                self.w_kind(Kind::Dict, is_ref);
                for (key, value) in value.iter() {
                    self.w_object(Some(key.clone().into()));
                    self.w_object(Some((*value.clone()).clone()));
                }

                self.w_kind(Kind::Null, is_ref); // NULL object terminated
            }
            Some(Object::Set(value)) => {
                let size = value.len();

                self.w_kind(Kind::Set, is_ref);
                self.w_long(size as i32);

                for item in value.iter() {
                    self.w_object(Some(item.clone().into()));
                }
            }
            Some(Object::FrozenSet(value)) => {
                let size = value.len();

                self.w_kind(Kind::FrozenSet, is_ref);
                self.w_long(size as i32);

                for item in value.iter() {
                    self.w_object(Some(item.clone().into()));
                }
            }
            Some(Object::Code(value)) => {
                let value = &*value;

                match value {
                    Code::V310(value) => {
                        self.w_kind(Kind::Code, is_ref);
                        self.w_long(value.argcount.try_into().unwrap());
                        self.w_long(value.posonlyargcount.try_into().unwrap());
                        self.w_long(value.kwonlyargcount.try_into().unwrap());
                        self.w_long(value.nlocals.try_into().unwrap());
                        self.w_long(value.stacksize.try_into().unwrap());
                        self.w_long(value.flags.bits().try_into().unwrap());
                        self.w_object(Some((*value.code).clone()));
                        self.w_object(Some((*value.consts).clone()));
                        self.w_object(Some((*value.names).clone()));
                        self.w_object(Some((*value.varnames).clone()));
                        self.w_object(Some((*value.freevars).clone()));
                        self.w_object(Some((*value.cellvars).clone()));
                        self.w_object(Some((*value.filename).clone()));
                        self.w_object(Some((*value.name).clone()));
                        self.w_long(value.firstlineno.try_into().unwrap());
                        self.w_object(Some((*value.lnotab).clone()));
                    }
                }
            }
            Some(Object::LoadRef(index)) => {
                let reference = self.references.get(index);

                match reference {
                    None => {
                        panic!("Reference not found in references list");
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
                        panic!("Reference not found in references list");
                    }
                    Some(reference) => {
                        self.w_object(Some((*reference).clone()));
                    }
                }
            }
        };

        // if cfg!(test) {
        // let mut file = OpenOptions::new()
        //     .append(true)
        //     .create(true)
        //     .open("write_log.txt")
        //     .expect("Unable to open file");

        // writeln!(
        //     file,
        //     "Writing object at index {} ({}): {:?} ",
        //     cursor_pos,
        //     self.data.len(),
        //     obj_clone,
        // )
        // .expect("Unable to write to file");
        // }
    }

    pub fn write_object(&mut self, obj: Option<Object>) -> Vec<u8> {
        self.w_object(obj);

        return self.data.clone();
    }
}
