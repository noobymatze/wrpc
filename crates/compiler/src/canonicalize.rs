use itertools::Itertools;

use crate::ast::canonical::Expr;
use crate::ast::constraints::Constraint;
use crate::ast::source::Decl;
use crate::ast::{canonical as can, source as src};
use crate::error::canonicalize;
use crate::reporting::Region;
use std::collections::{HashMap, HashSet};

pub fn canonicalize(module: &src::Module) -> Result<can::Module, Vec<canonicalize::Error>> {
    let mut records = HashMap::new();
    let mut enums = HashMap::new();
    let mut services = HashMap::new();
    let mut errors = vec![];
    for decl in module.declarations.iter() {
        match decl {
            Decl::Data(data) => match canonicalize_data(data) {
                Ok(record) => {
                    records.insert(record.name.value.clone(), record);
                }
                Err(record_errors) => {
                    let mut record_errors = record_errors
                        .iter()
                        .map(|error| {
                            canonicalize::Error::BadRecord(data.name.clone(), error.clone())
                        })
                        .collect::<Vec<canonicalize::Error>>();
                    errors.append(&mut record_errors);
                }
            },
            Decl::Enum(data) => match canonicalize_enum(data) {
                Ok(enum_value) => {
                    enums.insert(data.name.value.clone(), enum_value);
                }
                Err(enum_errors) => {
                    let mut enum_errors = enum_errors
                        .iter()
                        .map(|error| canonicalize::Error::BadEnum(data.name.clone(), error.clone()))
                        .collect::<Vec<canonicalize::Error>>();
                    errors.append(&mut enum_errors);
                }
            },
            Decl::Service(service) => match canonicalize_service(service) {
                Ok(record) => {
                    services.insert(record.name.value.clone(), record);
                }
                Err(record_errors) => {
                    let mut record_errors = record_errors
                        .iter()
                        .map(|error| {
                            canonicalize::Error::BadService(service.name.clone(), error.clone())
                        })
                        .collect::<Vec<canonicalize::Error>>();
                    errors.append(&mut record_errors);
                }
            },
        }
    }

    if errors.is_empty() {
        Ok(can::Module {
            records,
            services,
            enums,
        })
    } else {
        Err(errors)
    }
}

fn canonicalize_property<E, F>(
    property: &src::Property,
    map_err: F,
) -> Result<can::Property, Vec<E>>
where
    F: FnMut(&canonicalize::Annotation) -> E,
{
    let mut annotations = vec![];
    let mut constraints = vec![];
    let mut errors = vec![];
    match canonicalize_annotations(&property.annotations, &mut constraints, &mut annotations) {
        Err(annotation_errors) => {
            let mut annotation_errors = annotation_errors.iter().map(map_err).collect::<Vec<E>>();
            errors.append(&mut annotation_errors);
        }
        Ok(()) => (),
    };

    let deps = compute_property_dependency(&property.name.value, &constraints)
        .iter()
        .cloned()
        .collect_vec();

    if errors.is_empty() {
        Ok(can::Property {
            comment: property.doc_comment.clone(),
            name: property.name.clone(),
            type_: parse_type(&property.type_),
            annotations,
            constraints,
            deps,
        })
    } else {
        Err(errors)
    }
}

fn canonicalize_data(data: &src::Data) -> Result<can::Record, Vec<canonicalize::Record>> {
    let mut properties = vec![];
    let mut errors = vec![];
    for property in &data.properties {
        let result = canonicalize_property(property, |error| {
            canonicalize::Record::BadProperty(
                property.name.clone(),
                canonicalize::Property::BadAnnotation(error.clone()),
            )
        });

        match result {
            Err(mut prop_errors) => errors.append(&mut prop_errors),
            Ok(prop) => properties.push(prop),
        }
    }

    let property_deps = compute_property_dependencies(&properties);
    let property_validation_order = sorted_by_topology(&property_deps);

    let mut annotations = vec![];
    let mut constraints = vec![];
    let record_annotations =
        canonicalize_annotations(&data.annotations, &mut constraints, &mut annotations).map_err(
            |annotation_errors| {
                annotation_errors
                    .iter()
                    .map(|error| canonicalize::Record::BadAnnotation(error.clone()))
                    .collect::<Vec<canonicalize::Record>>()
            },
        );

    match record_annotations {
        Ok(()) => {
            if errors.is_empty() {
                Ok(can::Record {
                    annotations,
                    constraints,
                    property_validation_order,
                    comment: data.doc_comment.clone(),
                    name: data.name.clone(),
                    properties,
                    type_variables: data.type_variables.clone(),
                })
            } else {
                Err(errors)
            }
        }
        Err(mut annotation_errors) => {
            errors.append(&mut annotation_errors);
            Err(errors)
        }
    }
}

