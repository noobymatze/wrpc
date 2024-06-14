use crate::ast::{
    canonical::{Enum, Module, Property, Record, Type, Variant},
    source::Name,
};
use crate::codegen::command::Command;
use std::path::PathBuf;

pub mod command;
mod kotlin;
mod rust;
mod typescript;

pub fn generate(module: &Module, options: &Command) -> Result<(), ()> {
    match options {
        Command::Typescript(options) => typescript::generate_typescript_client(module, options),
        Command::Rust => rust::generate_rust_server(module),
    }
}
