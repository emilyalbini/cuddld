pub mod ast;
mod deserialize;
mod functions;
mod lexer;
mod builtins;
mod parser;
mod resolve;

pub use deserialize::{FromHcl, FromHclString, HclDeserializer};
pub use parser::parse_picohcl;
pub(crate) use resolve::{HclContext, HclResolver};
