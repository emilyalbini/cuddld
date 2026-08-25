use crate::picohcl::ast::{Expression, Statement};
use crate::template::Template;
use plinky_error::{ErasedContext as _, ErasedError, bail};
use plinky_macros::{Display, Error};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone)]
pub struct HclDeserializer {
    statements: HashMap<(String, Option<String>), Statement>,
}

impl HclDeserializer {
    pub(crate) fn from_statements(statements: Vec<Statement>) -> Self {
        Self {
            statements: statements
                .into_iter()
                .map(|statement| match &statement {
                    Statement::Assignment(assign) => ((assign.key.clone(), None), statement),
                    Statement::Block(block) => {
                        ((block.kind.clone(), block.name.clone()), statement)
                    }
                })
                .collect(),
        }
    }

    pub fn field<T: FromHclStatement>(&mut self, field_name: &str) -> Result<T, ErasedError> {
        let statement = self.statements.remove(&(field_name.into(), None));
        Ok(T::from_statement(statement)
            .with_context(|| format!("failed to parse field {field_name}"))?)
    }

    pub fn opt_field<T: FromHclStatement>(
        &mut self,
        field_name: &str,
    ) -> Result<Option<T>, ErasedError> {
        let statement = self.statements.remove(&(field_name.into(), None));
        match T::from_statement(statement) {
            Ok(value) => Ok(Some(value)),
            Err(FromHclError::MissingField) => Ok(None),
            Err(err) => Err(err).with_context(|| format!("failed to parse field {field_name}")),
        }
    }

    pub fn remaining(self) -> impl Iterator<Item = Statement> {
        self.statements.into_values()
    }

    pub fn ensure_exhaustive(&self) -> Result<(), ErasedError> {
        if !self.statements.is_empty() {
            let mut extra = Vec::new();
            for (name, optional) in self.statements.keys() {
                let mut name = name.clone();
                if let Some(optional) = optional {
                    name.push('.');
                    name.push_str(&optional);
                }
                extra.push(name);
            }
            bail!("unrecognized keys: {}", extra.join(", "));
        }
        Ok(())
    }
}

pub trait FromHclStatement: Sized {
    fn from_statement(statement: Option<Statement>) -> Result<Self, FromHclError>;
}

impl<T: FromHclExpression> FromHclStatement for T {
    fn from_statement(statement: Option<Statement>) -> Result<Self, FromHclError> {
        match statement {
            Some(Statement::Assignment(expr)) => T::from_expression(expr.value),
            Some(Statement::Block(_)) => Err(FromHclError::InvalidType),
            None => Err(FromHclError::MissingField),
        }
    }
}

impl<T: FromHclExpression> FromHclStatement for BTreeMap<String, T> {
    fn from_statement(statement: Option<Statement>) -> Result<Self, FromHclError> {
        match statement {
            Some(Statement::Assignment(_)) => Err(FromHclError::InvalidType),
            Some(Statement::Block(block)) => {
                if block.name.is_some() {
                    return Err(FromHclError::InvalidType);
                }
                let mut result = BTreeMap::new();
                for statement in block.contents {
                    match statement {
                        Statement::Assignment(a) => {
                            result.insert(a.key, T::from_expression(a.value)?);
                        }
                        _ => return Err(FromHclError::InvalidType),
                    }
                }
                Ok(result)
            }
            None => Err(FromHclError::MissingField),
        }
    }
}

pub trait FromHclExpression: Sized {
    fn from_expression(expr: Expression) -> Result<Self, FromHclError>;
}

impl FromHclExpression for Template {
    fn from_expression(expr: Expression) -> Result<Self, FromHclError> {
        match expr {
            Expression::String(s) => Ok(Template::parse(&s).expect("invalid legacy template")),
            Expression::Variable(v) => {
                Ok(Template::parse(&format!("${{{}}}", v.0)).expect("invalid legacy template"))
            }
            _ => Err(FromHclError::InvalidType),
        }
    }
}

impl FromHclExpression for String {
    fn from_expression(expr: Expression) -> Result<Self, FromHclError> {
        match expr {
            Expression::String(s) => Ok(s),
            _ => Err(FromHclError::InvalidType),
        }
    }
}

impl<T: FromHclString> FromHclExpression for T {
    fn from_expression(expr: Expression) -> Result<Self, FromHclError> {
        match expr {
            Expression::String(s) => T::from_string(s).map_err(FromHclError::Parse),
            _ => Err(FromHclError::InvalidType),
        }
    }
}

impl FromHclExpression for bool {
    fn from_expression(expr: Expression) -> Result<Self, FromHclError> {
        match expr {
            Expression::Bool(b) => Ok(b),
            _ => Err(FromHclError::InvalidType),
        }
    }
}

impl<T: FromHclExpression> FromHclExpression for Vec<T> {
    fn from_expression(expr: Expression) -> Result<Self, FromHclError> {
        match expr {
            Expression::List(expressions) => {
                let mut result = Vec::new();
                for expr in expressions {
                    result.push(T::from_expression(expr)?);
                }
                Ok(result)
            }
            _ => Err(FromHclError::InvalidType),
        }
    }
}

pub trait FromHclString: Sized {
    fn from_string(input: String) -> Result<Self, ErasedError>;
}

#[derive(Debug, Display, Error)]
pub enum FromHclError {
    #[display("invalid type for the expression")]
    InvalidType,
    #[display("required field is missing")]
    MissingField,
    #[display("failed to parse field: {f0}")]
    Parse(ErasedError),
}
