use crate::Diagnostic;
use std::any::{Any, TypeId, type_name};
use std::collections::HashMap;

pub trait DiagnosticBuilder {
    fn build(&self, ctx: &GatheredContext) -> Diagnostic;
}

pub trait DiagnosticContext: Any {}

pub struct GatheredContext {
    inner: HashMap<TypeId, Box<dyn DiagnosticContext>>,
}

impl GatheredContext {
    pub fn new() -> Self {
        Self { inner: HashMap::new() }
    }

    pub fn add<T: DiagnosticContext>(&mut self, ctx: T) {
        self.add_boxed(Box::new(ctx))
    }

    pub fn add_boxed(&mut self, ctx: Box<dyn DiagnosticContext>) {
        let id = (*ctx).type_id();
        if self.inner.insert(id, ctx).is_some() {
            panic!("the same type was added twice to the context");
        }
    }

    pub fn has<T: DiagnosticContext>(&self) -> bool {
        self.inner.contains_key(&TypeId::of::<T>())
    }

    pub fn maybe<T: DiagnosticContext>(&self) -> Option<&T> {
        let id = TypeId::of::<T>();
        let any = &**self.inner.get(&id)? as &dyn Any;
        Some(any.downcast_ref().expect("we already checked the type"))
    }

    pub fn required<T: DiagnosticContext>(&self) -> &T {
        match self.maybe::<T>() {
            Some(value) => value,
            None => panic!("missing required context {}", type_name::<T>()),
        }
    }
}