/// Returns a graph of dependencies for each [`Property`].
///
/// The returned graph is essential to determine the order in which
/// property validations should occur, to ensure all dependencies
/// are satisfied.
///
/// ## Explanation
///
/// A [`Constraint`] of a property can depend on other properties. For
/// example, the length of a zipcode of an address may depend on the
/// country of said address. That means, a country has to be valid in
/// order to check a zipcode, which requires generating validation of
/// a country before the zipcode.
///
/// To achieve this, this function computes the dependencies for each
/// property.
///
/// ## Example
///
/// The following specification
///
/// ```wrpc
/// data Address {
///     #(check (and (= .country "DE") (= (len .zipcode) 5)))
///     zipcode: String,
///     #(check (or (= .country "DE") (= .country "CH")))
///     country: String,
/// }
/// ```
///
/// should result in the following map:
///
/// ```
/// {"zipcode": ["country"], "country": []}
/// ```
///
fn compute_property_dependencies(properties: &Vec<can::Property>) -> HashMap<String, &Vec<String>> {
    let mut dependencies = HashMap::new();
    for property in properties {
        dependencies.insert(property.name.value.clone(), &property.deps);
    }

    dependencies
}

fn compute_property_dependency(
    prop_name: &String,
    constraints: &Vec<Constraint>,
) -> HashSet<String> {
    let mut deps = HashSet::new();
    for constraint in constraints {
        constraint.collect_accessed_deps(&mut deps);
    }
    // Remove the current property, it's not a real dependency.
    deps.remove(prop_name);
    deps
}

fn sorted_by_topology(graph: &HashMap<String, &Vec<String>>) -> Vec<String> {
    fn visit(
        node: &String,
        sorted_properties: &mut Vec<String>,
        visited: &mut HashSet<String>,
        temp_marks: &mut HashSet<String>,
        graph: &HashMap<String, &Vec<String>>,
    ) {
        if visited.contains(node) {
            return;
        }

        if temp_marks.contains(node) {
            // TODO: Fix!
            panic!("You are cycling."); // ERR
        }

        temp_marks.insert(node.clone());
        if let Some(deps) = graph.get(node) {
            for dep in *deps {
                visit(dep, sorted_properties, visited, temp_marks, graph);
            }
        }
        temp_marks.remove(node);
        visited.insert(node.clone());
        sorted_properties.push(node.clone());
    }

    let mut sorted_properties = vec![];
    let mut visited = HashSet::new();
    let mut temp_marks = HashSet::new();

    for node in graph.keys() {
        if visited.contains(node) {
            continue;
        }

        visit(
            node,
            &mut sorted_properties,
            &mut visited,
            &mut temp_marks,
            graph,
        );
    }

    sorted_properties
}

