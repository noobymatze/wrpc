use crate::ast::canonical::Annotation;
use crate::ast::constraints::Constraint;
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

const OPEN: &'static str = "{";
const CLOSE: &'static str = "}";

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
            println!("{}", generate_record(&record_package, record, true));
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
            let content = generate_record(&record_package, record, true);
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
    let is_sealed = !record.is_simple();

    let variants = record
        .variants
        .iter()
        .map(|variant| generate_variant(package, &record.name, variant, is_sealed))
        .collect::<Vec<String>>()
        .join("\n");

    let doc_comment = generate_doc_comment("", &record.comment);

    let decode_json = decode_enum(record);
    let encode_json = encode_enum(record);
    let name = &record.name.value;
    let class = if is_sealed {
        format!("sealed class {name} {{\n\n{variants}\n}}")
    } else {
        format!("enum class {name} {{\n{variants}\n}}")
    };

    println!("{}", encode_json);
    let file_header = model_header(package, true);
    format!("{file_header}\n\n{doc_comment}{class}\n\n{decode_json}\n\n{encode_json}")
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
                encode_type(&"result".to_string(), return_type)
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
                encode_type(&"result".to_string(), return_type)
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
    format!("{indent}val {name}: {type_},")
}

fn generate_record(package: &String, record: &Record, with_header: bool) -> String {
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

    [
        format!("{}", model_header(package, with_header)),
        empty_line(),
        format!("{}", generate_doc_comment("", &record.comment)),
        format!("{class}"),
        empty_line(),
        format!("{}", decode_record(record)),
        empty_line(),
        format!("{}", encode_record(record)),
    ]
    .join("\n")
}

fn empty_line() -> String {
    "".to_string()
}

fn model_header(package: &String, imports: bool) -> String {
    if !imports {
        return "".to_string();
    }

    [
        format!("package {package}.models"),
        empty_line(),
        format!("import kotlinx.serialization.json.*"),
        format!("import {package}.json.*"),
    ]
    .join("\n")
}

fn generate_request(package: &String, method: &Method) -> String {
    let request_name = method.name.request_name();
    let properties = method
        .parameters
        .iter()
        .map(|param| Property {
            annotations: param.annotations.clone(),
            comment: None,
            name: param.name.clone(),
            type_: param.type_.clone(),
        })
        .collect::<Vec<Property>>();

    let record = Record {
        name: Name::from_str(request_name.as_str()),
        annotations: vec![],
        comment: None,
        properties,
        type_variables: vec![],
    };

    generate_record(package, &record, false)
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
        Type::Ref(name, types) => {
            if types.is_empty() {
                name.clone()
            } else {
                let refs = types
                    .iter()
                    .map(|type_| generate_type_ref(package, type_))
                    .join(", ");

                format!("{name}<{refs}>")
            }
        }
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
    format!("put(\"{name}\", {})", encode_type(var_prop, type_))
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
    format!("package {package}.models{imports}\n\n{data}")
}

fn validate_value(record: &Record) -> String {
    "".to_string()
}

// JSON ENCODE

fn encode_record(record: &Record) -> String {
    let name = &record.name.value;
    let var_name = "value".to_string();
    let indent = "    ".to_string();

    [
        format!("fun encode{name}({var_name}: {name}): JsonElement = buildJsonObject {OPEN} "),
        encode_fields(&indent, &var_name, &record.properties),
        format!("{CLOSE}"),
    ]
    .join("\n")
}

fn encode_enum(record: &Enum) -> String {
    let name = &record.name.value;
    let var_name = "value".to_string();
    let indent = "    ".to_string();

    if record.is_simple() {
        [
            format!("fun encode{name}({var_name}: {name}): JsonElement = "),
            encode_simple_variants(&indent, &var_name, name, &record.variants),
        ]
        .join("\n")
    } else {
        let type_vars = type_vars(&record.type_variables)
            .map(|x| format!(" {x} "))
            .unwrap_or(" ".to_string());

        let type_ref = generate_type_ref(&"".to_string(), &record.as_type());

        [
            format!("fun{type_vars}encode{name}({var_name}: {type_ref}): JsonElement = buildJsonObject {OPEN} "),
            encode_variants(&indent, &var_name, name, &record.variants),
            format!("{CLOSE}"),
        ]
        .join("\n")
    }
}

