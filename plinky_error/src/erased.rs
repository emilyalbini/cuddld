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

    pub fn format_chain(self) -> String {
        let mut parts = self.contexts;
        parts.push_back(self.inner.to_string());

        let mut source = self.inner.source();
        while let Some(inner) = source {
            parts.push_back(inner.to_string());
            source = inner.source();
        }

        let mut repr = String::new();
        if let Some(error) = parts.pop_front() {
            repr.push_str(&format!("error: {error}\n"));
        }
        for cause in parts {
            repr.push_str(&format!("  caused by: {cause}\n"));
        }
        repr
    }
}

impl std::fmt::Display for ErasedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.inner, f)
    }
}

impl std::fmt::Debug for ErasedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.inner, f)
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
