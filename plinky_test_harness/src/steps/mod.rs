pub(crate) mod ar;
pub(crate) mod asm;
pub(crate) mod c;
pub(crate) mod dir;
pub(crate) mod ld;
pub(crate) mod rename;
pub(crate) mod rust;

use crate::TestContext;
use plinky_error::ErasedError;
use std::fmt::Debug;

pub trait Step: Debug + Send + Sync {
    fn run(&self, ctx: TestContext<'_>) -> Result<(), ErasedError>;

    fn is_leaf() -> bool
    where
        Self: Sized,
    {
        false
    }
}
