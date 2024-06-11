use crate::ast::{
    canonical::{Enum, Module, Property, Record, Type, Variant},
    source::Name,
};

mod kotlin;
mod rust;
mod typescript;

pub fn generate(module: &Module) -> Result<(), ()> {
    rust::generate_rust_server(module);
    typescript::generate_typescript_client(module)
}
