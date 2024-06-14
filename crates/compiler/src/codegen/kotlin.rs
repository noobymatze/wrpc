use crate::ast::{
    canonical::{Enum, Method, Module, Property, Record, Service, Type, Variant},
    source::Name,
};
use std::io;
use std::string::ToString;

pub fn generate_kotlin_server(module: &Module) -> Result<(), io::Error> {
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
    }

    Ok(())
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
        format!("sealed class {name} {{\n\n{variants}\n}}")
    } else {
        let name = record.name.value.clone();
        format!("enum class {name} {{\n{variants}\n}}")
    };

    format!("package {package}\n\n{doc_comment}{class}")
}

fn generate_service(package: &String, service: &Service) -> String {
    let methods = service
        .methods
        .iter()
        .map(|(_, method)| generate_method(package, &method))
        .collect::<Vec<String>>()
        .join("\n\n");

    let name = service.name.value.clone();
    let doc_comment = generate_doc_comment("    ", &service.comment);
    let companion_object = generate_companion_object(package, service);
    format!("{doc_comment}interface {name} {{\n{methods}\n\n{companion_object}\n}}")
}

fn generate_method(package: &String, method: &Method) -> String {
    let params = method
        .parameters
        .iter()
        .map(|param| {
            let name = param.name.value.clone();
            let type_ = generate_type_ref(package, &param.type_);
            format!("{name}: {type_}")
        })
        .collect::<Vec<String>>()
        .join(", ");

    let return_type = method
        .return_type
        .as_ref()
        .map(|type_| generate_type_ref(package, &type_))
        .unwrap_or("".to_string());

    let name = method.name.value.clone();
    let doc_comment = generate_doc_comment("    ", &method.comment);
    format!("{doc_comment}    fun {name}({params}): {return_type}")
}

fn generate_companion_object(record_package: &String, service: &Service) -> String {
    let name = service.name.value.clone();

    let handlers = service
        .methods
        .iter()
        .map(|(_, method)| generate_ktor_handler(record_package, service, method))
        .map(|handler| indent_lines("            ", handler))
        .collect::<Vec<String>>()
        .join("\n");

    let func = format!("        fun Routing.service(service: {name}) {{\n{handlers}\n        }}");

    format!("    companion object {{\n\n{func}\n    }}")
}

fn generate_ktor_handler(_record_package: &String, service: &Service, method: &Method) -> String {
    let service_name = service.name.value.clone();
    let name = method.name.value.clone();
    let request_name = format!("{}Request", method.name.capitalized());

    let x = format!("    val data = call.receive<{request_name}>()");

    format!("post(\"/{service_name}/{name}\") {{\n{x}\n}}")
}

fn generate_variant(
    package: &String,
    parent_name: &Name,
    variant: &Variant,
    is_sealed: bool,
) -> String {
    let doc_comment = generate_doc_comment("    ", &variant.comment);
    let variant = if is_sealed {
        generate_sealed_sub_class(package, parent_name, variant)
    } else {
        let name = variant.name.value.clone();
        format!("    {},", name)
    };

    format!("{doc_comment}{variant}")
}

fn generate_sealed_sub_class(package: &String, parent_name: &Name, variant: &Variant) -> String {
    let class = if variant.properties.is_empty() {
        let name = variant.name.value.clone();
        let parent_name = parent_name.value.clone();
        format!("    data object {name}: {parent_name}()\n")
    } else {
        let properties = variant
            .properties
            .iter()
            .map(|property| generate_property("        ", package, property))
            .collect::<Vec<String>>()
            .join("\n");

        let name = variant.name.value.clone();
        let parent_name = parent_name.value.clone();
        format!("    data class {name}(\n{properties}\n    ): {parent_name}()\n")
    };

    class
}

fn generate_property(indent: &str, package: &String, property: &Property) -> String {
    let name = property.name.value.clone();
    let type_ = generate_type_ref(package, &property.type_);
    let nullable = match &property.type_ {
        Type::Option(_) => "?",
        _ => "",
    };
    format!("{indent}val {name}: {type_}{nullable},")
}

fn generate_record(package: &String, record: &Record) -> String {
    let class = if record.properties.is_empty() {
        format!("data object {}", record.name.value)
    } else {
        let properties = record
            .properties
            .iter()
            .map(|property| generate_property("    ", package, property))
            .collect::<Vec<String>>()
            .join("\n");

        let name = record.name.value.clone();
        format!("data class {name}(\n{properties}\n)",)
    };

    let doc_comment = generate_doc_comment("", &record.comment);
    format!("package {package}\n\n{doc_comment}{class}")
}

