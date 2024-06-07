use std::path::PathBuf;

use crate::ast::Module;

pub mod ast;
pub mod codegen;
pub mod error;
mod parse;
pub mod reporting;

/// Parse the given `str` into a [`Module`].
pub fn parse(filename: Option<PathBuf>, str: &str) -> Result<Module, error::Error> {
    parse::parse(filename, str).map_err(error::Error::BadSyntax)
}
