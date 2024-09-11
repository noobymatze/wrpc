use itertools::Itertools;

use crate::ast::canonical::{
    Enum, Method, Module, Parameter, Property, Record, Service, Type, Variant,
};
use std::io;

pub fn generate_rust_server(module: &Module) -> Result<(), io::Error> {
    let record_package = "records".to_owned();
    //for decl in module.declarations.iter() {}
    for (_, record) in &module.records {
        println!("{}", generate_record(&record_package, record));
    }

    for (_, enum_value) in &module.enums {
        println!("{}", generate_enum(&record_package, enum_value));
    }

    for (_, service) in &module.services {
        println!("{}", generate_service(&record_package, service));
        println!("{}", generate_router(&record_package, service));
    }

    Ok(())
}

fn generate_service(package: &String, service: &Service) -> String {
    let methods = service
        .methods
        .iter()
        .map(|(_, method)| generate_method(package, &method))
        .collect::<Vec<String>>()
        .join("\n\n");

    let requests = service
        .methods
        .iter()
        .map(|(_, method)| generate_request(package, &method))
        .collect::<Vec<String>>()
        .join("\n\n");

    let name = service.name.value.clone();
    let async_trait = "#[async_trait]\n";
    let doc_comment = generate_doc_comment("    ", &service.comment);
    format!("{requests}\n\n{doc_comment}{async_trait}pub trait {name}: Send + Sync + 'static {{\n{methods}\n}}")
}

fn generate_request(package: &String, method: &Method) -> String {
    let request_name = method.name.request_name();
    let properties = method
        .parameters
        .iter()
        .map(|property| generate_param_property("    ", package, property))
        .collect::<Vec<String>>()
        .join("\n");

    format!("#[derive(Debug, Deserialize)]\npub struct {request_name} {{\n{properties}\n}}")
}

fn generate_method(package: &String, method: &Method) -> String {
    let request = method.name.request_name();

    let return_type = method
        .return_type
        .as_ref()
        .map(|type_| generate_type_ref(package, &type_))
        .unwrap_or("".to_string());

    let name = method.name.value.clone();
    let doc_comment = generate_doc_comment("    ", &method.comment);
    format!("{doc_comment}    async fn {name}(&self, request: {request}) -> {return_type};")
}

fn generate_router(package: &String, service: &Service) -> String {
    let methods = service
        .methods
        .iter()
        .map(|(_, method)| generate_router_method(package, &service, &method))
        .collect::<Vec<String>>()
        .join("\n\n");

    let routes = service
        .methods
        .iter()
        .map(|(_, method)| {
            let service_name = service.name.value.clone();
            let name = method.name.value.clone();
            format!("        .route(\"/{service_name}/{name}\", post({name}))")
        })
        .collect::<Vec<String>>()
        .join("\n");

    let name = service.name.value.clone();
    let layer = "        .layer(Extension(Arc::new(service)))";
    format!("pub fn router(service: impl {name}) -> Router {{\n    Router::new()\n{routes}\n{layer}\n}}\n\n{methods}")
}

fn generate_router_method(package: &String, service: &Service, method: &Method) -> String {
    let service = service.name.capitalized();
    let name = method.name.value.clone();
    let request_name = method.name.request_name();
    let return_type = method
        .return_type
        .as_ref()
        .map(|type_| format!("Json<{}>", generate_type_ref(package, &type_)))
        .unwrap_or("".to_string());
    let attributes = "#[debug_handler]\n";
    let body = [
        format!("    let result = service.{name}(request).await;"),
        format!("    Json(result)"),
    ]
    .join("\n");
    format!("{attributes}async fn {name}(Extension(service): Extension<Arc<dyn {service}>>, Json(request): Json<{request_name}>) -> {return_type} {{\n{body}\n}}")
}

