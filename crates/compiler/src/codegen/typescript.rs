use crate::ast::{
    canonical::{Enum, Method, Module, Parameter, Property, Record, Service, Type, Variant},
    source::Name,
};
use itertools::Itertools;
use std::collections::HashSet;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Options {
    pub print: bool,
    pub output: Option<PathBuf>,
}

pub fn generate_typescript_client(module: &Module, options: &Options) -> Result<(), ()> {
    let record_package = "records".to_string();
    let models = generate_models(&record_package, module);
    let client = generate_client_and_services(&record_package, module);

    if options.print {
        println!("{}", models);
        println!("{}", client);
    }

    if let Some(out) = &options.output {
        fs::create_dir_all(out).expect("Should work.");
        let mut file =
            File::create(out.join("models.ts")).expect("Should be able to create a file");
        file.write_all(models.as_bytes()).expect("Works");

        let mut file =
            File::create(out.join("client.ts")).expect("Should be able to create a file");
        file.write_all(client.as_bytes()).expect("Works");
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

    let result_type = r#"
/**
 * A {@link Result} either represents the result of a successful computation,
 * with a value of type {@link T} or a failed computation with an error of
 * type {@link E}.
 */
export type Result<T, E>
    = { type: "Ok"; value: T; }
    | { type: "Err"; error: E; };
    "#;

    format!("{result_type}\n{records}\n\n{enums}\n")
}

fn generate_client_and_services(package: &String, module: &Module) -> String {
    let interfaces = module
        .services
        .iter()
        .map(|(_, service)| generate_service_interface(&package, service))
        .collect::<Vec<String>>()
        .join("\n\n");

    let mut imports = find_used_types(module).iter().join(", ");

    [
        format!("import {{ {imports} }} from './models.ts';\n"),
        format!("{}\n", interfaces),
        format!("{}\n", generate_client(&package, module)),
        format!("{}", generate_service(&package, module)),
        format!("{}", generate_rpc_fn()),
    ]
    .join("\n")
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
        Type::Ref(name) => {
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
    let properties = record
        .properties
        .iter()
        .map(|property| generate_property("    ", package, property, false))
        .collect::<Vec<String>>()
        .join("\n");

    let name = record.name.value.clone();
    let class = format!("export type {name} = {{\n{properties}\n}}",);

    let doc_comment = generate_doc_comment("", &record.comment);
    format!("{doc_comment}{class}")
}

fn generate_property(indent: &str, package: &String, property: &Property, is_enum: bool) -> String {
    let name = property.name.value.clone();
    let type_ = generate_type_ref(package, &property.type_);
    format!("{indent}{name}: {type_},")
}

fn generate_enum(package: &String, record: &Enum) -> String {
    let is_sealed = record
        .variants
        .iter()
        .any(|variant| !variant.properties.is_empty());

    let variants = record
        .variants
        .iter()
        .map(|variant| generate_variant(package, &record.name, variant, is_sealed))
        .collect::<Vec<String>>()
        .join("\n");

    let doc_comment = generate_doc_comment("", &record.comment);

    let class = if is_sealed {
        let name = record.name.value.clone();
        format!("export type {name} = \n{variants}")
    } else {
        let name = record.name.value.clone();
        format!("export enum {name} {{\n{variants}\n}}")
    };

    format!("{doc_comment}{class}")
}

fn generate_variant(
    package: &String,
    parent_name: &Name,
    variant: &Variant,
    is_sealed: bool,
) -> String {
    //let doc_comment = generate_doc_comment("    ", &variant.comment);
    let variant = if is_sealed {
        generate_sealed_sub_class(package, parent_name, variant)
    } else {
        let name = variant.name.value.clone();
        format!("    {name} = '{name}',")
    };

    format!("{variant}")
}

fn generate_sealed_sub_class(package: &String, parent_name: &Name, variant: &Variant) -> String {
    let properties = variant
        .properties
        .iter()
        .map(|property| generate_property("", package, property, true))
        .collect::<Vec<String>>()
        .join(" ");

    let name = variant.name.value.clone();
    format!("    | {{ type: '{name}', {properties} }}")
}

fn generate_client(package: &String, module: &Module) -> String {
    let client_type = module
        .services
        .iter()
        .map(|(_, service)| {
            let name = service.name.uncapitalized();
            let service_client = service.name.value.clone();
            format!("    {name}: {service_client},")
        })
        .collect::<Vec<String>>()
        .join("\n");
    let comment = generate_doc_comment("", &Some("Represents a Client, which can be used to work with the corresponding server instance.".to_string()));
    format!("{comment}export type Client = {{\n{client_type}\n}}")
}

fn generate_service_interface(package: &String, service: &Service) -> String {
    let methods = service
        .methods
        .iter()
        .map(|(_, method)| generate_method_signature(package, &method))
        .collect::<Vec<String>>()
        .join("\n");

    let requests = service
        .methods
        .iter()
        .map(|(_, method)| generate_request(package, &method))
        .collect::<Vec<String>>()
        .join("\n\n");

    let comment = generate_doc_comment("", &service.comment);
    let name = service.name.capitalized();
    [
        format!("{requests}"),
        format!("{comment}export interface {name} {{\n{methods}\n}}"),
    ]
    .join("\n\n")
}

fn generate_method_signature(package: &String, method: &Method) -> String {
    let name = method.name.uncapitalized();
    let request_name = method.name.request_name();
    let return_type = method
        .return_type
        .as_ref()
        .map(|type_| generate_type_ref(package, &type_))
        .unwrap_or("void".to_string());

    let comment = generate_doc_comment("    ", &method.comment);
    format!(
        "{comment}    {name}: (params: {request_name}) => Promise<HttpResponse<{return_type}>>;"
    )
}

fn generate_param_property(indent: &str, package: &String, property: &Parameter) -> String {
    let name = property.name.value.clone();
    let type_ = generate_type_ref(package, &property.type_);
    format!("{indent}{name}: {type_},")
}

fn generate_service(package: &String, module: &Module) -> String {
    let impls = module
        .services
        .iter()
        .map(|(_, service)| {
            let name = service.name.uncapitalized();
            let methods = service
                .methods
                .iter()
                .map(|(_, method)| generate_method(package, service, &method))
                .collect::<Vec<String>>()
                .join(",\n");

            format!("        {name}: {{\n{methods}\n        }}")
        })
        .collect::<Vec<String>>()
        .join(",\n");

    format!(
        "export function client(baseUrl: string): Client {{\n    return {{\n{impls}\n    }}; \n}}"
    )
}

fn generate_request(package: &String, method: &Method) -> String {
    let name = method.name.capitalized();
    let request_name = method.name.request_name();
    let properties = method
        .parameters
        .iter()
        .map(|property| generate_param_property("    ", package, property))
        .collect::<Vec<String>>()
        .join("\n");

    format!("export type {request_name} = {{\n{properties}\n}}")
}

fn generate_method(package: &String, service: &Service, method: &Method) -> String {
    let service_name = service.name.value.clone();
    let method_name = method.name.value.clone();
    let name = method.name.uncapitalized();
    let request_name = method.name.request_name();

    let return_type = method
        .return_type
        .as_ref()
        .map(|type_| generate_type_ref(package, &type_))
        .unwrap_or("void".to_string());

    let indent = "            ";
    format!(
        "{indent}{name}: request<{request_name}, {return_type}>(baseUrl, \"/{service_name}/{method_name}\")"
    )
}

fn generate_doc_comment(indent: &str, comment: &Option<String>) -> String {
    match comment {
        None => "".to_string(),
        Some(comment) => {
            let content = comment
                .split("\n")
                .map(|line| format!("{indent} * {line}"))
                .collect::<Vec<String>>()
                .join("\n");
            format!("{indent}/**\n{content}\n{indent} */\n")
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
        Type::Ref(name) => name.clone(),
    }
}

fn generate_rpc_fn() -> String {
    r#"
/**
 * Represents an http response.
 */
export type HttpResponse<T>
    = { type: 'Ok'; value: T; }
    | { type: 'Err'; error: HttpError; }

/**
 * Represents any error, that could happen during a request.
 */
export type HttpError
    = { type: 'Network', }
    | { type: 'Timeout', }
    | { type: 'BadUrl', }
    | { type: 'BadStatus', headers: Headers, body: string }
    | { type: 'BadBody', };

/**
 * Returns a function, that can be used to call the given method
 * for an rpc.
 *
 * @param baseUrl
 * @param path
 */
function request<Params, Ret>(
    baseUrl: string,
    path: string,
): (params: Params) => Promise<HttpResponse<Ret>> {
    return async (params) => {
        try {
            const response = await fetch(`${baseUrl}${path}`, {
                method: "POST",
                body: JSON.stringify(params),
                headers: {
                    "Content-Type": "application/json",
                },
            });

            try {
                if (!response.ok) {
                    const statusCode = response.status;
                    const body = await response.text();
                    const headers = response.headers;
                    return {type: 'Err', error: {type: 'BadStatus', statusCode, headers, body}};
                }

                const value = await response.json();
                return {type: 'Ok', value };
            } catch (error) {
                return {type: 'Err', error: {type: 'BadBody'}};
            }
        } catch (error) {
            if (error instanceof DOMException && error.message === 'Timeout') {
                return {type: 'Err', error: {type: 'Timeout'}};
            }

            return {type: 'Err', error: {type: 'Network'}};
        }
    };
}
"#
    .to_string()
}
