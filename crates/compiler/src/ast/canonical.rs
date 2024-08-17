use std::collections::HashMap;

use crate::ast::source::Name;
use crate::reporting::Region;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use super::constraints::Constraint;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Module {
    pub records: HashMap<String, Record>,
    pub enums: HashMap<String, Enum>,
    pub services: HashMap<String, Service>,
}

impl Module {
    /// Try to find the [method_name] in the methods of the given [service_name].
    ///
    /// ## Example
    ///
    /// Given the following .wrpc service definition.
    ///
    /// ```ignore
    /// service RandomService {
    ///     random(seed: Int32): Int32
    /// }
    /// ```
    ///
    /// We should be able to find a method for it using this method.
    ///
    /// ```ignore
    /// let expected = Some(Method { .. })
    /// module.get_method("RandomService", "random") == expected
    /// ```
    pub fn get_method<S: Into<String>>(&self, service_name: S, method_name: S) -> Option<&Method> {
        self.services
            .get(service_name.into().as_str())
            .map(|service| service.methods.get(method_name.into().as_str()))
            .flatten()
    }

    pub fn get_sorted_services(&self) -> Vec<&Service> {
        self.services
            .iter()
            .map(|(_, value)| value)
            .sorted_by_key(|x| x.name.value.clone())
            .collect()
    }

    pub fn get_sorted_enums(&self) -> Vec<&Enum> {
        self.enums
            .iter()
            .map(|(_, value)| value)
            .sorted_by_key(|x| x.name.value.clone())
            .collect()
    }

    pub fn get_sorted_records(&self) -> Vec<&Record> {
        self.records
            .iter()
            .map(|(_, value)| value)
            .sorted_by_key(|x| x.name.value.clone())
            .collect()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Record {
    pub annotations: Vec<Annotation>,
    pub comment: Option<String>,
    pub name: Name,
    pub properties: Vec<Property>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Property {
    pub annotations: Vec<Annotation>,
    pub comment: Option<String>,
    pub name: Name,
    pub type_: Type,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Enum {
    pub annotations: Vec<Annotation>,
    pub comment: Option<String>,
    pub name: Name,
    pub variants: Vec<Variant>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Variant {
    pub annotations: Vec<Annotation>,
    pub comment: Option<String>,
    pub name: Name,
    pub properties: Vec<Property>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Service {
    pub annotations: Vec<Annotation>,
    pub comment: Option<String>,
    pub name: Name,
    pub methods: HashMap<String, Method>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Method {
    pub annotations: Vec<Annotation>,
    pub name: Name,
    pub comment: Option<String>,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<Type>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Parameter {
    pub comment: Option<String>,
    pub annotations: Vec<Annotation>,
    pub name: Name,
    pub type_: Type,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Type {
    String,
    Boolean,
    Int32,
    Int64,
    Float32,
    Float64,
    Map(Box<Type>, Box<Type>),
    Result(Box<Type>, Box<Type>),
    List(Box<Type>),
    Set(Box<Type>),
    Option(Box<Type>),
    Ref(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Annotation {
    Check(Vec<Constraint>),
    Custom(Expr),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Expr {
    Boolean(Region, bool),
    Number(Region, f64),
    Keyword(Region, String),
    String(Region, String),
    Symbol(Region, String),
    Map(Region, Vec<(Expr, Expr)>),
    List(Region, Vec<Expr>),
}