fn generate_enum(package: &String, record: &Enum) -> String {
    let variants = record
        .variants
        .iter()
        .map(|variant| generate_variant(package, variant))
        .collect::<Vec<String>>()
        .join("\n");

    let doc_comment = generate_doc_comment("", &record.comment);

    let name = record.name.value.clone();
    let derives = "#[derive(Debug, Serialize, Deserialize)]\n";
    let class = format!("{derives}pub enum {name} {{\n{variants}\n}}");

    format!("{doc_comment}{class}")
}

fn generate_variant(package: &String, variant: &Variant) -> String {
    let doc_comment = generate_doc_comment("    ", &variant.comment);
    let variant = generate_sealed_sub_class(package, variant);

    format!("{doc_comment}{variant}")
}

fn generate_sealed_sub_class(package: &String, variant: &Variant) -> String {
    let properties = variant
        .properties
        .iter()
        .map(|property| generate_property("        ", package, property, true))
        .collect::<Vec<String>>()
        .join("\n");

    let properties = if !variant.properties.is_empty() {
        format!(" {{\n{properties}\n    }}")
    } else {
        "".to_string()
    };

    let name = variant.name.value.clone();
    format!("    {name}{properties},")
}

fn generate_property(indent: &str, package: &String, property: &Property, is_enum: bool) -> String {
    let name = property.name.value.clone();
    let type_ = generate_type_ref(package, &property.type_);
    let pub_mod = if !is_enum { "pub " } else { "" };
    format!("{indent}{pub_mod}{name}: {type_},")
}

fn generate_param_property(indent: &str, package: &String, property: &Parameter) -> String {
    let name = property.name.value.clone();
    let type_ = generate_type_ref(package, &property.type_);
    format!("{indent}pub {name}: {type_},")
}

fn generate_record(package: &String, record: &Record) -> String {
    let properties = record
        .properties
        .iter()
        .map(|property| generate_property("    ", package, property, false))
        .collect::<Vec<String>>()
        .join("\n");

    let name = record.name.value.clone();
    let derives = "#[derive(Debug, Serialize, Deserialize)]\n";
    let class = format!("{derives}pub struct {name} {{\n{properties}\n}}",);

    let doc_comment = generate_doc_comment("", &record.comment);
    format!("{doc_comment}{class}")
}

fn generate_type_ref(package: &String, type_: &Type) -> String {
    match type_ {
        Type::String => "String".to_string(),
        Type::Boolean => "bool".to_string(),
        Type::Int32 => "i32".to_string(),
        Type::Int64 => "i64".to_string(),
        Type::Float32 => "f32".to_string(),
        Type::Float64 => "f64".to_string(),
        Type::Map(key_type, value_type) => {
            let key = generate_type_ref(package, key_type);
            let value = generate_type_ref(package, value_type);
            format!("HashMap<{key}, {value}>")
        }
        Type::Result(error_type, value_type) => {
            let error = generate_type_ref(package, error_type);
            let value = generate_type_ref(package, value_type);
            format!("Result<{value}, {error}>")
        }
        Type::List(value_type) => {
            let value = generate_type_ref(package, value_type);
            format!("Vec<{value}>")
        }
        Type::Set(value_type) => {
            let value = generate_type_ref(package, value_type);
            format!("HashSet<{value}>")
        }
        Type::Option(value_type) => {
            let value = generate_type_ref(package, value_type);
            format!("Option<{value}>")
        }
        Type::Ref(name, variables) => {
            if variables.is_empty() {
                name.clone()
            } else {
                let vars = variables
                    .iter()
                    .map(|type_| generate_type_ref(package, type_))
                    .join(", ");
                format!("{name}<{vars}>")
            }
        }
    }
}

fn generate_doc_comment(indent: &str, comment: &Option<String>) -> String {
    match comment {
        None => "".to_string(),
        Some(comment) => {
            let comment = comment
                .split("\n")
                .map(|line| format!("{indent}/// {line}"))
                .collect::<Vec<String>>()
                .join("\n");

            format!("{comment}\n")
        }
    }
}
