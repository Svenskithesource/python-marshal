use std::{collections::HashMap, sync::Arc};

use num_bigint::BigInt;
use num_complex::Complex;
use num_traits::{Signed, ToPrimitive};

use crate::{Code, Kind, Object};

pub struct PyWriter {
    data: Vec<u8>,
    marshal_version: u8,
    references: HashMap<usize, Arc<Object>>,
}

impl PyWriter {
    pub fn new(references: HashMap<usize, Arc<Object>>, marshal_version: u8) -> Self {
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

        while num <= BigInt::ZERO {
            digits.push((&value & BigInt::from(0x7FFF)).to_u16().unwrap());

            value >>= 15;
        }

        self.w_long((digits.len() as i32) * if num.is_negative() { -1 } else { 1 });
        for digit in digits {
            self.w_u16(digit);
        }
    }

    fn w_string(&mut self, value: &str, as_u8: bool) {
        if as_u8 {
            self.w_u8(value.len() as u8);
        } else {
            self.w_long(value.len() as i32);
        }

        self.data.extend_from_slice(value.as_bytes());
    }

    fn w_float_bin(&mut self, value: f64) {
        self.data.extend_from_slice(&value.to_le_bytes());
    }

    fn w_float_str(&mut self, value: f64) {
        self.w_string(&value.to_string(), true);
    }

    fn w_bytes(&mut self, value: &Vec<u8>) {
        self.data.extend_from_slice(&value);
    }

    fn w_object(&mut self, obj: Option<Object>, is_ref: bool) {
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
                let num = Arc::try_unwrap(num).unwrap();
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
                let value = &*value;
                if self.marshal_version >= 4 && value.is_ascii() {
                    if value.len() <= 255 {
                        self.w_kind(Kind::ShortAscii, is_ref);
                        self.w_u8(value.len() as u8);
                        self.w_bytes(&value.clone().into_bytes());
                    } else {
                        self.w_kind(Kind::ASCII, is_ref);
                        self.w_long(value.len() as i32);
                        self.w_bytes(&value.clone().into_bytes());
                    }
                } else {
                    self.w_kind(Kind::Unicode, is_ref);
                    self.w_string(value, false);
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
                    self.w_object(Some(item.clone()), false);
                }
            }
            Some(Object::List(value)) => {
                let size = value.len();

                self.w_kind(Kind::List, is_ref);
                self.w_long(size as i32);

                for item in value.iter() {
                    self.w_object(Some((*item.clone()).clone()), false);
                }
            }
            Some(Object::Dict(value)) => {
                self.w_kind(Kind::Dict, is_ref);
                for (key, value) in value.iter() {
                    self.w_object(Some(key.clone().into()), false);
                    self.w_object(Some((*value.clone()).clone()), false);
                }

                self.w_kind(Kind::Null, is_ref); // NULL object terminated
            }
            Some(Object::Set(value)) => {
                let size = value.len();

                self.w_kind(Kind::Set, is_ref);
                self.w_long(size as i32);

                for item in value.iter() {
                    self.w_object(Some(item.clone().into()), false);
                }
            }
            Some(Object::FrozenSet(value)) => {
                let size = value.len();

                self.w_kind(Kind::FrozenSet, is_ref);
                self.w_long(size as i32);

                for item in value.iter() {
                    self.w_object(Some(item.clone().into()), false);
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
                        self.w_object(Some(Object::Bytes(value.code.clone())), false);
                        self.w_object(Some(Object::Tuple(value.consts.clone())), false);
                        self.w_object(
                            Some(Object::Tuple(Arc::new(
                                value
                                    .names
                                    .clone()
                                    .into_iter()
                                    .map(|x| Object::String(x))
                                    .collect::<Vec<_>>(),
                            ))),
                            false,
                        );
                        self.w_object(
                            Some(Object::Tuple(Arc::new(
                                value
                                    .varnames
                                    .clone()
                                    .into_iter()
                                    .map(|x| Object::String(x))
                                    .collect::<Vec<_>>(),
                            ))),
                            false,
                        );
                        self.w_object(
                            Some(Object::Tuple(Arc::new(
                                value
                                    .freevars
                                    .clone()
                                    .into_iter()
                                    .map(|x| Object::String(x))
                                    .collect::<Vec<_>>(),
                            ))),
                            false,
                        );
                        self.w_object(
                            Some(Object::Tuple(Arc::new(
                                value
                                    .cellvars
                                    .clone()
                                    .into_iter()
                                    .map(|x| Object::String(x))
                                    .collect::<Vec<_>>(),
                            ))),
                            false,
                        );
                        self.w_object(Some(Object::String(value.filename.clone())), false);
                        self.w_object(Some(Object::String(value.name.clone())), false);
                        self.w_long(value.firstlineno.try_into().unwrap());
                        self.w_object(Some(Object::Bytes(value.lnotab.clone())), false);
                    }
                }
            }
            Some(Object::LoadRef(index)) => {
                let reference = self.references.get(&index);

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
            Some(Object::WriteRef(index)) => {
                let reference = self.references.get(&index);

                match reference {
                    None => {
                        panic!("Reference not found in references list");
                    }
                    Some(reference) => {
                        self.w_object(Some((**reference).clone()), true);
                    }
                }
            }
        };
    }

    pub fn write_object(&mut self, obj: Option<Object>) -> Vec<u8> {
        self.w_object(obj, false);

        return self.data.clone();
    }
}
