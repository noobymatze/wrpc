use std::path::PathBuf;

use error::{syntax, Error};
use reporting::WrpcDocBuilder;

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

/// Print all given errors to the terminal.
pub fn print_errors(filename: &PathBuf, str: &str, error: Error) {
    match error {
        Error::BadSyntax(errors) => {
            let alloc = WrpcDocBuilder::new(str);
            for error in errors {
                match error {
                    syntax::Error::ParseError(error) => {
                        let report = error.to_report(&alloc);
                        println!(
                            "\x1b[31m{}\x1b[0m\n",
                            report.render(&Some(filename.clone()), reporting::Target::Terminal)
                        );
                    }
                }
            }
        }
        Error::BadCanonicalization(error) => {
            println!("Bad canonicalization happened: {error:?}");
        }
    }
}
