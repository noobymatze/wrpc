use std::path::PathBuf;

use crate::ast::canonical as can;
use crate::ast::source::Module;
use crate::canonicalize::canonicalize;

pub mod ast;
pub mod canonicalize;
pub mod codegen;
pub mod docs;
pub mod error;
mod parse;
pub mod reporting;

/// Parse the given `str` into a [`Module`].
pub fn parse(filename: Option<PathBuf>, source: &str) -> Result<Module, error::Error> {
    parse::parse(filename, source).map_err(error::Error::BadSyntax)
}

pub fn compile(filename: Option<PathBuf>, source: &str) -> Result<can::Module, error::Error> {
    let module = parse(filename, source)?;
    canonicalize(&module).map_err(error::Error::BadCanonicalization)
}