fn type_vars(type_variables: &Vec<Name>) -> Option<String> {
    if type_variables.is_empty() {
        None
    } else {
        let result = format!(
            "<{}>",
            type_variables
                .iter()
                .map(|variable| &variable.value)
                .join(", "),
        );
        Some(result)
    }
}

fn encode_simple_variants(
    indent: &String,
    var_object: &String,
    enum_name: &String,
    variants: &Vec<Variant>,
) -> String {
    [
        format!("{indent}when ({var_object}) {OPEN}"),
        variants
            .iter()
            .map(|variant| {
                [format!(
                    "{indent}    {enum_name}.{} -> JsonPrimitive(\"{}\")",
                    variant.name.value, variant.name.value
                )]
                .join("\n")
            })
            .join("\n"),
        format!("{indent}{CLOSE}"),
    ]
    .join("\n")
}

fn encode_variants(
    indent: &String,
    var_object: &String,
    enum_name: &String,
    variants: &Vec<Variant>,
) -> String {
    [
        format!("{indent}when ({var_object}) {OPEN}"),
        variants
            .iter()
            .map(|variant| {
                [
                    format!(
                        "{indent}    is {enum_name}.{} -> {OPEN}",
                        variant.name.value
                    ),
                    format!("{indent}        put(\"@type\", \"{}\")", variant.name.value),
                    encode_fields(
                        &format!("{indent}        "),
                        var_object,
                        &variant.properties,
                    ),
                    format!("{indent}    {CLOSE}"),
                ]
                .join("\n")
            })
            .join("\n"),
        format!("{indent}{CLOSE}"),
    ]
    .join("\n")
}

fn encode_fields(indent: &String, var_object: &String, properties: &Vec<Property>) -> String {
    properties
        .iter()
        .map(|property| encode_field(&indent, &var_object, &property.name.value, &property.type_))
        .join("\n")
}

fn encode_field(indent: &String, var_object: &String, name: &String, type_: &Type) -> String {
    let type_ = encode_type(&format!("{var_object}.{name}"), type_);
    format!("{indent}put(\"{name}\", {type_})")
}

fn encode_type(var_expr: &String, type_: &Type) -> String {
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
            encode_type(&"it".to_string(), ok_type),
            encode_type(&"it".to_string(), error_type)
        ),
        Type::List(type_) => format!(
            "buildJsonArray {{ {var_expr}.forEach {{ add({}) }} }}",
            encode_type(&"it".to_string(), type_)
        ),
        Type::Set(type_) => format!(
            "buildJsonArray {{ {var_expr}.forEach {{ add({}) }} }}",
            encode_type(&"it".to_string(), type_)
        ),
        Type::Option(type_) => encode_type(&format!("{var_expr}"), type_),
        Type::Ref(name, _) => format!("encode{name}({var_expr})"),
    }
}

// JSON DECODE

fn decode_record(record: &Record) -> String {
    let name = &record.name.value;
    let indent = "    ".to_string();
    let var_object = "json".to_string();
    let var_errors = "errors".to_string();

    let constructor = call_constructor(&format!("{indent}    "), name, &record.properties);

    [
        format!("fun decode{name}(json: JsonElement, errors: Errors): {name}? {OPEN}"),
        format!("    if (json !is JsonObject) {OPEN}"),
        format!("        errors.error(errors.expect(\"OBJECT\"))"),
        format!("        return null"),
        format!("    {CLOSE}"),
        "".to_string(),
        decode_fields(&indent, &var_object, &var_errors, &record.properties),
        "".to_string(),
        format!("    if (errors.isEmpty()) {OPEN}"),
        format!("        val {name} = {constructor}"),
        //format!("        validate{name}({name}, errors)"),
        format!("        return {name}"),
        format!("    {CLOSE} else {OPEN}"),
        format!("        return null"),
        format!("    {CLOSE}"),
        format!("{CLOSE}"),
    ]
    .join("\n")
}

