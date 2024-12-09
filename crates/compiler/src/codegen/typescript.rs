use crate::ast::{
    canonical::{Enum, Module, Record, Service, Type},
    source::Name,
};
use askama::Template;
use itertools::Itertools;
use std::collections::HashSet;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::{fs, io};

#[derive(Debug)]
pub struct Options {
    pub print: bool,
    pub output: Option<PathBuf>,
}

pub fn generate_typescript_client(module: &Module, options: &Options) -> Result<(), io::Error> {
    let record_package = "records".to_string();
    let models = generate_models(&record_package, module);
    let client = generate_client(&record_package, module);

    if options.print {
        println!("{}", models);
        println!("{}", client);
    }

    if let Some(out) = &options.output {
        fs::create_dir_all(out)?;
        let mut file = File::create(out.join("models.ts"))?;
        file.write_all(models.as_bytes())?;

        let mut file = File::create(out.join("client.ts"))?;
        file.write_all(client.as_bytes())?;
    }

    Ok(())
}

fn generate_models(package: &String, module: &Module) -> String {
    //for decl in module.declarations.iter() {}
    let records = &module
        .records
        .iter()
        .map(|(_, record)| generate_record(&package, record))
        .collect::<Vec<String>>()
        .join("\n\n");

    let enums = &module
        .enums
        .iter()
        .map(|(_, record)| generate_enum(&package, record))
        .collect::<Vec<String>>()
        .join("\n\n");

    format!("{records}\n\n{enums}\n")
}

fn find_used_types(module: &Module) -> HashSet<String> {
    let mut result = HashSet::new();
    for (_, service) in &module.services {
        for (_, method) in &service.methods {
            for param in &method.parameters {
                collect_type_names(&mut result, &param.type_);
            }

            if let Some(type_) = &method.return_type {
                collect_type_names(&mut result, type_)
            }
        }
    }

    result
}

fn collect_type_names(types: &mut HashSet<String>, type_: &Type) {
    match type_ {
        Type::Ref(name, _) => {
            types.insert(name.clone());
        }
        Type::Result(ok_type, error_type) => {
            types.insert("Result".to_string());
            collect_type_names(types, ok_type);
            collect_type_names(types, error_type);
        }
        Type::Map(key_type, value_type) => {
            collect_type_names(types, key_type);
            collect_type_names(types, value_type);
        }
        Type::List(value_type) => {
            collect_type_names(types, value_type);
        }
        Type::Option(value_type) => {
            collect_type_names(types, value_type);
        }
        Type::Set(value_type) => {
            collect_type_names(types, value_type);
        }
        _ => {}
    }
}

fn generate_record(package: &String, record: &Record) -> String {
    RecordTemplate { package, record }
        .render()
        .expect("Render generate record should work.")
}

fn generate_enum(package: &String, record: &Enum) -> String {
    EnumTemplate { package, record }
        .render()
        .expect("Should render Enum")
}

fn generate_client(package: &String, module: &Module) -> String {
    let imports = find_used_types(module).iter().join(", ");

    ServiceTemplate {
        package,
        services: &module.get_sorted_services(),
        imports: &imports,
    }
    .render()
    .expect("Should render Client")
}

fn generate_type_variables(variables: &Vec<Name>) -> String {
    if variables.is_empty() {
        return "".to_string();
    }

    let vars = variables
        .iter()
        .map(|variable| variable.value.clone())
        .join(", ");

    format!("<{vars}>")
}

fn generate_doc_comment(indent: &str, comment: &Option<String>) -> String {
    match comment {
        None => "".to_string(),
        Some(comment) => {
            let content = comment
                .split("\n")
                .map(|line| format!("{indent} * {line}"))
                .join("\n");
            format!("{indent}/**\n{content}\n{indent} */")
        }
    }
}

fn generate_type_ref(package: &String, type_: &Type) -> String {
    match type_ {
        Type::String => "string".to_string(),
        Type::Boolean => "boolean".to_string(),
        Type::Int32 => "number".to_string(),
        Type::Int64 => "number".to_string(),
        Type::Float32 => "number".to_string(),
        Type::Float64 => "number".to_string(),
        Type::Map(key_type, value_type) => {
            let key = generate_type_ref(package, key_type);
            let value = generate_type_ref(package, value_type);
            format!("{{[{key}]: {value}}}")
        }
        Type::Result(error_type, value_type) => {
            let error = generate_type_ref(package, error_type);
            let value = generate_type_ref(package, value_type);
            format!("Result<{value}, {error}>")
        }
        Type::List(value_type) => {
            let value = generate_type_ref(package, value_type);
            format!("{value}[]")
        }
        Type::Set(value_type) => {
            let value = generate_type_ref(package, value_type);
            format!("{value}[]")
        }
        Type::Option(value_type) => {
            let value = generate_type_ref(package, value_type);
            format!("{value} | undefined")
        }
        Type::Ref(name, _) => name.clone(),
    }
}

#[derive(Template)]
#[template(path = "typescript/record.ts", escape = "txt")]
struct RecordTemplate<'a> {
    record: &'a Record,
    package: &'a String,
}

#[derive(Template)]
#[template(path = "typescript/enum.ts", escape = "txt")]
struct EnumTemplate<'a> {
    record: &'a Enum,
    package: &'a String,
}

#[derive(Template)]
#[template(path = "typescript/client.ts", escape = "txt")]
struct ServiceTemplate<'a> {
    services: &'a Vec<&'a Service>,
    package: &'a String,
    imports: &'a String,
}

#[cfg(test)]
mod tests {
    use crate::{
        codegen::typescript::{EnumTemplate, RecordTemplate, ServiceTemplate},
        compile,
        error::Error,
    };
    use askama::Template;

    #[test]
    fn test() -> Result<(), Error> {
        let spec = r#"
            // Dies ist ein Test.
            data Address {
                #(check (not (blank .name)))
                street: String,
                houseNo: Int32,
                #(check
                    (or (and (= .country "DE") (= (len .zipcode) 5))
                        (and (= .country "CH") (= (len .zipcode) 4))))
                zipcode: String,
                #(check (or (= .country "DE") (= .country "CH")))
                country: String,
            }

            enum LoginResult {
                Hello { name: String },
                Test { name: String },
            }

            enum Country {
                DE,
                EN,
                CH,
            }

            // The [SessionService] manages sessions and allows a
            // user to login.
            service SessionService {

                // Signs in a user based on the given [Credentials](#Credentials).
                def login(credentials: Credentials, name: String): Greet

                // Sign out the given session via the given SessionId.
                def logout(session: SessionId): Result<Error, Greet>

            }
        "#;

        let module = compile(None, spec)?;
        let result = module.records.get("Address").expect("Get Address");
        let login_result = module.enums.get("LoginResult").expect("Get LoginResult");
        let country = module.enums.get("Country").expect("Get Country");
        let session_service = module
            .services
            .get("SessionService")
            .expect("Get SessionService");
        let package = "test".to_string();

        //println!("{}", validate_value("", "test", "errors", &vec![leq]));
        let foo = RecordTemplate {
            package: &package,
            record: result,
        }
        .render()
        .unwrap();

        println!("{}", foo);

        let foo = EnumTemplate {
            package: &package,
            record: login_result,
        }
        .render()
        .unwrap();

        println!("{}", foo);

        let foo = EnumTemplate {
            package: &package,
            record: country,
        }
        .render()
        .unwrap();

        println!("{}", foo);

        let foo = ServiceTemplate {
            package: &package,
            services: &vec![&session_service],
            imports: &"".to_string(),
        }
        .render()
        .unwrap();

        println!("{}", foo);

        Ok(())
    }
}
