use crate::Request;
use std::any::Any;
use std::fmt::{Debug, Display};
use std::num::{ParseIntError, TryFromIntError};
use std::string::FromUtf8Error;

pub trait Error: Debug + Display + Send + Sync + Any + 'static {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    fn provide(&self, _request: &mut Request) {}
}

impl<T: Error> Error for Box<T> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        (self as &T).source()
    }

    fn provide<'a>(&self, request: &mut Request) {
        (self as &T).provide(request)
    }
}

impl<E: Error> From<E> for Box<dyn Error> {
    fn from(error: E) -> Self {
        Box::new(error)
    }
}

impl dyn Error {
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        (self as &dyn Any).downcast_ref()
    }
}

impl Error for String {}

impl Error for std::io::Error {}

impl Error for TryFromIntError {}

impl Error for ParseIntError {}

impl Error for FromUtf8Error {}
