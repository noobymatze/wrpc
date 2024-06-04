use crate::ast::{Data, Decl, Meta, Method, Module, Name, Parameter, Property, Service, Type};
use crate::error::syntax;
use crate::parse::lexer::LexResult;
use crate::parse::token::Token;
use crate::reporting::{Position, Region};
use std::path::PathBuf;
use std::vec;

pub mod lexer;
pub mod token;

pub fn parse(_filename: Option<PathBuf>, source: &str) -> Result<Module, Vec<syntax::Error>> {
    let tokenizer = lexer::lexer(source);
    let mut parser = Parser::new(tokenizer);
    parser.parse_module()
}

#[derive(Debug)]
struct Parser<T: Iterator<Item = LexResult>> {
    input: T,
    token1: Option<Result<(Region, Token), syntax::Token>>,
    errors: Vec<syntax::Error>,
    last_position: Position,
}

impl<T> Parser<T>
where
    T: Iterator<Item = LexResult>,
{
    fn new(input: T) -> Self {
        Parser {
            input,
            token1: None,
            errors: vec![],
            last_position: Position { line: 0, col: 0 },
        }
    }

    fn parse_module(&mut self) -> Result<Module, Vec<syntax::Error>> {
        let mut declarations: Vec<Decl> = vec![];

        loop {
            let next = self.parse_decl();
            match next {
                Ok(None) => break,
                Ok(Some(decl)) => declarations.push(decl),
                Err(error) => {
                    self.errors
                        .push(syntax::Error::ParseError(syntax::Module::Decl(error)));
                    self.recover();
                }
            }
        }

        let errors = &self.errors;
        if !errors.is_empty() {
            Err(errors.clone()) // There is probably a better way, then to clone the whole thing
        } else {
            Ok(Module {
                version: "1".into(),
                declarations,
                doc_comment: None,
            })
        }
    }

    fn parse_decl(&mut self) -> Result<Option<Decl>, syntax::Decl> {
        let comment = self.parse_comment().map_err(syntax::Decl::BadComment)?;
        //let annotations = self.parse_annotations()?;
        match self.advance() {
            None => Ok(None),
            Some(Ok((_, Token::Data))) => self
                .parse_data(comment, vec![])
                .map(|x| Some(Decl::Data(x))),
            Some(Ok((_, Token::Service))) => self
                .parse_service(comment, vec![])
                .map(|x| Some(Decl::Service(x)))
                .map_err(syntax::Decl::BadService),
            Some(Ok((region, _))) => Err(syntax::Decl::Start(region.start.line, region.start.col)),
            Some(Err(_)) => Err(syntax::Decl::Start(0, 0)),
            //Some((_, Token::Enum)) => self
            //    .parse_enum(comment, annotations)
            //    .map(|x| Some(Decl::Enum(x))),
            //Some((_, Token::Service)) => self
            //    .parse_service(comment, annotations)
            //    .map(|x| Some(Decl::Service(x))),
            //Some((region, token)) => Err(UnexpectedToken {
            //    region: Some(region),
            //    found: token,
            //    expected: Token::Data,
            //}),
        }
    }

    fn recover(&mut self) {
        while !matches!(
            self.peek(),
            Some(Token::Data) | Some(Token::Service) | Some(Token::Enum) | Some(Token::Eof) | None
        ) {
            self.advance();
        }
    }

    fn parse_service(
        &mut self,
        comment: Option<String>,
        annotations: Vec<Meta>,
    ) -> Result<Service, syntax::Service> {
        let name = self.expect_name().map_err(syntax::Service::BadName)?;
        self.expect_token(Token::LBrace)
            .map_err(|pos| syntax::Service::Start(pos.line, pos.col))?;

        let methods = self.parse_methods().map_err(syntax::Service::BadMethod)?;
        self.expect_token(Token::RBrace)
            .map_err(|pos| syntax::Service::MissingEnd(pos.line, pos.col))?;

        Ok(Service {
            annotations,
            doc_comment: comment,
            name,
            methods,
        })
    }

    fn parse_methods(&mut self) -> Result<Vec<Method>, syntax::Method> {
        let mut methods = vec![];
        while !matches!(self.peek(), Some(Token::RBrace) | Some(Token::Eof) | None) {
            println!("{:?}", self.peek());
            self.parse_method(&mut methods)?;
        }

        Ok(methods)
    }

    fn parse_method(&mut self, methods: &mut Vec<Method>) -> Result<(), syntax::Method> {
        let comment = self.parse_comment().map_err(syntax::Method::BadComment)?;
        // parse annotations
        self.expect_token(Token::Def)
            .map_err(|pos| syntax::Method::MissingDef(pos.line, pos.col))?;
        let name = self.expect_name().map_err(syntax::Method::BadName)?;
        self.expect_token(Token::LParen)
            .map_err(|pos| syntax::Method::MissingParamStart(name.clone(), pos.line, pos.col))?;
        let properties = self.parse_properties().map_err(syntax::Method::BadParam)?;

        self.expect_token(Token::RParen)
            .map_err(|pos| syntax::Method::MissingParamEnd(name.clone(), pos.line, pos.col))?;

        let return_type = if self.matches(Token::Colon) {
            Some(self.parse_type().map_err(syntax::Method::BadReturnType)?)
        } else {
            None
        };

        let method = Method {
            name,
            parameters: properties
                .iter()
                .map(|prop| Parameter {
                    name: prop.name.clone(),
                    annotations: prop.annotations.clone(),
                    type_: prop.type_.clone(),
                })
                .collect(),
            annotations: vec![],
            return_type,
            doc_comment: comment,
        };

        methods.push(method);

        Ok(())
    }

    fn parse_data(
        &mut self,
        comment: Option<String>,
        annotations: Vec<Meta>,
    ) -> Result<Data, syntax::Decl> {
        let name = self.expect_name().map_err(syntax::Decl::DataName)?;
        let mut properties = vec![];
        if self.matches(Token::LBrace) {
            let mut parsed_properties =
                self.parse_properties().map_err(syntax::Decl::BadProperty)?;
            properties.append(&mut parsed_properties);
            self.expect_token(Token::RBrace)
                .map_err(|pos| syntax::Decl::End(pos.line, pos.col))?;
        }

        Ok(Data {
            annotations,
            doc_comment: comment,
            name,
            properties,
        })
    }

    fn parse_properties(&mut self) -> Result<Vec<Property>, syntax::Property> {
        let mut properties = vec![];
        while !matches!(
            self.peek(),
            Some(Token::RBrace) | Some(Token::Eof) | None | Some(Token::RParen)
        ) {
            self.parse_property(&mut properties)?;
            self.matches(Token::Comma);
        }

        Ok(properties)
    }

    fn parse_property(&mut self, properties: &mut Vec<Property>) -> Result<(), syntax::Property> {
        let comment = self.parse_comment().map_err(syntax::Property::BadComment)?;
        // parse annotations
        if matches!(self.peek(), Some(Token::Identifier(_))) {
            let name = self.expect_name().map_err(syntax::Property::BadName)?;
            self.expect_token(Token::Colon)
                .map_err(|pos| syntax::Property::MissingColon(name.clone(), pos.line, pos.col))?;

            let type_ = self
                .parse_type()
                .map_err(|error| syntax::Property::BadType(name.clone(), error))?;

            let property = Property {
                name,
                type_,
                annotations: vec![],
                doc_comment: comment,
            };
            properties.push(property);
        }

        Ok(())
    }

    fn parse_comment(&mut self) -> Result<Option<String>, syntax::Token> {
        let mut lines = vec![];
        while matches!(self.peek(), Some(Token::Comment(_))) {
            if let Some(Ok((_, Token::Comment(comment)))) = self.advance() {
                lines.push(comment);
            }
        }

        let comment = lines
            .iter()
            .map(|line| line.trim())
            .filter(|line| line.len() > 0)
            .collect::<Vec<_>>()
            .join("\n");

        if comment.is_empty() {
            return Ok(None);
        }

        Ok(Some(comment))
    }

    fn parse_type(&mut self) -> Result<Type, syntax::Type> {
        let name = self.expect_name().map_err(syntax::Type::BadName)?;
        Ok(Type {
            name,
            variables: vec![],
        })
    }

    // HELPERS

    //fn check(&mut self, expected: Token) -> bool {
    //    matches!(self.peek(), Some(token) if token == &expected)
    //}

    fn matches(&mut self, expected: Token) -> bool {
        if matches!(self.peek(), Some(token) if token == &expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect_token(&mut self, token: Token) -> Result<(), Position> {
        match self.advance() {
            Some(Ok((_, tok))) if tok == token => Ok(()),
            Some(Ok((region, _))) => Err(region.end),
            Some(Err(bad_token)) => Err(bad_token.position()),
            None => Err(self.last_position.clone()),
        }
    }

    fn expect_name(&mut self) -> Result<Name, syntax::Name> {
        match self.advance() {
            Some(Ok((region, Token::Identifier(name)))) => Ok(Name {
                region,
                value: name,
            }),
            Some(Ok((region, _))) => Err(syntax::Name::ExpectedName(
                region.start.line,
                region.start.col,
            )),
            Some(Err(bad_token)) => Err(syntax::Name::BadToken(bad_token)),
            None => Err(syntax::Name::BadToken(syntax::Token::Eof(
                self.last_position.line,
                self.last_position.col,
            ))),
        }
    }

    fn peek(&mut self) -> Option<&Token> {
        self.next_token();
        self.token1
            .as_ref()
            .and_then(|x| x.as_ref().ok().map(|(_, token)| token))
    }

    fn advance(&mut self) -> Option<Result<(Region, Token), syntax::Token>> {
        self.next_token();
        match self.token1.take() {
            None => None,
            Some(Ok((region, Token::Eof))) => {
                self.last_position = region.end.clone();
                None
            }
            Some(value) => Some(value),
        }
    }

    fn next_token(&mut self) {
        if self.token1.is_none() {
            self.token1 = self.input.next();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn test_data_decl_without_braces_is_ok() {
        let result = parse(None, "data Test");
        assert!(result.is_ok())
    }

    #[test]
    fn test_data_decl_with_missing_ending_brace_errors() {
        let result = parse(None, "data Test {");
        assert!(result.is_err())
    }

    #[test]
    fn test_data_decl_with_ending_brace_is_ok() {
        let result = parse(None, "data Test {}");
        assert!(result.is_ok())
    }
}
