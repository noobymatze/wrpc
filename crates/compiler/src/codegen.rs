use crate::ast::Module;

mod typescript;

pub fn generate_typescript_client(module: &Module) -> Result<(), ()> {
    for decl in module.declarations.iter() {}

    Ok(())
}