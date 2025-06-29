use std::collections::{HashMap, HashSet};

use hashable::HashableHashSet;
use indexmap::set::MutableValues;

use crate::{Code, Object, ObjectHashable};

/// Trait for transforming Python objects.
// TODO: Don't use Sized to fix the error
#[allow(non_snake_case, unused_variables)]
pub trait Transformer {
    /// Dispatch method to visit an object and return a transformed version. When returning `None`, the object is left unchanged.
    fn visit(&mut self, obj: &mut Object) -> Option<Object> {
        // Return None to keep the object as is
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
        if let Object::Tuple(tuple) = obj {
            for obj in tuple {
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

    fn visit_Dict(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::Dict(dict) = obj {
            for (_, value) in dict.iter_mut() {
                value.transform(self);
            }
        }

        None
    }

    fn visit_Set(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::Set(set) = obj {
            for i in 0..set.len() {
                let obj = set.get_index_mut2(i)?;
                obj.transform(self);
            }
        }

        None
    }

    fn visit_FrozenSet(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::FrozenSet(set) = obj {
            for i in 0..set.len() {
                let obj = set.get_index_mut2(i)?;
                obj.transform(self);
            }
        }

        None
    }

    fn visit_Code(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::Code(code) = obj {
            match *code {
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

    fn visit_LoadRef(&mut self, obj: &mut Object) -> Option<Object> {
        None
    }

    fn visit_StoreRef(&mut self, obj: &mut Object) -> Option<Object> {
        None
    }

    /// Same as `visit`, but for hashable objects.
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

    fn visit_HashableLoadRef(&mut self, obj: &mut ObjectHashable) -> Option<ObjectHashable> {
        None
    }

    fn visit_HashableStoreRef(&mut self, obj: &mut ObjectHashable) -> Option<ObjectHashable> {
        None
    }
}

pub trait Transformable {
    fn transform(&mut self, transformer: &mut (impl Transformer + ?Sized));
}

impl Transformable for Object {
    fn transform(&mut self, transformer: &mut (impl Transformer + ?Sized)) {
        if let Some(new_obj) = transformer.visit(self) {
            *self = new_obj;
        }
    }
}

impl Transformable for ObjectHashable {
    fn transform(&mut self, transformer: &mut (impl Transformer + ?Sized)) {
        if let Some(new_obj) = transformer.visit_Hashable(self) {
            *self = new_obj;
        }
    }
}

/// Removes unused references from a list of references and an object and updates the reference indices in the objects.
pub struct ReferenceOptimizer {
    pub references: Vec<Object>,
    pub new_references: Vec<Object>,
    pub references_used: HashSet<usize>,
    /// Map of old index to new index
    reference_map: HashMap<usize, usize>,
}

impl ReferenceOptimizer {
    pub fn new(references: Vec<Object>, references_used: HashSet<usize>) -> Self {
        Self {
            references,
            new_references: Vec::new(),
            references_used,
            reference_map: HashMap::new(),
        }
    }
}

impl Transformer for ReferenceOptimizer {
    fn visit_LoadRef(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::LoadRef(index) = obj {
            let new_index = self.reference_map.get(index)?;

            Some(Object::LoadRef(*new_index))
        } else {
            None
        }
    }

    fn visit_HashableLoadRef(&mut self, obj: &mut ObjectHashable) -> Option<ObjectHashable> {
        if let ObjectHashable::LoadRef(index) = obj {
            let new_index = self.reference_map.get(index)?;

            Some(ObjectHashable::LoadRef(*new_index))
        } else {
            None
        }
    }

    fn visit_StoreRef(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::StoreRef(index) = obj {
            if self.references_used.contains(index) {
                let mut obj = self.references.get(*index)?.clone();
                obj.transform(self); // Transform the object to ensure it is up-to-date

                self.new_references.push(obj);
                let new_index = self.new_references.len() - 1;
                self.reference_map.insert(*index, new_index);

                Some(Object::StoreRef(new_index))
            } else {
                let mut obj = self.references.get(*index)?.clone();
                obj.transform(self);

                Some(obj)
            }
        } else {
            None
        }
    }

    fn visit_HashableStoreRef(&mut self, obj: &mut ObjectHashable) -> Option<ObjectHashable> {
        if let ObjectHashable::StoreRef(index) = obj {
            if self.references_used.contains(index) {
                let mut obj = self.references.get(*index)?.clone();
                obj.transform(self);

                self.new_references.push(obj);
                let new_index = self.new_references.len() - 1;
                self.reference_map.insert(*index, new_index);

                Some(ObjectHashable::StoreRef(new_index))
            } else {
                let mut obj = self.references.get(*index)?.clone();
                obj.transform(self);

                ObjectHashable::from_ref(obj, &self.new_references).ok()
            }
        } else {
            None
        }
    }
}

/// Creates a set of used references from a given object and a list of references.
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
}

pub fn get_used_references(obj: &mut Object, references: Vec<Object>) -> HashSet<usize> {
    let mut counter = ReferenceCounter::new(references);

    obj.transform(&mut counter);

    counter.references_used
}