fn decode_enum(record: &Enum) -> String {
    let name = &record.name.value;
    if record.is_simple() {
        let variants = record
            .variants
            .iter()
            .map(|variant| {
                [
                    format!("if (value == \"{}\") {OPEN}", variant.name.value),
                    format!("        return {name}.{}", variant.name.value),
                    format!("    {CLOSE}"),
                ]
                .join("\n")
            })
            .join(" else ");

        [
            format!("fun decode{name}(json: JsonElement, errors: Errors): {name}? {OPEN}"),
            format!("    if (json !is JsonPrimitive || !json.isString) {OPEN}"),
            format!("        errors.error(errors.expect(\"STRING\"))"),
            format!("        return null"),
            format!("    {CLOSE}"),
            format!("    val value = json.content"),
            format!("    {variants} else {OPEN}"),
            format!(
                "        errors.error(errors.expect(\"ONEOF {}\"))",
                record.name.value
            ),
            format!("        return null"),
            format!("    {CLOSE}"),
            format!("{CLOSE}"),
        ]
        .join("\n")
    } else {
        [
            format!("fun decode{name}(json: JsonElement, errors: Errors): {name}? {OPEN}"),
            format!("    if (json !is JsonObject) {OPEN}"),
            format!("        errors.error(errors.expect(\"OBJECT\"))"),
            format!("        return null"),
            format!("    {CLOSE}"),
            decode_field(
                &"    ".to_string(),
                &"json".to_string(),
                &"type".to_string(),
                &"@type".to_string(),
                &"errors".to_string(),
                &Type::String,
            ),
            format!(
                "    {}",
                record
                    .variants
                    .iter()
                    .map(|variant| {
                        [
                            format!("if (type == \"{}\") {OPEN}", variant.name.value),
                            decode_variant(
                                &"    ".to_string(),
                                &variant,
                                &"json".to_string(),
                                &"errors".to_string(),
                            ),
                            format!("    {CLOSE}"),
                        ]
                        .join("\n")
                    })
                    .join(" else ")
            ),
            "".to_string(),
            format!("{CLOSE}"),
        ]
        .join("\n")
    }
}

fn decode_variant(
    indent: &String,
    variant: &Variant,
    var_object: &String,
    var_error: &String,
) -> String {
    let name = &variant.name.value;
    let constructor = call_constructor(&format!("{indent}        "), name, &variant.properties);
    [
        decode_fields(
            &format!("{indent}    "),
            var_object,
            var_error,
            &variant.properties,
        ),
        "".to_string(),
        format!("{indent}    if (errors.isEmpty()) {OPEN}"),
        format!("{indent}        val {name} = {constructor}",),
        //format!("{indent}        validate{name}({name}, errors)"),
        format!("{indent}        return {name}"),
        format!("{indent}    {CLOSE} else {OPEN}"),
        format!("{indent}        return null"),
        format!("{indent}    {CLOSE}"),
    ]
    .join("\n")
}

fn call_constructor(indent: &String, name: &String, properties: &Vec<Property>) -> String {
    let properties = properties
        .iter()
        .map(|property| {
            let name = &property.name.value;
            let required = if matches!(property.type_, Type::Option(_)) {
                ""
            } else {
                "!!"
            };

            format!("{indent}    {name} = {name}{required},")
        })
        .join("\n");

    [format!("{name}("), properties, format!("{indent})")].join("\n")
}

fn decode_fields(
    indent: &str,
    var_object: &String,
    var_error: &String,
    properties: &Vec<Property>,
) -> String {
    properties
        .iter()
        .map(|property| {
            decode_field(
                &format!("{indent}"),
                &var_object,
                &property.name.value,
                &property.name.value,
                &var_error,
                &property.type_,
            )
        })
        .join("\n\n")
}

fn decode_field(
    indent: &str,
    var_object: &String,
    var_name: &String,
    field_name: &String,
    var_error: &String,
    type_: &Type,
) -> String {
    let var_field = format!("{var_name}Field");

    [
        format!("{indent}val {var_field} = {var_object}[\"{field_name}\"]"),
        decode_type(
            indent,
            &var_field,
            var_name,
            var_error,
            type_,
            true,
            |err| format!("errors.field(\"{field_name}\", {err})"),
        ),
    ]
    .join("\n")
}

