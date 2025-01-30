use std::collections::HashSet;

use hashable::HashableHashSet;
use indexmap::set::MutableValues;

use crate::{Code, Object, ObjectHashable};

#[allow(non_snake_case, unused_variables)]
pub trait Transformer {
    // Return None to keep the object as is
    fn visit(&mut self, obj: &mut Object) -> Option<Object> {
        match obj {
            Object::None => self.visit_None(obj),
            Object::StopIteration => self.visit_StopIteration(obj),
            Object::Ellipsis => self.visit_Ellipsis(obj),
            Object::Bool(_) => self.visit_Bool(obj),
            Object::Long(_) => self.visit_Long(obj),
            Object::Float(_) => self.visit_Float(obj),
            Object::Complex(_) => self.visit_Complex(obj),
            Object::Bytes(_) => self.visit_Bytes(obj),
            Object::String(_) => self.visit_String(obj),
            Object::Tuple(_) => self.visit_Tuple(obj),
            Object::List(_) => self.visit_List(obj),
            Object::Dict(_) => self.visit_Dict(obj),
            Object::Set(_) => self.visit_Set(obj),
            Object::FrozenSet(_) => self.visit_FrozenSet(obj),
            Object::Code(_) => self.visit_Code(obj),
            Object::LoadRef(_) => self.visit_LoadRef(obj),
            Object::StoreRef(_) => self.visit_StoreRef(obj),
        }
    }

    fn visit_None(&mut self, obj: &mut Object) -> Option<Object> {
        None
    }

    fn visit_StopIteration(&mut self, obj: &mut Object) -> Option<Object> {
        None
    }

    fn visit_Ellipsis(&mut self, obj: &mut Object) -> Option<Object> {
        None
    }

    fn visit_Bool(&mut self, obj: &mut Object) -> Option<Object> {
        None
    }

    fn visit_Long(&mut self, obj: &mut Object) -> Option<Object> {
        None
    }

    fn visit_Float(&mut self, obj: &mut Object) -> Option<Object> {
        None
    }

    fn visit_Complex(&mut self, obj: &mut Object) -> Option<Object> {
        None
    }

    fn visit_Bytes(&mut self, obj: &mut Object) -> Option<Object> {
        None
    }

    fn visit_String(&mut self, obj: &mut Object) -> Option<Object> {
        None
    }

    fn visit_Tuple(&mut self, obj: &mut Object) -> Option<Object> {
        None
    }

    fn visit_List(&mut self, obj: &mut Object) -> Option<Object> {
        None
    }

    fn visit_Dict(&mut self, obj: &mut Object) -> Option<Object> {
        None
    }

    fn visit_Set(&mut self, obj: &mut Object) -> Option<Object> {
        None
    }

    fn visit_FrozenSet(&mut self, obj: &mut Object) -> Option<Object> {
        None
    }

    fn visit_Code(&mut self, obj: &mut Object) -> Option<Object> {
        None
    }

    fn visit_LoadRef(&mut self, obj: &mut Object) -> Option<Object> {
        None
    }

    fn visit_StoreRef(&mut self, obj: &mut Object) -> Option<Object> {
        None
    }

    fn visit_Hashable(&mut self, obj: &mut ObjectHashable) -> Option<ObjectHashable> {
        match obj {
            ObjectHashable::None => self.visit_HashableNone(obj),
            ObjectHashable::StopIteration => self.visit_HashableStopIteration(obj),
            ObjectHashable::Ellipsis => self.visit_HashableEllipsis(obj),
            ObjectHashable::Bool(_) => self.visit_HashableBool(obj),
            ObjectHashable::Long(_) => self.visit_HashableLong(obj),
            ObjectHashable::Float(_) => self.visit_HashableFloat(obj),
            ObjectHashable::Complex(_) => self.visit_HashableComplex(obj),
            ObjectHashable::Bytes(_) => self.visit_HashableBytes(obj),
            ObjectHashable::String(_) => self.visit_HashableString(obj),
            ObjectHashable::Tuple(_) => self.visit_HashableTuple(obj),
            ObjectHashable::FrozenSet(_) => self.visit_HashableFrozenSet(obj),
            ObjectHashable::LoadRef(_) => self.visit_HashableLoadRef(obj),
            ObjectHashable::StoreRef(_) => self.visit_HashableStoreRef(obj),
        }
    }

    fn visit_HashableNone(&mut self, obj: &mut ObjectHashable) -> Option<ObjectHashable> {
        None
    }

    fn visit_HashableStopIteration(&mut self, obj: &mut ObjectHashable) -> Option<ObjectHashable> {
        None
    }

    fn visit_HashableEllipsis(&mut self, obj: &mut ObjectHashable) -> Option<ObjectHashable> {
        None
    }

    fn visit_HashableBool(&mut self, obj: &mut ObjectHashable) -> Option<ObjectHashable> {
        None
    }

