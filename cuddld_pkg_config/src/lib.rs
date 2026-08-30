#![feature(error_generic_member_access)]

mod discover;
mod lexer;
mod parser;
mod template;
mod types;

pub use crate::discover::{DiscoverError, PkgConfigEnv, discover};
pub use crate::lexer::LexError;
pub use crate::parser::ParseError;
pub use crate::types::*;
