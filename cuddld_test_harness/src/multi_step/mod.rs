mod gather;
mod steps;
mod tests;

pub use self::gather::DefineSteps;
pub use self::steps::Step;
pub use tests::{Arch, TestContext};

use self::gather::{DefineStepsFn, gather};
use cuddld_error::ErasedError;
use std::path::Path;

pub fn main(path: &Path, define_steps: DefineStepsFn) -> Result<(), ErasedError> {
    let tests = gather(path, "", define_steps)?;
    crate::nextest::main(tests)
}