    fn visit_HashableLong(&mut self, obj: &mut ObjectHashable) -> Option<ObjectHashable> {
        None
    }

    fn visit_HashableFloat(&mut self, obj: &mut ObjectHashable) -> Option<ObjectHashable> {
        None
    }

    fn visit_HashableComplex(&mut self, obj: &mut ObjectHashable) -> Option<ObjectHashable> {
        None
    }

    fn visit_HashableBytes(&mut self, obj: &mut ObjectHashable) -> Option<ObjectHashable> {
        None
    }

    fn visit_HashableString(&mut self, obj: &mut ObjectHashable) -> Option<ObjectHashable> {
        None
    }

    fn visit_HashableTuple(&mut self, obj: &mut ObjectHashable) -> Option<ObjectHashable> {
        None
    }

    fn visit_HashableFrozenSet(&mut self, obj: &mut ObjectHashable) -> Option<ObjectHashable> {
        None
    }

    fn visit_HashableLoadRef(&mut self, obj: &mut ObjectHashable) -> Option<ObjectHashable> {
        None
    }

    fn visit_HashableStoreRef(&mut self, obj: &mut ObjectHashable) -> Option<ObjectHashable> {
        None
    }
}

impl Object {
    pub fn transform(&mut self, transformer: &mut impl Transformer) {
        if let Some(new_obj) = transformer.visit(self) {
            *self = new_obj;
        }
    }
}

impl ObjectHashable {
    pub fn transform(&mut self, transformer: &mut impl Transformer) {
        if let Some(new_obj) = transformer.visit_Hashable(self) {
            *self = new_obj;
        }
    }
}

pub struct ReferenceOptimizer {
    pub references: Vec<Object>,
    pub references_used: HashSet<usize>,
    removed_references: HashSet<usize>,
}

impl ReferenceOptimizer {
    pub fn new(references: Vec<Object>, references_used: HashSet<usize>) -> Self {
        Self {
            references,
            references_used,
            removed_references: HashSet::new(),
        }
    }
}

impl Transformer for ReferenceOptimizer {
    fn visit_LoadRef(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::LoadRef(index) = obj {
            if self.removed_references.iter().any(|i| *i < *index) {
                let new_index = *index
                    - self
                        .removed_references
                        .iter()
                        .filter(|i| *i < index)
                        .count();

                Some(Object::LoadRef(new_index)) // Update the index to account for removed references
            } else {
                None
            }
        } else {
            None
        }
    }

    fn visit_StoreRef(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::StoreRef(index) = obj {
            if !self.references_used.contains(index) {
                let new_index = *index
                    - self
                        .removed_references
                        .iter()
                        .filter(|i| *i < index)
                        .count();

                let mut referenced_obj = self.references.get(new_index).unwrap().clone();
                self.references.remove(new_index);
                self.removed_references.insert(*index);

                referenced_obj.transform(self); // Remove any nested references

                Some(referenced_obj)
            } else if self.removed_references.iter().any(|i| *i < *index) {
                let new_index = *index
                    - self
                        .removed_references
                        .iter()
                        .filter(|i| *i < index)
                        .count();

                let mut referenced_obj = self.references.get(new_index).unwrap().clone();
                referenced_obj.transform(self);

                self.references[new_index] = referenced_obj; // Make sure all indexes are updated in the references

                Some(Object::StoreRef(new_index)) // Update the index to account for removed references
            } else {
                None
            }
        } else {
            None
        }
    }

    fn visit_Code(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::Code(code) = obj {
            match **code {
                Code::V310(ref mut code) => {
                    code.code.transform(self);
                    code.consts.transform(self);
                    code.names.transform(self);
                    code.varnames.transform(self);
                    code.freevars.transform(self);
                    code.cellvars.transform(self);
                    code.filename.transform(self);
                    code.name.transform(self);
                    code.lnotab.transform(self);
                }
                Code::V311(ref mut code) | Code::V312(ref mut code) | Code::V313(ref mut code) => {
                    code.code.transform(self);
                    code.consts.transform(self);
                    code.names.transform(self);
                    code.localsplusnames.transform(self);
                    code.localspluskinds.transform(self);
                    code.filename.transform(self);
                    code.name.transform(self);
                    code.qualname.transform(self);
                    code.linetable.transform(self);
                    code.exceptiontable.transform(self);
                }
            }
        }

        None
    }

    fn visit_Dict(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::Dict(dict) = obj {
            for (_, value) in dict.iter_mut() {
                value.transform(self);
            }
        }

        None
    }

    fn visit_FrozenSet(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::FrozenSet(set) = obj {
            for i in 0..set.len() {
                let obj = set.get_index_mut2(i).unwrap();
                obj.transform(self);
            }
        }

        None
    }

    fn visit_Tuple(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::Tuple(tuple) = obj {
            for obj in tuple {
                obj.transform(self);
            }
        }

        None
    }

    fn visit_Set(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::Set(set) = obj {
            for i in 0..set.len() {
                let obj = set.get_index_mut2(i).unwrap();
                obj.transform(self);
            }
        }

        None
    }

