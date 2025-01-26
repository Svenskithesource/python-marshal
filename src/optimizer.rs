use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

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
    removed_references: usize,
}

impl ReferenceOptimizer {
    pub fn new(references: Vec<Object>, references_used: HashSet<usize>) -> Self {
        Self {
            references,
            references_used,
            removed_references: 0,
        }
    }
}

impl Transformer for ReferenceOptimizer {
    fn visit_LoadRef(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::LoadRef(index) = obj {
            Some(Object::LoadRef(*index - self.removed_references))
        } else {
            None
        }
    }

    fn visit_StoreRef(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::StoreRef(index) = obj {
            if !self.references_used.contains(index) {
                let mut referenced_obj = self.references.get(*index).unwrap().clone();
                self.references.remove(*index);
                self.removed_references += 1;

                referenced_obj.transform(self); // Remove any nested references

                Some(referenced_obj)
            } else {
                let new_index = *index - self.removed_references;
                let mut referenced_obj = self.references.get(new_index).unwrap().clone();
                referenced_obj.transform(self);

                self.references[new_index] = referenced_obj; // Make sure all indexes are updated in the references

                Some(Object::StoreRef(new_index)) // Update the index to account for removed references
            }
        } else {
            None
        }
    }

    fn visit_Code(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::Code(code) = obj {
            let code = Arc::get_mut(code).unwrap();
            match code {
                Code::V310(ref mut code) => {
                    Arc::get_mut(&mut code.code).unwrap().transform(self);
                    Arc::get_mut(&mut code.consts).unwrap().transform(self);
                    Arc::get_mut(&mut code.names).unwrap().transform(self);
                    Arc::get_mut(&mut code.varnames).unwrap().transform(self);
                    Arc::get_mut(&mut code.freevars).unwrap().transform(self);
                    Arc::get_mut(&mut code.cellvars).unwrap().transform(self);
                    Arc::get_mut(&mut code.filename).unwrap().transform(self);
                    Arc::get_mut(&mut code.name).unwrap().transform(self);
                    Arc::get_mut(&mut code.lnotab).unwrap().transform(self);
                }
            }
        }

        None
    }

    fn visit_Dict(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::Dict(dict) = obj {
            for (_, value) in Arc::get_mut(dict).unwrap().iter_mut() {
                Arc::get_mut(value).unwrap().transform(self);
            }
        }

        None
    }

    fn visit_FrozenSet(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::FrozenSet(set) = obj {
            for i in 0..set.len() {
                let obj = Arc::get_mut(set).unwrap().get_index_mut2(i).unwrap();
                obj.transform(self);
            }
        }

        None
    }

    fn visit_Tuple(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::Tuple(tuple) = obj {
            for obj in Arc::get_mut(tuple).unwrap() {
                let obj = Arc::get_mut(obj).unwrap();
                obj.transform(self);
            }
        }

        None
    }

    fn visit_Set(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::Set(set) = obj {
            for i in 0..set.len() {
                let obj = Arc::get_mut(set).unwrap().get_index_mut2(i).unwrap();
                obj.transform(self);
            }
        }

        None
    }

    fn visit_List(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::List(list) = obj {
            for obj in Arc::get_mut(list).unwrap().iter_mut() {
                let obj = Arc::get_mut(obj).unwrap();
                obj.transform(self);
            }
        }

        None
    }

    fn visit_HashableTuple(&mut self, obj: &mut ObjectHashable) -> Option<ObjectHashable> {
        if let ObjectHashable::Tuple(tuple) = obj {
            let tuple = Arc::get_mut(tuple).unwrap();
            for obj in tuple.iter_mut() {
                obj.transform(self);
            }
        }

        None
    }

    fn visit_HashableFrozenSet(&mut self, obj: &mut ObjectHashable) -> Option<ObjectHashable> {
        if let ObjectHashable::FrozenSet(set) = obj {
            for mut obj in Arc::make_mut(set).drain() {
                obj.transform(self);
            }
        }

        None
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
            let code = Arc::get_mut(code).unwrap();
            match code {
                Code::V310(ref mut code) => {
                    Arc::get_mut(&mut code.code).unwrap().transform(self);
                    Arc::get_mut(&mut code.consts).unwrap().transform(self);
                    Arc::get_mut(&mut code.names).unwrap().transform(self);
                    Arc::get_mut(&mut code.varnames).unwrap().transform(self);
                    Arc::get_mut(&mut code.freevars).unwrap().transform(self);
                    Arc::get_mut(&mut code.cellvars).unwrap().transform(self);
                    Arc::get_mut(&mut code.filename).unwrap().transform(self);
                    Arc::get_mut(&mut code.name).unwrap().transform(self);
                    Arc::get_mut(&mut code.lnotab).unwrap().transform(self);
                }
            }
        }

        None
    }

    fn visit_Dict(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::Dict(dict) = obj {
            for (_, value) in Arc::get_mut(dict).unwrap().iter_mut() {
                Arc::get_mut(value).unwrap().transform(self);
            }
        }

        None
    }

    fn visit_FrozenSet(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::FrozenSet(set) = obj {
            for i in 0..set.len() {
                let obj = Arc::get_mut(set).unwrap().get_index_mut2(i).unwrap();
                obj.transform(self);
            }
        }

        None
    }

    fn visit_Tuple(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::Tuple(tuple) = obj {
            for obj in Arc::get_mut(tuple).unwrap() {
                let obj = Arc::get_mut(obj).unwrap();
                obj.transform(self);
            }
        }

        None
    }

    fn visit_Set(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::Set(set) = obj {
            for i in 0..set.len() {
                let obj = Arc::get_mut(set).unwrap().get_index_mut2(i).unwrap();
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
            for obj in Arc::get_mut(tuple).unwrap() {
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
