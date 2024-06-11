use crate::ast::{
    canonical::{Enum, Method, Module, Parameter, Property, Record, Service, Type, Variant},
    source::Name,
};

pub fn generate_typescript_client(module: &Module) -> Result<(), ()> {
    let record_package = "records".to_owned();
    //for decl in module.declarations.iter() {}
    for (_, record) in &module.records {
        println!("{}", generate_record(&record_package, record));
    }

    for (_, enum_value) in &module.enums {
        println!("{}", generate_enum(&record_package, enum_value));
    }

    for (_, service) in &module.services {
        println!("{}", generate_service_interface(&record_package, service));
    }

    println!("{}", generate_service(&record_package, &module));
    println!("{}", generate_client(&record_package, &module));

    let rpcFn = r#"
function rpc<Params, Ret>(
    baseUrl: string,
    path: string,
): (params: Params) => Promise<Ret> {
    return async (params) => {
    return fetch(`${baseUrl}${path}`, {
        method: "POST",
        body: JSON.stringify(params),
        headers: {
        "Content-Type": "application/json",
        },
    }).then((response) => response.json());
    };
}"#;

    println!("{}", rpcFn);

    Ok(())
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
        format!("type {name} = \n{variants}")
    } else {
        let name = record.name.value.clone();
        format!("enum {name} {{\n{variants}\n}}")
    };

    format!("{doc_comment}{class}\n")
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
    format!("export type Client = {{\n{client_type}\n}}")
}

fn generate_service_interface(package: &String, service: &Service) -> String {
    let methods = service
        .methods
        .iter()
        .map(|(_, method)| generate_method_signature(package, &method))
        .collect::<Vec<String>>()
        .join("\n");

    let name = service.name.capitalized();
    format!("export interface {name} {{\n{methods}\n}}")
}

fn generate_method_signature(package: &String, method: &Method) -> String {
    let name = method.name.uncapitalized();
    let return_type = method
        .return_type
        .as_ref()
        .map(|type_| generate_type_ref(package, &type_))
        .unwrap_or("void".to_string());

    let req = generate_request(package, method);

    format!("    {name}: (params: {req}) => Promise<{return_type}>;")
}

fn generate_request(package: &String, method: &Method) -> String {
    let name = method.name.capitalized();
    let request_name = method.name.request_name();
    let properties = method
        .parameters
        .iter()
        .map(|property| generate_param_property("", package, property))
        .collect::<Vec<String>>()
        .join("");

    format!("{{{properties}}}")
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
        "export function createClient(baseUrl: string): Client {{\n    return {{\n{impls}\n    }}; \n}}"
    )
}

fn generate_method(package: &String, service: &Service, method: &Method) -> String {
    let service_name = service.name.value.clone();
    let method_name = method.name.value.clone();
    let name = method.name.uncapitalized();
    let request_name = method.name.request_name();
    let request = generate_request(package, method);

    let return_type = method
        .return_type
        .as_ref()
        .map(|type_| generate_type_ref(package, &type_))
        .unwrap_or("void".to_string());

    let indent = "            ";
    format!(
        "{indent}{name}: rpc<{request}, {return_type}>(baseUrl, \"/{service_name}/{method_name}\")"
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
