use crate::ast::{
    canonical::{Enum, Method, Module, Property, Record, Service, Type, Variant},
    source::Name,
};
use itertools::Itertools;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::string::ToString;
use std::{fs, io};

#[derive(Debug)]
pub struct Options {
    pub print: bool,
    pub output: Option<PathBuf>,
    pub package: String,
}

pub fn generate_kotlin_server(module: &Module, options: &Options) -> Result<(), io::Error> {
    let record_package = &options.package;
    //for decl in module.declarations.iter() {}
    if options.print {
        println!("{}", generate_json_functions(record_package));
        println!("{}", generate_result_type(record_package));
        for (_, record) in &module.records {
            println!("{}", generate_record(&record_package, record));
        }

        for (_, enum_value) in &module.enums {
            println!("{}", generate_enum(&record_package, enum_value));
        }

        for (_, service) in &module.services {
            println!("{}", generate_service(&record_package, service));
        }
    }

    if let Some(out) = &options.output {
        fs::create_dir_all(out)?;
        fs::create_dir_all(out.join("json"))?;
        fs::create_dir_all(out.join("models"))?;
        fs::create_dir_all(out.join("services"))?;

        // JSON HELPERS
        let json_helpers = generate_json_functions(record_package);
        let mut file = File::create(out.join("json").join("json.kt"))?;
        file.write_all(json_helpers.as_bytes())?;

        let result_type = generate_result_type(record_package);
        let mut file = File::create(out.join("models").join("Result.kt"))?;
        file.write_all(result_type.as_bytes())?;

        for (_, record) in &module.records {
            let content = generate_record(&record_package, record);
            let mut file =
                File::create(out.join("models").join(format!("{}.kt", record.name.value)))?;
            file.write_all(content.as_bytes())?;
        }

        for (_, enum_value) in &module.enums {
            let content = generate_enum(&record_package, enum_value);
            let mut file = File::create(
                out.join("models")
                    .join(format!("{}.kt", enum_value.name.value)),
            )?;
            file.write_all(content.as_bytes())?;
        }

        for (_, service) in &module.services {
            let content = generate_service(&record_package, service);
            let mut file = File::create(
                out.join("services")
                    .join(format!("{}.kt", service.name.value)),
            )?;
            file.write_all(content.as_bytes())?;
        }
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

    let imports = format!(
        r#"
import kotlinx.serialization.json.*
import {package}.json.*
    "#
    );

    format!("package {package}.models\n{imports}\n\n{doc_comment}{class}")
}

fn generate_service(package: &String, service: &Service) -> String {
    let methods = service
        .methods
        .iter()
        .map(|(_, method)| generate_method(package, &method))
        .join("\n\n");

    let requests = service
        .methods
        .iter()
        .filter(|(_, method)| !method.parameters.is_empty())
        .map(|(_, method)| indent_lines("    ", generate_request(package, method)))
        .join("\n\n");

    let imports = format!(
        r#"
import io.ktor.http.*
import io.ktor.server.application.*
import io.ktor.server.request.*
import io.ktor.server.response.*
import io.ktor.server.routing.*
import kotlinx.serialization.json.*
import {package}.json.*
import {package}.models.*
    "#
    );

    let name = service.name.value.clone();
    let doc_comment = generate_doc_comment("", &service.comment);
    let companion_object = generate_companion_object(package, service);
    format!("package {package}.services\n{imports}\n\n{doc_comment}interface {name} {{\n{requests}\n\n{methods}\n\n{companion_object}\n}}")
}

fn generate_method(package: &String, method: &Method) -> String {
    let request = if method.parameters.is_empty() {
        "".to_string()
    } else {
        format!("request: {}", method.name.request_name())
    };

    let return_type = method
        .return_type
        .as_ref()
        .map(|type_| format!(": {}", generate_type_ref(package, &type_)))
        .unwrap_or("".to_string());

    let name = method.name.value.clone();
    let doc_comment = generate_doc_comment("    ", &method.comment);
    format!("{doc_comment}    fun {name}({request}){return_type}")
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
    let request_name = method.name.request_name();
    let x = match (method.parameters.is_empty(), &method.return_type) {
        (false, Some(return_type)) => [
            format!("    val data = call.receiveNullable<JsonElement>()"),
            format!("    var error: JsonElement? = null"),
            format!(
                "    val request = {request_name}.decode(data, required = true) {{ error = it }}"
            ),
            format!("    if (error != null) {{"),
            format!("        call.respondNullable(HttpStatusCode.BadRequest, error)"),
            format!("    }} else {{"),
            format!("        val result = service.{name}(request!!)"),
            format!(
                "        call.respondNullable({})",
                encode_json(&"result".to_string(), return_type)
            ),
            format!("    }}"),
        ]
        .join("\n"),
        (false, None) => [
            format!("    val data = call.receiveNullable<JsonElement>()"),
            format!("    var error: JsonElement? = null"),
            format!(
                "    val request = {request_name}.decode(data, required = true) {{ error = it }}"
            ),
            format!("    if (error != null) {{"),
            format!("        call.respondNullable(HttpStatusCode.BadRequest, error)"),
            format!("    }} else {{"),
            format!("        service.{name}(request!!)"),
            format!("        call.respond(HttpStatusCode.NoContent)"),
            format!("    }}"),
        ]
        .join("\n"),
        (true, None) => [format!("        service.{name}()")].join("\n"),
        (true, Some(return_type)) => [
            format!("    val result = service.{name}()"),
            format!(
                "    call.respondNullable({})",
                encode_json(&"result".to_string(), return_type)
            ),
        ]
        .join("\n"),
    };

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

    let fields = record
        .properties
        .iter()
        .map(|property| (&property.name, &property.type_))
        .collect::<Vec<(&Name, &Type)>>();
    let parse_json_fn = parse_json_object(&record.name.value, &fields);
    let encode_json_fn = encode_json_object(&record.name.value, &fields);

    let companion_object = [
        format!("    companion object {{"),
        indent_lines("        ", parse_json_fn),
        format!("    }}"),
    ]
    .join("\n");

    let imports = format!(
        r#"
import kotlinx.serialization.json.*
import {package}.json.*
    "#
    );

    let doc_comment = generate_doc_comment("", &record.comment);
    format!("package {package}.models\n{imports}\n\n{doc_comment}{class} {{\n\n{encode_json_fn}\n\n{companion_object}\n\n}}")
}

fn generate_request(package: &String, method: &Method) -> String {
    let request_name = method.name.request_name();
    let properties = method
        .parameters
        .iter()
        .map(|param| {
            let name = &param.name.value;
            let type_ref = generate_type_ref(package, &param.type_);
            format!("    val {name}: {type_ref}")
        })
        .collect::<Vec<String>>()
        .join(",\n");

    let fields = method
        .parameters
        .iter()
        .map(|property| (&property.name, &property.type_))
        .collect::<Vec<(&Name, &Type)>>();

    let parse_json_fn = parse_json_object(&method.name.request_name(), &fields);

    let companion_object = [
        format!("    companion object {{"),
        indent_lines("        ", parse_json_fn),
        format!("    }}"),
    ]
    .join("\n");

    let modifier = if method.parameters.len() == 1 {
        "@JvmInline\nvalue"
    } else {
        "data"
    };
    let class = format!("{modifier} class {request_name}(\n{properties}\n)");

    format!("{class} {{\n\n{companion_object}\n\n}}")
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

fn parse_json_object(name: &String, fields: &Vec<(&Name, &Type)>) -> String {
    let var_json = &"json".to_string();
    let var_errors = &"errors".to_string();
    let parsing_fields = fields
        .iter()
        .map(|(name, type_)| parse_json_field(&"this".to_string(), name, type_))
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
        format!("inline fun decode({var_json}: JsonElement?, required: Boolean = true, error: (JsonElement) -> Unit): {name}? = "),
        format!("    decodeObject({var_json}, required, error) {{"),
        format!("        val {var_errors} = mutableMapOf<String, JsonElement>()"),
        indent_lines("        ", parsing_fields),
        "".to_string(),
        format!("        if ({var_errors}.isNotEmpty()) {{"),
        format!("            error(JsonObject({var_errors}))"),
        format!("            null"),
        format!("        }} else {{"),
        format!("            {name}("),
        indent_lines("                ", constructor_fields),
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

fn encode_json_object(name: &String, fields: &Vec<(&Name, &Type)>) -> String {
    let parsing_fields = fields
        .iter()
        .map(|(name, type_)| encode_json_field(&name.value, &name.value, type_))
        .join("\n");

    [
        format!("fun encode(): JsonElement = buildJsonObject {{ "),
        indent_lines("    ", parsing_fields),
        format!("}}"),
    ]
    .join("\n")
}

fn encode_json_field(name: &String, var_prop: &String, type_: &Type) -> String {
    format!("put(\"{name}\", {})", encode_json(var_prop, type_))
}

fn encode_json(var_expr: &String, type_: &Type) -> String {
    match type_ {
        Type::String => var_expr.clone(),
        Type::Boolean => var_expr.clone(),
        Type::Int32 => var_expr.clone(),
        Type::Int64 => var_expr.clone(),
        Type::Float32 => var_expr.clone(),
        Type::Float64 => var_expr.clone(),
        Type::Map(_, _) => "".to_string(),
        Type::Result(error_type, ok_type) => format!(
            "{var_expr}.encode(encodeOk = {{ {} }}, encodeErr = {{ {} }})",
            encode_json(&"it".to_string(), ok_type),
            encode_json(&"it".to_string(), error_type)
        ),
        Type::List(type_) => format!(
            "buildJsonArray {{ {var_expr}.forEach {{ add({}) }} }}",
            encode_json(&"it".to_string(), type_)
        ),
        Type::Set(type_) => format!(
            "encodeArray({var_expr}) {{ {} }}",
            encode_json(&"it".to_string(), type_)
        ),
        Type::Option(type_) => encode_json(&format!("{var_expr}?"), type_),
        Type::Ref(name) => format!("{var_expr}.encode()"),
    }
}

fn parse_json<F>(var: &String, var_expr: &String, required: bool, type_: &Type, error: F) -> String
where
    F: Fn(String) -> String,
{
    match type_ {
        Type::String => [
            format!("val {var} = decodeString({var_expr}, required = {required}) {{"),
            format!("    {}", error("it".to_string())),
            format!("}}",),
        ]
        .join("\n"),
        Type::Int32 => [
            format!("val {var} = decodeInt32({var_expr}, required = {required}) {{"),
            format!("    {}", error("it".to_string())),
            format!("}}",),
        ]
        .join("\n"),
        Type::Int64 => [
            format!("val {var} = decodeInt64({var_expr}, required = {required}) {{"),
            format!("    {}", error("it".to_string())),
            format!("}}",),
        ]
        .join("\n"),
        Type::Boolean => [
            format!("val {var} = decodeBoolean({var_expr}, required = {required}) {{"),
            format!("    {}", error("it".to_string())),
            format!("}}",),
        ]
        .join("\n"),
        Type::Float32 => [
            format!("val {var} = decodeFloat32({var_expr}, required = {required}) {{"),
            format!("    {}", error("it".to_string())),
            format!("}}",),
        ]
        .join("\n"),
        Type::Float64 => [
            format!("val {var} = decodeFloat64({var_expr}, required = {required}) {{"),
            format!("    {}", error("it".to_string())),
            format!("}}",),
        ]
        .join("\n"),
        Type::Option(type_) => parse_json(var, var_expr, false, type_, error),
        Type::Ref(name) => [
            format!("val {var} = {name}.decode({var_expr}, required = {required}) {{"),
            format!("    {}", error("it".to_string())),
            format!("}}"),
        ]
        .join("\n"),
        _ => format!(""),
    }
}

fn generate_json_functions(package: &String) -> String {
    let data = include_str!("kotlin/json.kt");
    format!("package {package}.json\n\n{data}")
}

fn generate_result_type(package: &String) -> String {
    let data = include_str!("kotlin/result.kt");
    let imports = r#"
import io.noobymatze.ruff.generated.json.*
import kotlinx.serialization.json.*
    "#;
    format!("package {package}.models\n{imports}\n\n{data}")
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
            &"Greet".to_string(),
            &vec![
                (&Name::from_str("test"), &Type::String),
                (&Name::from_str("foo"), &Type::String),
                (&Name::from_str("age"), &Type::Option(Box::new(Type::Int32))),
                (&Name::from_str("person"), &Type::Ref("Person".to_string())),
            ],
        );

        println!("{parsing}",);
    }
}
