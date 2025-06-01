use crate::{
    error::Error,
    optimize_references,
    optimizer::{Transformable, Transformer},
    Object, ObjectHashable,
};

/// Checks if there are any recursive references in the given object or the ones it references.
struct RecursiveCheck {
    references: Vec<Object>,
    recursive_refs: Vec<usize>,
    /// Stack to keep track of the current references being visited.
    ref_stack: Vec<usize>,
}

impl RecursiveCheck {
    pub fn new(references: Vec<Object>) -> Self {
        Self {
            references,
            recursive_refs: Vec::new(),
            ref_stack: Vec::new(),
        }
    }
}

impl Transformer for RecursiveCheck {
    fn visit_LoadRef(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::LoadRef(index) = obj {
            if self.ref_stack.contains(index) {
                self.recursive_refs.push(*index);
            } else {
                self.ref_stack.push(*index);
                let mut obj = self.references[*index].clone();
                self.visit(&mut obj);
                self.ref_stack.pop();
            }
        }

        None
    }

    fn visit_StoreRef(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::StoreRef(index) = obj {
            if !self.ref_stack.contains(index) {
                self.ref_stack.push(*index);
                let mut obj = self.references[*index].clone();
                self.visit(&mut obj);
                self.ref_stack.pop();
            } else {
                panic!("Recursive reference in StoreRef");
            }
        }

        None
    }

    fn visit_HashableLoadRef(&mut self, obj: &mut ObjectHashable) -> Option<ObjectHashable> {
        if let ObjectHashable::LoadRef(index) = obj {
            if self.ref_stack.contains(index) {
                self.recursive_refs.push(*index);
            } else {
                self.ref_stack.push(*index);
                let mut obj = self.references[*index].clone();
                self.visit(&mut obj);
                self.ref_stack.pop();
            }
        }

        None
    }

    fn visit_HashableStoreRef(&mut self, obj: &mut ObjectHashable) -> Option<ObjectHashable> {
        if let ObjectHashable::StoreRef(index) = obj {
            if !self.ref_stack.contains(index) {
                self.ref_stack.push(*index);
                let mut obj = self.references[*index].clone();
                self.visit(&mut obj);
                self.ref_stack.pop();
            } else {
                panic!("Recursive reference in StoreRef");
            }
        }

        None
    }
}

/// Replaces LoadRef and StoreRef with the actual referenced objects. For any pyc file this should replace all references as it is not possible to have a recursive reference in a pyc file that isn't specifically crafted to do so.
struct Resolver {
    references: Vec<Object>,
    recursive_refs: Vec<usize>,
}

impl Resolver {
    pub fn new(references: Vec<Object>, recursive_refs: Vec<usize>) -> Self {
        Self {
            references,
            recursive_refs,
        }
    }
}

impl Transformer for Resolver {
    fn visit_LoadRef(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::LoadRef(index) = obj {
            if !self.recursive_refs.contains(index) {
                Some(self.references[*index].clone())
            } else {
                None
            }
        } else {
            None
        }
    }

    fn visit_StoreRef(&mut self, obj: &mut Object) -> Option<Object> {
        if let Object::StoreRef(index) = obj {
            if !self.recursive_refs.contains(index) {
                let mut obj = self.references[*index].clone();
                obj.transform(self);

                self.references[*index] = obj;
            }
        }

        None
    }

    fn visit_HashableLoadRef(&mut self, obj: &mut ObjectHashable) -> Option<ObjectHashable> {
        if let ObjectHashable::LoadRef(index) = obj {
            if !self.recursive_refs.contains(index) {
                ObjectHashable::from_ref(self.references[*index].clone(), &self.references).ok()
            } else {
                None
            }
        } else {
            None
        }
    }

    fn visit_HashableStoreRef(&mut self, obj: &mut ObjectHashable) -> Option<ObjectHashable> {
        if let ObjectHashable::StoreRef(index) = obj {
            if !self.recursive_refs.contains(index) {
                let mut obj = self.references[*index].clone();
                obj.transform(self);

                self.references[*index] = obj;
            }
        }

        None
    }
}

/// Returns a list of indices of all recursive references in the given object and its references.
pub fn get_recursive_refs(obj: Object, references: Vec<Object>) -> Result<Vec<usize>, Error> {
    let mut checker = RecursiveCheck::new(references);

    let mut obj = obj.clone();

    obj.transform(&mut checker);

    Ok(checker.recursive_refs)
}

/// Attempts to resolve all references in the given object and its references. This will remove all unused references and resolve all non-recursively stored references.
/// If there are any recursive references, they will be left as LoadRef or StoreRef objects and included in the returned references.
pub fn resolve_all_refs(
    obj: Object,
    references: Vec<Object>,
) -> Result<(Object, Vec<Object>), Error> {
    let (optimized_obj, optimized_refs) = optimize_references(obj, references); // Remove all unused references

    // Resolve all non-recursively stored references
    let recursive_refs = get_recursive_refs(optimized_obj.clone(), optimized_refs.clone())?;

    let mut resolver = Resolver::new(optimized_refs.clone(), recursive_refs);

    let mut obj = optimized_obj.clone();

    obj.transform(&mut resolver);

    let (obj, resolved_refs) = optimize_references(obj, resolver.references); // Clean up leftover references

    Ok((obj, resolved_refs))
}