fn decode_type<F>(
    indent: &str,
    var_json: &String,
    var_name: &String,
    var_error: &String,
    type_: &Type,
    required: bool,
    error: F,
) -> String
where
    F: Fn(String) -> String,
{
    let type_name = match &type_ {
        Type::Option(type_) => generate_type_ref(&"".to_string(), &type_),
        _ => generate_type_ref(&"".to_string(), &type_),
    };

    let not_null_error = if required {
        [
            format!("else {OPEN}"),
            format!(
                "{indent}    {var_error}.error({})",
                error(format!("{var_error}.notNull"))
            ),
            format!("{indent}{CLOSE}"),
        ]
        .join("\n")
    } else {
        "".to_string()
    };

    match &type_ {
        Type::String => [
            format!("{indent}var {var_name}: {type_name}? = null"),
            format!("{indent}if ({var_json} != null) {OPEN}"),
            format!("{indent}    if ({var_json} is JsonPrimitive && {var_json}.isString) {OPEN}"),
            format!("{indent}        {var_name} = {var_json}.content"),
            format!("{indent}    {CLOSE} else {OPEN}"),
            format!(
                "{indent}        {var_error}.error({})",
                error(format!("{var_error}.expect(\"STRING\")"))
            ),
            format!("{indent}    {CLOSE}"),
            format!("{indent}{CLOSE} {not_null_error}"),
        ]
        .join("\n"),
        Type::Int32 => [
            format!("{indent}var {var_name}: {type_name}? = null"),
            format!("{indent}if ({var_json} != null) {OPEN}"),
            format!("{indent}    if ({var_json} is JsonPrimitive) {OPEN}"),
            format!("{indent}        {var_name} = {var_json}.intOrNull"),
            format!("{indent}    {CLOSE}"),
            format!("{indent}    if ({var_name} == null) {OPEN}"),
            format!(
                "{indent}        {var_error}.error({})",
                error(format!("{var_error}.expect(\"INT32\")"))
            ),
            format!("{indent}    {CLOSE}"),
            format!("{indent}{CLOSE} {not_null_error}"),
        ]
        .join("\n"),
        Type::Int64 => [
            format!("{indent}var {var_name}: {type_name}? = null"),
            format!("{indent}if ({var_json} != null) {OPEN}"),
            format!("{indent}    if ({var_json} is JsonPrimitive) {OPEN}"),
            format!("{indent}        {var_name} = {var_json}.longOrNull"),
            format!("{indent}    {CLOSE}"),
            format!("{indent}    if ({var_name} == null) {OPEN}"),
            format!(
                "{indent}        {var_error}.error({})",
                error(format!("{var_error}.expect(\"INT64\")"))
            ),
            format!("{indent}    {CLOSE}"),
            format!("{indent}{CLOSE} {not_null_error}"),
        ]
        .join("\n"),
        Type::Float32 => [
            format!("{indent}var {var_name}: {type_name}? = null"),
            format!("{indent}if ({var_json} != null) {OPEN}"),
            format!("{indent}    if ({var_json} is JsonPrimitive) {OPEN}"),
            format!("{indent}        {var_name} = {var_json}.floatOrNull"),
            format!("{indent}    {CLOSE}"),
            format!("{indent}    if ({var_name} == null) {OPEN}"),
            format!(
                "{indent}        {var_error}.error({})",
                error(format!("{var_error}.expect(\"FLOAT32\")"))
            ),
            format!("{indent}    {CLOSE}"),
            format!("{indent}{CLOSE} {not_null_error}"),
        ]
        .join("\n"),
        Type::Float64 => [
            format!("{indent}var {var_name}: {type_name}? = null"),
            format!("{indent}if ({var_json} != null) {OPEN}"),
            format!("{indent}    if ({var_json} is JsonPrimitive) {OPEN}"),
            format!("{indent}        {var_name} = {var_json}.doubleOrNull"),
            format!("{indent}    {CLOSE}"),
            format!("{indent}    if ({var_name} == null) {OPEN}"),
            format!(
                "{indent}        {var_error}.error({})",
                error(format!("{var_error}.expect(\"FLOAT64\")"))
            ),
            format!("{indent}    {CLOSE}"),
            format!("{indent}{CLOSE} {not_null_error}"),
        ]
        .join("\n"),
        Type::Boolean => [
            format!("{indent}var {var_name}: {type_name}? = null"),
            format!("{indent}if ({var_json} != null) {OPEN}"),
            format!("{indent}    if ({var_json} is JsonPrimitive) {OPEN}"),
            format!("{indent}        {var_name} = {var_json}.booleanOrNull"),
            format!("{indent}    {CLOSE}"),
            format!("{indent}    if ({var_name} == null) {OPEN}"),
            format!(
                "{indent}        {var_error}.error({})",
                error(format!("{var_error}.expect(\"BOOLEAN\")"))
            ),
            format!("{indent}    {CLOSE}"),
            format!("{indent}{CLOSE} {not_null_error}"),
        ]
        .join("\n"),
        Type::Option(type_) => {
            decode_type(indent, var_json, var_name, var_error, type_, false, error)
        }
        Type::Ref(name, _) => [
            format!("{indent}val {var_name}Errors = Errors()"),
            format!("{indent}val {var_name} = decode{name}({var_json}, {var_name}Errors)"),
            format!("{indent}{var_error}.error({var_name}Errors)"),
        ]
        .join("\n"),
        _ => format!("not implemented: {type_:?}"),
    }
}

