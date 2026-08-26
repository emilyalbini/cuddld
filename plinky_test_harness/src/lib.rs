#![feature(test)]
#![feature(error_generic_member_access)]

extern crate test;

#[macro_use]
pub mod snapshot;
mod gather;
pub mod picohcl;
mod steps;
mod tests;
pub mod utils;

pub use crate::gather::DefineSteps;
use crate::gather::{DefineStepsFn, gather};
pub use crate::steps::Step;
use crate::utils::err_str;
use std::path::Path;
pub use tests::{Arch, TestContext};

pub fn main(path: &Path, define_steps: DefineStepsFn) {
    let args = std::env::args().collect::<Vec<_>>();
    let tests = err_str(gather(path, "", define_steps)).unwrap();
    test::test_main(&args, tests, None);
}