fn generate_type_ref(package: &String, type_: &Type) -> String {
    match type_ {
        Type::String => "kotlin.String".to_string(),
        Type::Boolean => "kotlin.Boolean".to_string(),
        Type::Int32 => "kotlin.Int".to_string(),
        Type::Int64 => "kotlin.Long".to_string(),
        Type::Float32 => "kotlin.Float".to_string(),
        Type::Float64 => "kotlin.Double".to_string(),
        Type::Map(key_type, value_type) => {
            let key = generate_type_ref(package, key_type);
            let value = generate_type_ref(package, value_type);
            format!("kotlin.Map<{key}, {value}>")
        }
        Type::Result(error_type, value_type) => {
            let error = generate_type_ref(package, error_type);
            let value = generate_type_ref(package, value_type);
            format!("Result<{error}, {value}>")
        }
        Type::List(value_type) => {
            let value = generate_type_ref(package, value_type);
            format!("kotlin.collections.List<{value}>")
        }
        Type::Set(value_type) => {
            let value = generate_type_ref(package, value_type);
            format!("kotlin.collections.Set<{value}>")
        }
        Type::Option(value_type) => {
            let value = generate_type_ref(package, value_type);
            format!("{value}?")
        }
        Type::Ref(name) => name.clone(),
    }
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

fn indent_lines(indent: &str, value: String) -> String {
    value
        .split("\n")
        .map(|line| format!("{indent}{line}"))
        .collect::<Vec<String>>()
        .join("\n")
}

fn parse_json_object(name: &Name, fields: Vec<(&Name, &Type)>) -> String {
    let var_json = &"json".to_string();
    let name = &name.value;
    let var_errors = &"errors".to_string();
    let parsing_fields = fields
        .iter()
        .map(|(name, type_)| parse_json_field(&"value".to_string(), name, type_))
        .collect::<Vec<String>>()
        .join("\n\n");

    let constructor_fields = fields
        .iter()
        .map(|(name, type_)| match type_ {
            Type::Option(type_) => format!("{} = {}", name.value, name.value),
            _ => format!("{} = {}!!", name.value, name.value),
        })
        .collect::<Vec<String>>()
        .join(",\n");

    [
        format!("inline fun parse({var_json}: JsonElement?, required: Boolean = true, error: (JsonElement) -> Unit): {name}? = "),
        format!("    parseObject({var_json}, required, error) {{ value ->"),
        format!("        val {var_errors} = mutableMapOf<String, JsonElement>()"),
        indent_lines("        ", parsing_fields),
        "".to_string(),
        format!("        if ({var_errors}.isNotEmpty()) {{"),
        format!("            error(JsonObject({var_errors}))"),
        format!("            null"),
        format!("        }} else {{"),
        format!("            {name}("),
        indent_lines("            ", constructor_fields),
        format!("            )"),
        format!("        }}"),
        format!("    }}"),
    ]
    .join("\n")
}

fn parse_json_field(var_json: &String, name: &Name, type_: &Type) -> String {
    let name = &name.value;
    let var_expr = format!("{var_json}[\"{name}\"]");
    parse_json(name, &var_expr, true, type_, |error| {
        format!("errors[\"{name}\"] = {error}")
    })
}

fn parse_json<F>(var: &String, var_expr: &String, required: bool, type_: &Type, error: F) -> String
where
    F: Fn(String) -> String,
{
    match type_ {
        Type::String => [
            format!("val {var} = parseString({var_expr}, required = {required}) {{"),
            format!("    {}", error("it".to_string())),
            format!("}}",),
        ]
        .join("\n"),
        Type::Int32 => [
            format!("val {var} = parseInt({var_expr}, required = {required}) {{"),
            format!("    {}", error("it".to_string())),
            format!("}}",),
        ]
        .join("\n"),
        Type::Int64 => [
            format!("val {var} = parseLong({var_expr}, required = {required}) {{"),
            format!("    {}", error("it".to_string())),
            format!("}}",),
        ]
        .join("\n"),
        Type::Boolean => [
            format!("val {var} = parseBoolean({var_expr}, required = {required}) {{"),
            format!("    {}", error("it".to_string())),
            format!("}}",),
        ]
        .join("\n"),
        Type::Option(type_) => parse_json(var, var_expr, false, type_, error),
        Type::Ref(name) => [
            format!("val {var} = {name}.parse({var_expr}, required = {required}) {{"),
            format!("    {}", error("it".to_string())),
            format!("}}"),
        ]
        .join("\n"),
        _ => format!(""),
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::canonical::Type;
    use crate::ast::source::Name;
    use crate::codegen::kotlin::{parse_json, parse_json_field, parse_json_object};

    #[test]
    fn test() {
        let record_name = Name::from_str("Greet");

        let parsing = parse_json_object(
            &record_name,
            vec![
                (&Name::from_str("test"), &Type::String),
                (&Name::from_str("foo"), &Type::String),
                (&Name::from_str("age"), &Type::Option(Box::new(Type::Int32))),
                (&Name::from_str("person"), &Type::Ref("Person".to_string())),
            ],
        );

        println!("{parsing}",);
    }
}