    fn visit_List(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::List(list) = obj {
            for obj in list.iter_mut() {
                obj.transform(self);
            }
        }

        None
    }

    fn visit_HashableTuple(&mut self, obj: &mut ObjectHashable) -> Option<ObjectHashable> {
        if let ObjectHashable::Tuple(tuple) = obj {
            for obj in tuple.iter_mut() {
                obj.transform(self);
            }
        }

        None
    }

    fn visit_HashableFrozenSet(&mut self, obj: &mut ObjectHashable) -> Option<ObjectHashable> {
        if let ObjectHashable::FrozenSet(set) = obj {
            let mut new_set = HashableHashSet::new();
            for obj in set.iter() {
                let mut obj = obj.clone();
                obj.transform(self);
                new_set.insert(obj);
            }

            Some(ObjectHashable::FrozenSet(new_set))
        } else {
            None
        }
    }
}

struct ReferenceCounter {
    pub references: Vec<Object>,
    pub references_used: HashSet<usize>, // Indexes of references that are used
}

impl ReferenceCounter {
    pub fn new(references: Vec<Object>) -> Self {
        Self {
            references,
            references_used: HashSet::new(),
        }
    }
}

impl Transformer for ReferenceCounter {
    fn visit_LoadRef(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::LoadRef(index) = obj {
            self.references_used.insert(*index);
        }

        None
    }

    fn visit_HashableLoadRef(&mut self, obj: &mut ObjectHashable) -> Option<ObjectHashable> {
        if let ObjectHashable::LoadRef(index) = obj {
            self.references_used.insert(*index);
        }

        None
    }

    fn visit_StoreRef(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::StoreRef(index) = obj {
            if let Some(resolved_obj) = self.references.get_mut(*index) {
                let mut temp_obj = resolved_obj.clone();
                temp_obj.transform(self);
            }
        }

        None
    }

    fn visit_HashableStoreRef(&mut self, obj: &mut ObjectHashable) -> Option<ObjectHashable> {
        if let ObjectHashable::StoreRef(index) = obj {
            if let Some(resolved_obj) = self.references.get_mut(*index) {
                let mut temp_obj = resolved_obj.clone();
                temp_obj.transform(self);
            }
        }

        None
    }

    fn visit_Code(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::Code(code) = obj {
            match **code {
                Code::V310(ref mut code) => {
                    code.code.transform(self);
                    code.consts.transform(self);
                    code.names.transform(self);
                    code.varnames.transform(self);
                    code.freevars.transform(self);
                    code.cellvars.transform(self);
                    code.filename.transform(self);
                    code.name.transform(self);
                    code.lnotab.transform(self);
                }
                Code::V311(ref mut code) | Code::V312(ref mut code) | Code::V313(ref mut code) => {
                    code.code.transform(self);
                    code.consts.transform(self);
                    code.names.transform(self);
                    code.localsplusnames.transform(self);
                    code.localspluskinds.transform(self);
                    code.filename.transform(self);
                    code.name.transform(self);
                    code.qualname.transform(self);
                    code.linetable.transform(self);
                    code.exceptiontable.transform(self);
                }
            }
        }

        None
    }

    fn visit_Dict(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::Dict(dict) = obj {
            for (_, value) in dict.iter_mut() {
                value.transform(self);
            }
        }

        None
    }

    fn visit_FrozenSet(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::FrozenSet(set) = obj {
            for i in 0..set.len() {
                let obj = set.get_index_mut2(i).unwrap();
                obj.transform(self);
            }
        }

        None
    }

    fn visit_Tuple(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::Tuple(tuple) = obj {
            for obj in tuple {
                obj.transform(self);
            }
        }

        None
    }

    fn visit_Set(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::Set(set) = obj {
            for i in 0..set.len() {
                let obj = set.get_index_mut2(i).unwrap();
                obj.transform(self);
            }
        }

        None
    }

    fn visit_List(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::List(list) = obj {
            for obj in list.iter() {
                let mut obj = (**obj).clone();
                obj.transform(self);
            }
        }

        None
    }

    fn visit_HashableTuple(&mut self, obj: &mut ObjectHashable) -> Option<ObjectHashable> {
        if let ObjectHashable::Tuple(tuple) = obj {
            for obj in tuple.iter_mut() {
                obj.transform(self);
            }
        }

        None
    }

    fn visit_HashableFrozenSet(&mut self, obj: &mut ObjectHashable) -> Option<ObjectHashable> {
        if let ObjectHashable::FrozenSet(set) = obj {
            for obj in set.iter() {
                let mut obj = obj.clone();
                obj.transform(self);
            }
        }

        None
    }
}

pub fn get_used_references(obj: &mut Object, references: Vec<Object>) -> HashSet<usize> {
    let mut counter = ReferenceCounter::new(references);

    obj.transform(&mut counter);

    counter.references_used
}