fn render(constraint: &Constraint, property: &Option<Property>) -> String {
    match constraint {
        Constraint::Lt(constraints) => {
            if constraints.len() <= 1 {
                "true".to_string()
            } else {
                let result = constraints
                    .iter()
                    .map(|constraint| render(constraint, property))
                    .collect::<Vec<String>>();
                parenthesize(&result, "<")
            }
        }
        Constraint::Eq(constraints) => {
            if constraints.len() <= 1 {
                "true".to_string()
            } else {
                let result = constraints
                    .iter()
                    .map(|constraint| render(constraint, property))
                    .collect::<Vec<String>>();
                parenthesize(&result, "==")
            }
        }
        Constraint::Le(constraints) => {
            if constraints.len() <= 1 {
                "true".to_string()
            } else {
                let result = constraints
                    .iter()
                    .map(|constraint| render(constraint, property))
                    .collect::<Vec<String>>();
                parenthesize(&result, "<=")
            }
        }
        Constraint::Gt(constraints) => {
            if constraints.len() <= 1 {
                "true".to_string()
            } else {
                let result = constraints
                    .iter()
                    .map(|constraint| render(constraint, property))
                    .collect::<Vec<String>>();
                parenthesize(&result, ">")
            }
        }
        Constraint::Ge(constraints) => {
            if constraints.len() <= 1 {
                "true".to_string()
            } else {
                let result = constraints
                    .iter()
                    .map(|constraint| render(constraint, property))
                    .collect::<Vec<String>>();
                parenthesize(&result, ">=")
            }
        }
        Constraint::Or(constraints) => {
            let x = constraints
                .iter()
                .map(|constraint| render(constraint, property))
                .join(" || ");
            format!("({x})")
        }
        Constraint::And(constraints) => {
            let x = constraints
                .iter()
                .map(|constraint| render(constraint, property))
                .join(" && ");
            format!("({x})")
        }
        Constraint::Xor(constraints) => {
            let x = constraints
                .iter()
                .map(|constraint| render(constraint, property))
                .join(" xor ");
            format!("({x})")
        }
        Constraint::Len(constraint) => {
            let value = render(constraint, property);
            format!("{value}.size")
        }
        Constraint::Blank(constraint) => {
            let value = render(constraint, property);
            format!("{value}.isBlank()")
        }
        Constraint::Not(constraint) => {
            let value = render(constraint, property);
            format!("!({value})")
        }
        Constraint::Number(value) => format!("{value}"),
        Constraint::String(value) => format!("\"{value}\""),
        Constraint::Boolean(value) => value.to_string(),
        Constraint::Map(_) => "".to_string(),
        Constraint::Ref(name) => name.clone(),
    }
}

fn parenthesize(values: &Vec<String>, op: &str) -> String {
    // (= x 5 6 8)
    // => (((x == 5) == 6) == 8)
    let mut stack = values.iter().rev().cloned().collect::<Vec<String>>();
    while stack.len() > 1 {
        let a = stack.pop();
        let b = stack.pop();
        if let (Some(a), Some(b)) = (a, b) {
            stack.push(format!("({a} {op} {b})"));
        }
    }

    stack.pop().unwrap()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::ast::canonical::{Module, Record, Type};
    use crate::ast::constraints::Constraint;
    use crate::ast::source::Name;
    use crate::codegen::kotlin::{
        decode_enum, decode_field, decode_record, decode_type, generate_kotlin_server,
        generate_record, parenthesize, render, Options,
    };
    use crate::error::Error;
    use crate::reporting::Region;
    use crate::{compile, parse};

    #[test]
    fn test() -> Result<(), Error> {
        let spec = r#"
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
        "#;

        let module = compile(None, spec)?;
        let result = module.records.get("Address").expect("Get Address");
        let login_result = module.enums.get("LoginResult").expect("Get LoginResult");
        let country = module.enums.get("Country").expect("Get Country");

        println!("{}", decode_enum(&login_result));
        println!("{}", decode_enum(&country));
        println!("{}", generate_record(&"test".to_string(), &result, true));

        Ok(())
    }
}