fn canonicalize_annotations(
    annotations: &Vec<src::Annotation>,
    constraints: &mut Vec<Constraint>,
    other: &mut Vec<Expr>,
) -> Result<(), Vec<canonicalize::Annotation>> {
    let mut errors = vec![];
    for annotation in annotations {
        if let Err(error) = parse_annotation(annotation, constraints, other) {
            errors.push(error);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn parse_annotation(
    annotation: &src::Annotation,
    constraints: &mut Vec<Constraint>,
    other: &mut Vec<Expr>,
) -> Result<(), canonicalize::Annotation> {
    match &annotation.expr {
        src::Expr::List(region, expressions) => match expressions.as_slice() {
            [] => Err(canonicalize::Annotation::Empty(region.clone())),
            [src::Expr::Symbol(_, value), args @ ..] if value == "check" => {
                let mut parsed_constraints = parse_constraints(args)?;
                constraints.append(&mut parsed_constraints);
                Ok(())
            }
            //[src::Expr::Symbol(region, value), _args @ ..] => Err(

            //    canonicalize::Annotation::UnknownSymbol(region.clone(), value.clone()),
            //),
            _ => {
                other.push(canonicalize_expr(&annotation.expr));
                Ok(())
            }
        },
        _ => {
            other.push(canonicalize_expr(&annotation.expr));
            Ok(())
        }
    }
}

fn parse_constraints(args: &[src::Expr]) -> Result<Vec<Constraint>, canonicalize::Annotation> {
    let mut constraints = vec![];
    for expr in args {
        constraints.push(parse_constraint(expr)?);
    }
    Ok(constraints)
}

fn parse_constraint(expr: &src::Expr) -> Result<Constraint, canonicalize::Annotation> {
    let value = match expr {
        src::Expr::Boolean(_, value) => Constraint::Boolean(*value),
        src::Expr::Number(_, value) => Constraint::Number(*value),
        src::Expr::String(_, value) => Constraint::String(value.clone()),
        src::Expr::Symbol(region, value) => {
            if value.starts_with(".") {
                let (_, value) = value.split_at(1);
                Constraint::Access(value.to_owned())
            } else {
                return Err(canonicalize::Annotation::UnknownSymbol(
                    region.clone(),
                    value.clone(),
                ));
            }
        }
        src::Expr::Keyword(_, value) => Constraint::String(value.clone()),
        src::Expr::List(_, expressions) => match expressions.as_slice() {
            [src::Expr::Symbol(_, value), args @ ..] if value == "<" => Constraint::Lt(
                args.iter()
                    .map(parse_constraint)
                    .collect::<Result<Vec<Constraint>, canonicalize::Annotation>>()?,
            ),
            [src::Expr::Symbol(_, value), args @ ..] if value == "<=" => Constraint::Le(
                args.iter()
                    .map(parse_constraint)
                    .collect::<Result<Vec<Constraint>, canonicalize::Annotation>>()?,
            ),
            [src::Expr::Symbol(_, value), args @ ..] if value == "=" => Constraint::Eq(
                args.iter()
                    .map(parse_constraint)
                    .collect::<Result<Vec<Constraint>, canonicalize::Annotation>>()?,
            ),
            [src::Expr::Symbol(_, value), args @ ..] if value == ">=" => Constraint::Ge(
                args.iter()
                    .map(parse_constraint)
                    .collect::<Result<Vec<Constraint>, canonicalize::Annotation>>()?,
            ),
            [src::Expr::Symbol(_, value), args @ ..] if value == ">" => Constraint::Gt(
                args.iter()
                    .map(parse_constraint)
                    .collect::<Result<Vec<Constraint>, canonicalize::Annotation>>()?,
            ),
            [src::Expr::Symbol(_, value), args @ ..] if value == "or" => Constraint::Or(
                args.iter()
                    .map(parse_constraint)
                    .collect::<Result<Vec<Constraint>, canonicalize::Annotation>>()?,
            ),
            [src::Expr::Symbol(_, value), arg, _args @ ..] if value == "len" => {
                Constraint::Len(Box::new(parse_constraint(arg)?))
            }
            [src::Expr::Symbol(_, value), args @ ..] if value == "and" => Constraint::And(
                args.iter()
                    .map(parse_constraint)
                    .collect::<Result<Vec<Constraint>, canonicalize::Annotation>>()?,
            ),
            [src::Expr::Symbol(_, value), arg, _args @ ..] if value == "blank" => {
                let constraint = parse_constraint(arg)?;
                Constraint::Blank(Box::new(constraint))
            }
            [src::Expr::Symbol(_, value), arg, _args @ ..] if value == "not" => {
                let constraint = parse_constraint(arg)?;
                Constraint::Not(Box::new(constraint))
            }
            _ => unimplemented!(),
        },
        src::Expr::Map(_, values) => Constraint::Map(
            values
                .iter()
                .map(|(a, b)| {
                    parse_constraint(a)
                        .and_then(|key| parse_constraint(b).map(|value| (key, value)))
                })
                .collect::<Result<Vec<(Constraint, Constraint)>, canonicalize::Annotation>>()?,
        ),
    };

    Ok(value)
}

fn canonicalize_expr(expr: &src::Expr) -> can::Expr {
    match expr {
        src::Expr::Boolean(region, value) => can::Expr::Boolean(region.clone(), value.clone()),
        src::Expr::Number(region, value) => can::Expr::Number(region.clone(), value.clone()),
        src::Expr::Keyword(region, value) => can::Expr::Keyword(region.clone(), value.clone()),
        src::Expr::String(region, value) => can::Expr::String(region.clone(), value.clone()),
        src::Expr::Symbol(region, value) => can::Expr::Symbol(region.clone(), value.clone()),
        src::Expr::Map(region, value) => can::Expr::Map(
            region.clone(),
            value
                .iter()
                .map(|(key, value)| (canonicalize_expr(key), canonicalize_expr(value)))
                .collect(),
        ),
        src::Expr::List(region, value) => can::Expr::List(
            region.clone(),
            value.iter().map(canonicalize_expr).collect(),
        ),
    }
}

fn canonicalize_enum(data: &src::Enum) -> Result<can::Enum, Vec<canonicalize::Enum>> {
    let mut errors = vec![];
    let mut variants = vec![];

    for variant in &data.variants {
        match canonicalize_variant(variant) {
            Ok(variant) => variants.push(variant),
            Err(variant_errors) => {
                let mut variant_errors = variant_errors
                    .iter()
                    .map(|error| {
                        canonicalize::Enum::BadVariant(variant.name.clone(), error.clone())
                    })
                    .collect();
                errors.append(&mut variant_errors);
            }
        }
    }

    let mut annotations = vec![];
    let mut constraints = vec![];
    if let Err(annotation_errors) =
        canonicalize_annotations(&data.annotations, &mut constraints, &mut annotations)
    {
        let mut annotation_errors = annotation_errors
            .iter()
            .map(|error| canonicalize::Enum::BadAnnotation(error.clone()))
            .collect();

        errors.append(&mut annotation_errors);
    }

    if errors.is_empty() {
        Ok(can::Enum {
            annotations,
            constraints,
            comment: data.doc_comment.clone(),
            name: data.name.clone(),
            variants,
            type_variables: data.type_variables.clone(),
        })
    } else {
        Err(errors)
    }
}

fn canonicalize_variant(
    variant: &src::Variant,
) -> Result<can::Variant, Vec<canonicalize::Variant>> {
    let mut properties = vec![];
    let mut errors = vec![];
    for property in &variant.properties {
        let result = canonicalize_property(property, |error| {
            canonicalize::Variant::BadProperty(
                property.name.clone(),
                canonicalize::Property::BadAnnotation(error.clone()),
            )
        });

        match result {
            Ok(property) => properties.push(property),
            Err(mut prop_errors) => errors.append(&mut prop_errors),
        }
    }

    let mut annotations = vec![];
    let mut constraints = vec![];
    let record_annotations =
        canonicalize_annotations(&variant.annotations, &mut constraints, &mut annotations).map_err(
            |annotation_errors| {
                annotation_errors
                    .iter()
                    .map(|error| canonicalize::Variant::BadAnnotation(error.clone()))
                    .collect::<Vec<canonicalize::Variant>>()
            },
        );

    match record_annotations {
        Ok(_) => {
            if errors.is_empty() {
                Ok(can::Variant {
                    annotations,
                    constraints,
                    comment: variant.doc_comment.clone(),
                    name: variant.name.clone(),
                    properties,
                })
            } else {
                Err(errors)
            }
        }
        Err(mut annotation_errors) => {
            errors.append(&mut annotation_errors);
            Err(errors)
        }
    }
}

fn canonicalize_service(
    service: &src::Service,
) -> Result<can::Service, Vec<canonicalize::Service>> {
    let mut methods = HashMap::new();
    let mut errors = vec![];
    let mut annotations = vec![];
    let mut constraints = vec![];
    if let Err(annotation_errors) =
        canonicalize_annotations(&service.annotations, &mut constraints, &mut annotations)
    {
        let mut annotation_errors = annotation_errors
            .iter()
            .map(|error| canonicalize::Service::BadAnnotation(error.clone()))
            .collect();

        errors.append(&mut annotation_errors);
    }

    for method in &service.methods {
        match canonicalize_method(method) {
            Ok(method) => {
                methods.insert(method.name.value.clone(), method);
            }
            Err(method_errors) => {
                let mut method_errors = method_errors
                    .iter()
                    .map(|error| {
                        canonicalize::Service::BadMethod(method.name.clone(), error.clone())
                    })
                    .collect::<Vec<canonicalize::Service>>();
                errors.append(&mut method_errors);
            }
        }
    }

    if !constraints.is_empty() {
        errors.push(canonicalize::Service::BadAnnotation(
            canonicalize::Annotation::InvalidAnnotation(Region::line(1, 0, 0)),
        ));
    }

    if errors.is_empty() {
        Ok(can::Service {
            annotations,
            name: service.name.clone(),
            comment: service.doc_comment.clone(),
            methods,
        })
    } else {
        Err(errors)
    }
}

fn canonicalize_method(method: &src::Method) -> Result<can::Method, Vec<canonicalize::Method>> {
    let mut parameters = vec![];
    let mut errors = vec![];
    for parameter in &method.parameters {
        let mut annotations = vec![];
        let mut constraints = vec![];
        match canonicalize_annotations(&parameter.annotations, &mut constraints, &mut annotations) {
            Err(annotation_errors) => {
                let mut annotation_errors = annotation_errors
                    .iter()
                    .map(|error| {
                        canonicalize::Method::BadParameter(
                            parameter.name.clone(),
                            canonicalize::Parameter::BadAnnotation(error.clone()),
                        )
                    })
                    .collect::<Vec<canonicalize::Method>>();
                errors.append(&mut annotation_errors);
            }
            Ok(_) => parameters.push(can::Parameter {
                comment: None,
                annotations,
                constraints,
                name: parameter.name.clone(),
                type_: parse_type(&parameter.type_),
            }),
        };
    }

    let mut annotations = vec![];
    let mut constraints = vec![];
    let record_annotations =
        canonicalize_annotations(&method.annotations, &mut constraints, &mut annotations).map_err(
            |annotation_errors| {
                annotation_errors
                    .iter()
                    .map(|error| canonicalize::Method::BadAnnotation(error.clone()))
                    .collect::<Vec<canonicalize::Method>>()
            },
        );

    match record_annotations {
        Ok(_) => {
            if errors.is_empty() {
                Ok(can::Method {
                    annotations,
                    comment: method.doc_comment.clone(),
                    name: method.name.clone(),
                    return_type: method.return_type.clone().map(|type_| parse_type(&type_)),
                    parameters,
                })
            } else {
                Err(errors)
            }
        }
        Err(mut annotation_errors) => {
            errors.append(&mut annotation_errors);
            Err(errors)
        }
    }
}

fn parse_type(type_: &src::Type) -> can::Type {
    match type_.name.value.as_str() {
        "String" => can::Type::String,
        "Int32" => can::Type::Int32,
        "Int64" => can::Type::Int64,
        "Float32" => can::Type::Float32,
        "Float64" => can::Type::Float64,
        "Boolean" => can::Type::Boolean,
        "Map" => {
            let key_type = parse_type(&type_.variables[0]);
            let value_type = parse_type(&type_.variables[1]);
            can::Type::Map(key_type.into(), value_type.into())
        }
        "Set" => {
            let result = parse_type(&type_.variables[0]);
            can::Type::Set(result.into())
        }
        "List" => {
            let result = parse_type(&type_.variables[0]);
            can::Type::List(result.into())
        }
        "Option" => {
            let result = parse_type(&type_.variables[0]);
            can::Type::Option(result.into())
        }
        "Result" => {
            let error = parse_type(&type_.variables[0]);
            let value = parse_type(&type_.variables[1]);
            can::Type::Result(error.into(), value.into())
        }
        _ => can::Type::Ref(
            type_.name.value.clone(),
            type_.variables.iter().map(parse_type).collect(),
        ),
    }
}
