use crate::Error;
use std::any::{Any, TypeId};

pub struct Request {
    type_id: TypeId,
    found: Option<Box<dyn Any>>,
}

impl Request {
    pub fn provide<T: Any>(&mut self, f: impl FnOnce() -> T) -> &mut Self {
        if TypeId::of::<T>() == self.type_id {
            self.found = Some(Box::new(f()));
        }
        self
    }
}

pub fn request<T: Any>(error: &dyn Error) -> Option<T> {
    let mut request = Request { type_id: TypeId::of::<T>(), found: None };
    error.provide(&mut request);

    if let Some(found) = request.found {
        Some(*found.downcast().expect("type id mismatch"))
    } else {
        None
    }
}
