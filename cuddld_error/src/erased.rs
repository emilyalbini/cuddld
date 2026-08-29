use std::collections::VecDeque;
use std::error::Error as StdError;

pub struct ErasedError {
    inner: Box<dyn StdError + Send + Sync + 'static>,
    contexts: VecDeque<String>,
}

impl ErasedError {
    pub fn from_boxed_error(inner: Box<dyn StdError + Send + Sync + 'static>) -> Self {
        ErasedError { inner, contexts: VecDeque::new() }
    }

    fn parts(&self) -> Vec<String> {
        let mut parts = self.contexts.iter().cloned().collect::<Vec<_>>();
        parts.push(self.inner.to_string());

        let mut source = self.inner.source();
        while let Some(inner) = source {
            parts.push(inner.to_string());
            source = inner.source();
        }

        parts
    }

    pub fn format_chain(&self) -> String {
        let mut parts = self.parts();
        let mut repr = String::new();
        if let Some(error) = parts.pop() {
            repr.push_str(&format!("error: {error}\n"));
        }
        for cause in parts {
            repr.push_str(&format!("  caused by: {cause}\n"));
        }
        repr
    }

    fn format_compact(&self) -> String {
        let mut repr = String::new();
        for part in self.parts() {
            if !repr.is_empty() {
                repr.push_str(": ");
            }
            repr.push_str(&part);
        }
        repr
    }
}

impl std::fmt::Display for ErasedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.format_compact())
    }
}

impl std::fmt::Debug for ErasedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.format_compact())
    }
}

impl<E> From<E> for ErasedError
where
    E: StdError + Send + Sync + 'static,
{
    fn from(value: E) -> Self {
        ErasedError { inner: Box::new(value), contexts: VecDeque::new() }
    }
}

pub trait ErasedContext<T> {
    fn with_context(self, context: impl FnOnce() -> String) -> Result<T, ErasedError>;
    fn context(self, context: impl Into<String>) -> Result<T, ErasedError>;
}

impl<T, E> ErasedContext<T> for Result<T, E>
where
    E: StdError + Send + Sync + 'static,
{
    fn context(self, context: impl Into<String>) -> Result<T, ErasedError> {
        self.with_context(|| context.into())
    }

    fn with_context(self, context: impl FnOnce() -> String) -> Result<T, ErasedError> {
        match self {
            Ok(ok) => Ok(ok),
            Err(source) => {
                Err(ErasedError { inner: source.into(), contexts: VecDeque::from([context()]) })
            }
        }
    }
}

impl<T> ErasedContext<T> for Result<T, ErasedError> {
    fn context(self, context: impl Into<String>) -> Result<T, ErasedError> {
        self.with_context(|| context.into())
    }

    fn with_context(self, context: impl FnOnce() -> String) -> Result<T, ErasedError> {
        match self {
            Ok(ok) => Ok(ok),
            Err(mut err) => {
                err.contexts.push_front(context());
                Err(err)
            }
        }
    }
}

#[macro_export]
macro_rules! erased {
    ($($tt:tt)*) => {
        $crate::ErasedError::from_boxed_error(format!($($tt)*).into())
    }
}

#[macro_export]
macro_rules! bail {
    ($($tt:tt)*) => {
        return Err($crate::ErasedError::from_boxed_error(format!($($tt)*).into()))
    }
}
