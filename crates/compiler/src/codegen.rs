use std::io;

use crate::ast::canonical::Module;
use crate::codegen::command::Command;

pub mod command;
mod kotlin;
mod rust;
mod typescript;

pub fn generate(module: &Module, options: &Command) -> Result<(), io::Error> {
    match options {
        Command::Typescript(options) => typescript::generate_typescript_client(module, options),
        Command::Rust => rust::generate_rust_server(module),
        Command::Kotlin(options) => kotlin::generate_kotlin_server(module, options),
    }
}
