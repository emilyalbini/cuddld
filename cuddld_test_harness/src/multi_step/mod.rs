mod gather;
mod steps;
mod tests;

pub use self::gather::DefineSteps;
use self::gather::{DefineStepsFn, gather};
pub use self::steps::Step;
pub use tests::{Arch, TestContext};

use crate::utils::err_str;
use std::path::Path;

pub fn main(path: &Path, define_steps: DefineStepsFn) {
    let args = std::env::args().collect::<Vec<_>>();
    let tests = err_str(gather(path, "", define_steps)).unwrap();
    test::test_main(&args, tests, None);
}
